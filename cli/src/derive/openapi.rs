//! OpenAPI 3.0 and 3.1 to a provider shape (ADR 0088, spike 7.3 §3's table).
//!
//! One entry per operation and documented response status, each selected by the operation's method,
//! its path template and the status, so a derived shape keeps an operation's outcomes apart as a
//! recorded one does (ADR 0086 decision 4). Every place the mapping makes the shape admit more than the
//! document says, or claim what the document did not, is a [`Loss`]: half of what this module answers.

use super::schema::{Deriver, escape, strip_null};
use super::{Loss, Options, is_json};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

/// The methods an OpenAPI path item may hold an operation under.
const METHODS: &[&str] = &[
  "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

/// The interactions of a provider shape, from an OpenAPI document whose version has been checked.
pub fn interactions(document: &Value, options: &Options, losses: &mut Vec<Loss>) -> Vec<Value> {
  let mut deriver = Deriver::new(document, options, losses);
  let mut interactions = Vec::new();
  let Some(paths) = document.get("paths").and_then(Value::as_object) else {
    return interactions;
  };
  let document_servers = document.get("servers");
  for (path, item) in paths {
    let item_pointer = format!("/paths/{}", escape(path));
    let (item, item_pointer) = deriver.resolve_at(item, &item_pointer);
    let path_servers = item.get("servers").or(document_servers);
    for method in METHODS {
      let Some(operation) = item.get(*method) else {
        continue;
      };
      let operation_pointer = format!("{item_pointer}/{method}");
      let servers = operation.get("servers").or(path_servers);
      let base = deriver.base_path(servers, &operation_pointer, method, path);
      let template = format!("{base}{path}");
      interactions.extend(deriver.operation(method, path, &template, operation, &operation_pointer));
    }
  }
  interactions
}

impl Deriver<'_> {
  /// The prefix every server that applies to an operation puts before its paths: their common path,
  /// or `--base-path`. Servers that disagree give none, which is reported.
  fn base_path(&mut self, servers: Option<&Value>, pointer: &str, method: &str, path: &str) -> String {
    if let Some(base) = &self.options.base_path {
      return normalise_base(base);
    }
    let Some(servers) = servers
      .and_then(Value::as_array)
      .filter(|servers| !servers.is_empty())
    else {
      return String::new();
    };
    let bases: BTreeSet<String> = servers.iter().map(server_path).collect();
    if bases.len() == 1 {
      return bases.into_iter().next().unwrap_or_default();
    }
    self.entry = format!("{} {path}", method.to_uppercase());
    self.lose(
      "base-path-ambiguous",
      pointer,
      format!(
        "the servers that apply put different paths before it ({}), so none is used; name the one \
         consumers call with --base-path",
        bases
          .iter()
          .map(|base| if base.is_empty() {
            "/".to_string()
          } else {
            base.clone()
          })
          .collect::<Vec<_>>()
          .join(", ")
      ),
    );
    self.entry.clear();
    String::new()
  }

  /// Every entry one operation contributes: one per documented status, a range less the statuses
  /// documented explicitly, and no `default`.
  fn operation(
    &mut self,
    method: &str,
    path: &str,
    template: &str,
    operation: &Value,
    pointer: &str,
  ) -> Vec<Value> {
    let method_upper = method.to_uppercase();
    let operation_name = format!("{method_upper} {template}");
    let Some(responses) = operation.get("responses").and_then(Value::as_object) else {
      return Vec::new();
    };
    let explicit: BTreeSet<u16> = responses.keys().filter_map(|key| key.parse().ok()).collect();
    let mut entries = Vec::new();
    for (key, response) in responses {
      let response_pointer = format!("{pointer}/responses/{}", escape(key));
      self.entry = format!("{operation_name} {key}");
      let status = match status_of(key, &explicit) {
        Status::Code(code) => json!({ "shape": "equality", "example": code }),
        Status::Codes(codes) if codes.is_empty() => continue,
        Status::Codes(codes) => {
          json!({ "codes": codes, "example": codes[0], "shape": "http:status-class" })
        }
        Status::Default => {
          self.lose(
            "default-response",
            &response_pointer,
            "a 'default' response is not imported: it stands for every status the operation does \
             not name, so a consumer interaction expecting one of those is not checked",
          );
          continue;
        }
        Status::Invalid => {
          self.lose(
            "response-unreadable",
            &response_pointer,
            format!("'{key}' is not a status, a range such as 4XX, or 'default'"),
          );
          continue;
        }
      };
      let (response, response_pointer) = self.resolve_at(response, &response_pointer);
      let mut parts = Map::new();
      parts.insert("status".to_string(), status.clone());
      let headers = self.headers(&response, &response_pointer);
      if !headers.is_empty() {
        parts.insert(
          "headers".to_string(),
          json!({ "members": Value::Object(headers), "shape": "object" }),
        );
      }
      if let Some(body) = self.body(&response, &response_pointer) {
        parts.insert("body".to_string(), body);
      }

      let mut source = Map::new();
      source.insert("openapi".to_string(), Value::from(format!("{method} {path}")));
      if let Some(id) = operation.get("operationId").and_then(Value::as_str) {
        source.insert("operation-id".to_string(), Value::from(id));
      }
      source.insert("response".to_string(), Value::from(key.as_str()));

      entries.push(json!({
        "description": operation_name,
        "selector": {
          "request": {
            "method": { "example": method_upper, "shape": "equality" },
            "path": { "example": example_path(template), "shape": "http:path", "template": template }
          },
          "response": { "status": status }
        },
        "source": Value::Object(source),
        "parts": { "response": Value::Object(parts) }
      }));
    }
    self.entry.clear();
    entries
  }

  /// A response's documented headers, as the transport presents one: its field lines (component-
  /// interfaces spec §5.9). And `content-type`, when the response has exactly one media type.
  fn headers(&mut self, response: &Value, pointer: &str) -> Map<String, Value> {
    let mut members = Map::new();
    if let Some(content) = response.get("content").and_then(Value::as_object)
      && content.len() == 1
      && let Some(media_type) = content.keys().next()
      && !media_type.contains('*')
    {
      members.insert(
        "content-type".to_string(),
        field_lines(
          json!({ "example": media_type, "shape": "http:media-type", "type": media_type }),
          true,
        ),
      );
    }
    let Some(headers) = response.get("headers").and_then(Value::as_object) else {
      return members;
    };
    for (name, header) in headers {
      let lower = name.to_ascii_lowercase();
      // OpenAPI ignores a `Content-Type` header definition: the media types say it.
      if lower == "content-type" {
        continue;
      }
      let header_pointer = format!("{pointer}/headers/{}", escape(name));
      let (header, header_pointer) = self.resolve_at(header, &header_pointer);
      let required = header.get("required").and_then(Value::as_bool).unwrap_or(false);
      let shape = match header.get("schema") {
        Some(schema) => self.header_text(schema, &format!("{header_pointer}/schema")),
        None => {
          self.lose(
            "untyped-schema",
            &header_pointer,
            "the header has no schema, so any text is admitted",
          );
          json!({ "example": "string", "shape": "string" })
        }
      };
      members.insert(lower, field_lines(shape, required));
    }
    members
  }

  /// A header's schema, read as the text a field line is: a header is never a JSON number or
  /// boolean, so an `integer` is the digits that write one, and an enum its values written out.
  fn header_text(&mut self, schema: &Value, pointer: &str) -> Value {
    let (schema, pointer) = match schema.get("$ref").and_then(Value::as_str) {
      Some(reference) => match self.lookup(reference) {
        Some(target) => (target.clone(), reference[1..].to_string()),
        None => {
          self.lose(
            "external-reference",
            pointer,
            format!("'{reference}' is not a schema in this document; any text is admitted"),
          );
          return json!({ "example": "string", "shape": "string" });
        }
      },
      None => (schema.clone(), pointer.to_string()),
    };
    let pointer = pointer.as_str();
    let (schema, _) = strip_null(&schema);
    let text = |value: &Value| match value {
      Value::String(text) => Value::from(text.as_str()),
      other => Value::from(other.to_string()),
    };
    if let Some(constant) = schema.get("const") {
      return json!({ "example": text(constant), "shape": "equality" });
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array) {
      let options: Vec<Value> = options
        .iter()
        .filter(|option| !option.is_null())
        .map(text)
        .collect();
      return match options.as_slice() {
        [] => json!({ "example": "string", "shape": "string" }),
        [only] => json!({ "example": only, "shape": "equality" }),
        _ => json!({ "example": options[0], "options": options, "shape": "any-of" }),
      };
    }
    self.bounds(&schema, pointer);
    let example = |made_up: &str| {
      schema
        .get("example")
        .filter(|example| !example.is_object() && !example.is_array())
        .map(text)
        .unwrap_or_else(|| Value::from(made_up))
    };
    match schema.get("type").and_then(Value::as_str) {
      Some("string") => self.string(&schema, pointer),
      Some("integer") => json!({ "example": example("1"), "pattern": "^-?[0-9]+$", "shape": "regex" }),
      Some("number") => json!({
        "example": example("1.5"),
        "pattern": "^-?[0-9]+(\\.[0-9]+)?([eE][+-]?[0-9]+)?$",
        "shape": "regex"
      }),
      Some("boolean") => json!({ "example": "true", "options": ["true", "false"], "shape": "any-of" }),
      Some(kind @ ("array" | "object")) => {
        self.lose(
          "header-serialised",
          pointer,
          format!(
            "the header is an {kind} written as text by its style, which is not derived; any text is admitted"
          ),
        );
        json!({ "example": "string", "shape": "string" })
      }
      _ => {
        self.lose(
          "untyped-schema",
          pointer,
          "the header's schema names no type a field line can be, so any text is admitted",
        );
        json!({ "example": "string", "shape": "string" })
      }
    }
  }

  /// The body of the response's JSON media type. Anything else claims no body, and says so.
  fn body(&mut self, response: &Value, pointer: &str) -> Option<Value> {
    let content = response.get("content").and_then(Value::as_object)?;
    let json_type = content.keys().find(|media_type| is_json(media_type));
    let Some(json_type) = json_type else {
      if !content.is_empty() {
        self.lose(
          "body-not-json",
          &format!("{pointer}/content"),
          format!(
            "the body is {}, and only a JSON body is derived; no body is claimed",
            content.keys().cloned().collect::<Vec<_>>().join(", ")
          ),
        );
      }
      return None;
    };
    let media_pointer = format!("{pointer}/content/{}", escape(json_type));
    match content[json_type].get("schema") {
      Some(schema) => Some(self.schema(schema, &format!("{media_pointer}/schema"), 0)),
      None => {
        self.lose(
          "untyped-schema",
          &media_pointer,
          "the body has no schema, so any value is admitted",
        );
        Some(json!({ "shape": "any" }))
      }
    }
  }
}

enum Status {
  Code(u16),
  Codes(Vec<u16>),
  Default,
  Invalid,
}

/// A response's key: a status, a range less the statuses documented explicitly, or `default`.
fn status_of(key: &str, explicit: &BTreeSet<u16>) -> Status {
  if key == "default" {
    return Status::Default;
  }
  if let Ok(code) = key.parse::<u16>()
    && (100..=599).contains(&code)
  {
    return Status::Code(code);
  }
  let bytes = key.as_bytes();
  if bytes.len() == 3
    && (b'1'..=b'5').contains(&bytes[0])
    && bytes[1].eq_ignore_ascii_case(&b'x')
    && bytes[2].eq_ignore_ascii_case(&b'x')
  {
    let from = u16::from(bytes[0] - b'0') * 100;
    return Status::Codes(
      (from..from + 100)
        .filter(|code| !explicit.contains(code))
        .collect(),
    );
  }
  Status::Invalid
}

/// A header's shape as the transport presents a header: one field line, held to `shape`.
fn field_lines(shape: Value, required: bool) -> Value {
  let lines = json!({ "items": shape, "max": 1, "min": 1, "shape": "each-like" });
  if required {
    lines
  } else {
    json!({ "of": lines, "shape": "optional" })
  }
}

/// The path a server URL puts before an operation's: its variables at their defaults, with no
/// trailing slash, and nothing for `/`.
fn server_path(server: &Value) -> String {
  let mut url = server
    .get("url")
    .and_then(Value::as_str)
    .unwrap_or_default()
    .to_string();
  if let Some(variables) = server.get("variables").and_then(Value::as_object) {
    for (name, variable) in variables {
      if let Some(default) = variable.get("default").and_then(Value::as_str) {
        url = url.replace(&format!("{{{name}}}"), default);
      }
    }
  }
  let path = match url.find("://") {
    Some(at) => {
      let rest = &url[at + 3..];
      rest.find('/').map(|slash| &rest[slash..]).unwrap_or("")
    }
    None => url.as_str(),
  };
  let path = path.split(['?', '#']).next().unwrap_or_default();
  normalise_base(path)
}

fn normalise_base(base: &str) -> String {
  let trimmed = base.trim().trim_end_matches('/');
  if trimmed.is_empty() {
    String::new()
  } else if trimmed.starts_with('/') {
    trimmed.to_string()
  } else {
    format!("/{trimmed}")
  }
}

/// A path the template admits, for the selector's example: each parameter written as its name.
fn example_path(template: &str) -> String {
  template.replace(['{', '}'], "")
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn a_range_leaves_out_the_statuses_documented_explicitly() {
    let explicit = BTreeSet::from([404]);
    let Status::Codes(codes) = status_of("4XX", &explicit) else {
      panic!("not a range");
    };
    assert_eq!(codes.len(), 99);
    assert!(!codes.contains(&404));
    assert!(matches!(status_of("2xx", &BTreeSet::new()), Status::Codes(_)));
    assert!(matches!(status_of("default", &explicit), Status::Default));
    assert!(matches!(status_of("6XX", &explicit), Status::Invalid));
    assert!(matches!(status_of("200", &explicit), Status::Code(200)));
  }

  #[test]
  fn a_server_url_gives_its_path_with_variables_at_their_defaults() {
    assert_eq!(
      server_path(&json!({ "url": "https://api.example.com/v1/" })),
      "/v1"
    );
    assert_eq!(server_path(&json!({ "url": "https://api.example.com" })), "");
    assert_eq!(server_path(&json!({ "url": "/api" })), "/api");
    assert_eq!(
      server_path(&json!({
        "url": "https://{host}/{base}",
        "variables": { "host": { "default": "x" }, "base": { "default": "v2" } }
      })),
      "/v2"
    );
  }

  #[test]
  fn json_media_types_are_json_and_suffixes_count() {
    assert!(is_json("application/json"));
    assert!(is_json("application/json; charset=utf-8"));
    assert!(is_json("application/problem+json"));
    assert!(!is_json("text/plain"));
    assert!(!is_json("application/xml"));
  }
}
