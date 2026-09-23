//! `csv` — a Janus content component for `text/csv`.
//!
//! Written against the published component interfaces (component-interfaces spec v1, the WIT world in
//! `wit/component.wit`) and nothing else. It implements the `content` interface: `decode`, `encode`,
//! `compile` (which contributes no fragment) and `detect` (which it declines).
//!
//! CSV is lossy, and this component says so instead of pretending (spec §6.4). Every field decodes as a
//! string: CSV has no numeric type, no boolean and no null, so `12.50`, `true` and an empty field are
//! the text that spells them. A document with a header row decodes to an array of objects, one member
//! per column in header order. A body declared `header=absent` (the RFC 4180 media-type parameter, or
//! the `header` decode option) decodes to an array of arrays.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Map, Value, json};

pub const NAME: &str = "csv";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const MEDIA_TYPE: &str = "text/csv";
const PROTOCOL_VERSION: u64 = 1;

/// One instance's state. The only thing an instance remembers is whether it has been greeted: the
/// spec forbids state that outlives a session (§3.4), and this component has none worth keeping.
#[derive(Default)]
pub struct Component {
  greeted: bool,
}

type OpResult = Result<Value, Value>;

impl Component {
  /// One request frame in, exactly one response frame out (spec §9.2). Never panics by design: a
  /// frame that cannot be read is answered with `malformed-frame` and an empty id (protocol §4.4).
  pub fn call(&mut self, request: &[u8]) -> Vec<u8> {
    let frame: Value = match serde_json::from_slice(request) {
      Ok(frame) => frame,
      Err(err) => {
        return response(
          "",
          Err(error(
            "malformed-frame",
            "protocol",
            &format!("frame is not JSON: {err}"),
            None,
          )),
        );
      }
    };
    let id = frame.get("id").and_then(Value::as_str).unwrap_or("").to_string();
    if frame.get("type").and_then(Value::as_str) != Some("request") {
      return response(
        &id,
        Err(error(
          "malformed-frame",
          "protocol",
          "frame 'type' must be 'request' on this pipe",
          None,
        )),
      );
    }
    let (Some(op), Some(body)) = (
      frame.get("op").and_then(Value::as_str),
      frame.get("body").filter(|b| b.is_object()),
    ) else {
      return response(
        &id,
        Err(error(
          "malformed-frame",
          "protocol",
          "a request frame needs a string 'op' and an object 'body'",
          None,
        )),
      );
    };
    response(&id, self.dispatch(op, body))
  }

  fn dispatch(&mut self, op: &str, body: &Value) -> OpResult {
    if op == "component/hello" {
      return self.hello(body);
    }
    if !self.greeted {
      return Err(error(
        "handshake-required",
        "protocol",
        &format!("'{op}' before component/hello"),
        None,
      ));
    }
    match op {
      "component/shutdown" => Ok(json!({})),
      "content/decode" => decode(body),
      "content/encode" => encode(body),
      // No fragment: the kernel compiles the slot generically and calls decode at execution time
      // (spec §6.3). Contributing one is task 8.4's business.
      "content/compile" => Ok(json!({})),
      _ => Err(error(
        "operation-unsupported",
        "protocol",
        &format!("the csv component does not implement '{op}'"),
        Some(json!({ "op": op })),
      )),
    }
  }

  fn hello(&mut self, body: &Value) -> OpResult {
    let offered = body
      .get("component-protocol-versions")
      .and_then(Value::as_array)
      .cloned()
      .unwrap_or_default();
    if !offered.iter().any(|v| v.as_u64() == Some(PROTOCOL_VERSION)) {
      return Err(error(
        "protocol-version-unsupported",
        "protocol",
        "the csv component speaks component protocol 1 only",
        Some(json!({ "supported": [PROTOCOL_VERSION] })),
      ));
    }
    self.greeted = true;
    Ok(json!({
      "component-protocol-version": PROTOCOL_VERSION,
      "component": { "name": NAME, "version": VERSION },
      "interfaces": ["content"],
      "contributes": {
        "content-types": [ {
          "media-type": MEDIA_TYPE,
          "degradations": [
            { "code": "string-only",
              "message": "CSV carries no types: every field decodes as a string, so integers, decimals and booleans are the text that spells them." },
            { "code": "no-null",
              "message": "An empty field decodes as the empty string; CSV cannot tell it from an absent or null value." } ] } ]
      }
    }))
  }
}

/// `content/decode`: octets to document (spec §6.1).
fn decode(body: &Value) -> OpResult {
  let content_type = content_type(body)?;
  let header = header_mode(&content_type, body.get("options"))?;
  let octets = slot_octets(body.get("value"))?;
  let text =
    std::str::from_utf8(&octets).map_err(|err| decode_failed(&format!("CSV is not UTF-8: {err}"), None))?;

  let mut reader = csv::ReaderBuilder::new()
    .has_headers(false)
    .flexible(false)
    .from_reader(text.as_bytes());
  let mut rows = Vec::new();
  for record in reader.records() {
    let record = record.map_err(|err| {
      let line = err.position().map(|p| p.line());
      decode_failed(&format!("{err}"), line)
    })?;
    rows.push(record.iter().map(str::to_string).collect::<Vec<_>>());
  }

  let mut degradations = Vec::new();
  let document = if header {
    let Some((names, records)) = rows.split_first() else {
      return Err(decode_failed(
        "header=present but the body has no header row",
        None,
      ));
    };
    for (i, name) in names.iter().enumerate() {
      if names[..i].contains(name) {
        return Err(decode_failed(
          &format!("column '{name}' appears twice in the header row"),
          Some(1),
        ));
      }
    }
    for (i, name) in names.iter().enumerate() {
      let column: Vec<&str> = records.iter().map(|r| r[i].as_str()).collect();
      degradations.extend(column_degradations(
        &column,
        &format!("$[*]{}", member_path(name)),
        name,
      ));
    }
    Value::Array(
      records
        .iter()
        .map(|r| {
          Value::Object(
            names
              .iter()
              .cloned()
              .zip(r.iter().cloned().map(Value::String))
              .collect::<Map<_, _>>(),
          )
        })
        .collect(),
    )
  } else {
    let width = rows.first().map_or(0, Vec::len);
    for i in 0..width {
      let column: Vec<&str> = rows.iter().map(|r| r[i].as_str()).collect();
      degradations.extend(column_degradations(
        &column,
        &format!("$[*][{i}]"),
        &format!("{i}"),
      ));
    }
    Value::Array(
      rows
        .into_iter()
        .map(|r| Value::Array(r.into_iter().map(Value::String).collect()))
        .collect(),
    )
  };

  let mut result = json!({ "document": document });
  if !degradations.is_empty() {
    result["degradations"] = Value::Array(degradations);
  }
  Ok(result)
}

/// Where a loss actually bit, per column (spec §6.4): the static declaration says CSV is lossy, these
/// say which columns held something a typed format would have kept.
fn column_degradations(column: &[&str], path: &str, name: &str) -> Vec<Value> {
  let mut out = Vec::new();
  let non_empty: Vec<&&str> = column.iter().filter(|v| !v.is_empty()).collect();
  if !non_empty.is_empty() && non_empty.iter().all(|v| v.parse::<f64>().is_ok()) {
    out.push(json!({ "code": "numeric-lexical", "path": path,
      "message": format!("column '{name}' decoded as strings; CSV has no numeric type") }));
  } else if !non_empty.is_empty() && non_empty.iter().all(|v| **v == "true" || **v == "false") {
    out.push(json!({ "code": "no-boolean", "path": path,
      "message": format!("column '{name}' decoded as strings; CSV has no boolean type") }));
  }
  if column.iter().any(|v| v.is_empty()) {
    out.push(json!({ "code": "no-null", "path": path,
      "message": format!("column '{name}' has empty fields, decoded as the empty string") }));
  }
  out
}

/// `content/encode`: document to octets (spec §6.2). `decode(encode(d)) == d` for every document
/// `decode` can produce — arrays of objects, or arrays of arrays, of strings. Anything wider is
/// written as the text that spells it (a number as its JSON text, `null` as an empty field), which
/// `decode` then reads back as a string: that is the declared degradation, not a round-trip failure.
fn encode(body: &Value) -> OpResult {
  let content_type = content_type(body)?;
  let header = header_mode(&content_type, body.get("options"))?;
  let document = match body.get("encoded").and_then(Value::as_str).unwrap_or("json") {
    "json" => body.get("document").cloned().unwrap_or(Value::Null),
    other => {
      return Err(encode_failed(
        &format!("a CSV document is structure, not '{other}'-encoded octets"),
        None,
      ));
    }
  };
  let Value::Array(rows) = &document else {
    return Err(encode_failed("a CSV document is an array of rows", Some("$")));
  };

  let mut writer = csv::WriterBuilder::new()
    .terminator(csv::Terminator::CRLF)
    .from_writer(Vec::new());
  if header {
    let mut columns: Vec<String> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
      let Value::Object(members) = row else {
        return Err(encode_failed(
          "with header=present every row is an object",
          Some(&format!("$[{i}]")),
        ));
      };
      for name in members.keys() {
        if !columns.contains(name) {
          columns.push(name.clone());
        }
      }
    }
    if !columns.is_empty() {
      write_record(&mut writer, columns.iter().cloned())?;
    }
    for (i, row) in rows.iter().enumerate() {
      let fields = columns
        .iter()
        .map(|name| {
          field(
            row.get(name).unwrap_or(&Value::Null),
            &format!("$[{i}]{}", member_path(name)),
          )
        })
        .collect::<Result<Vec<_>, _>>()?;
      write_record(&mut writer, fields)?;
    }
  } else {
    for (i, row) in rows.iter().enumerate() {
      let Value::Array(values) = row else {
        return Err(encode_failed(
          "with header=absent every row is an array",
          Some(&format!("$[{i}]")),
        ));
      };
      let fields = values
        .iter()
        .enumerate()
        .map(|(j, v)| field(v, &format!("$[{i}][{j}]")))
        .collect::<Result<Vec<_>, _>>()?;
      write_record(&mut writer, fields)?;
    }
  }
  let octets = writer
    .into_inner()
    .map_err(|err| encode_failed(&err.to_string(), None))?;
  Ok(
    json!({ "value": { "content": BASE64.encode(octets), "encoded": "base64", "content-type": content_type } }),
  )
}

fn field(value: &Value, path: &str) -> Result<String, Value> {
  match value {
    Value::String(s) => Ok(s.clone()),
    Value::Null => Ok(String::new()),
    Value::Bool(b) => Ok(b.to_string()),
    Value::Number(n) => Ok(n.to_string()),
    Value::Array(_) | Value::Object(_) => Err(encode_failed(
      "CSV fields are flat: a field cannot hold an array or an object",
      Some(path),
    )),
  }
}

fn write_record<I: IntoIterator<Item = String>>(
  writer: &mut csv::Writer<Vec<u8>>,
  fields: I,
) -> Result<(), Value> {
  writer
    .write_record(fields)
    .map_err(|err| encode_failed(&err.to_string(), None))
}

/// The request's content type, which must be ours. Parameters are allowed: `charset` must be UTF-8,
/// `header` is read by [`header_mode`], and anything else is ignored.
fn content_type(body: &Value) -> Result<String, Value> {
  let content_type = body
    .get("content-type")
    .and_then(Value::as_str)
    .unwrap_or_default();
  let mut parts = content_type.split(';').map(str::trim);
  let essence = parts.next().unwrap_or_default();
  if !essence.eq_ignore_ascii_case(MEDIA_TYPE) {
    return Err(error(
      "unsupported-content-type",
      "document",
      &format!("the csv component handles text/csv, not '{content_type}'"),
      Some(json!({ "content-type": content_type })),
    ));
  }
  for param in parts {
    if let Some((name, value)) = param.split_once('=')
      && name.trim().eq_ignore_ascii_case("charset")
      && !matches!(
        value.trim().trim_matches('"').to_ascii_lowercase().as_str(),
        "utf-8" | "utf8" | "us-ascii"
      )
    {
      return Err(error(
        "unsupported-content-type",
        "document",
        &format!(
          "the csv component reads UTF-8 only, not charset '{}'",
          value.trim()
        ),
        Some(json!({ "content-type": content_type })),
      ));
    }
  }
  Ok(content_type.to_string())
}

/// Whether the first row is a header. RFC 4180's `header` parameter on the media type, then the
/// `header` option, then `present` — the common case, and the one that decodes to named columns.
fn header_mode(content_type: &str, options: Option<&Value>) -> Result<bool, Value> {
  let from_type = content_type.split(';').skip(1).find_map(|p| {
    let (name, value) = p.split_once('=')?;
    name
      .trim()
      .eq_ignore_ascii_case("header")
      .then(|| value.trim().trim_matches('"').to_ascii_lowercase())
  });
  let from_options = options
    .and_then(|o| o.get("header"))
    .and_then(Value::as_str)
    .map(str::to_ascii_lowercase);
  match from_type.or(from_options).as_deref() {
    None | Some("present") => Ok(true),
    Some("absent") => Ok(false),
    Some(other) => Err(error(
      "invalid-config",
      "document",
      &format!("header must be 'present' or 'absent', not '{other}'"),
      Some(json!({ "header": other })),
    )),
  }
}

/// A slot value's octets (parts schema `SlotValue`): base64, text, or — for a JSON-encoded slot — a
/// string, which is the only JSON value that can hold CSV.
fn slot_octets(value: Option<&Value>) -> Result<Vec<u8>, Value> {
  let Some(value) = value.filter(|v| v.is_object()) else {
    return Err(decode_failed("'value' must be a slot value", None));
  };
  let content = value.get("content");
  match (
    value.get("encoded").and_then(Value::as_str).unwrap_or("json"),
    content,
  ) {
    ("base64", Some(Value::String(s))) => BASE64
      .decode(s)
      .map_err(|err| decode_failed(&format!("slot content is not base64: {err}"), None)),
    ("text" | "json", Some(Value::String(s))) => Ok(s.as_bytes().to_vec()),
    (encoded, _) => Err(decode_failed(
      &format!(
        "a CSV slot holds octets or text, not a '{encoded}'-encoded {}",
        kind(content)
      ),
      None,
    )),
  }
}

fn kind(value: Option<&Value>) -> &'static str {
  match value {
    None | Some(Value::Null) => "nothing",
    Some(Value::Bool(_)) => "boolean",
    Some(Value::Number(_)) => "number",
    Some(Value::String(_)) => "string",
    Some(Value::Array(_)) => "array",
    Some(Value::Object(_)) => "object",
  }
}

/// A member path segment in the plan grammar's path syntax: `.name` when it is a plain identifier,
/// `['name']` otherwise.
fn member_path(name: &str) -> String {
  if !name.is_empty()
    && name
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    && !name.starts_with(|c: char| c.is_ascii_digit())
  {
    format!(".{name}")
  } else {
    format!("['{}']", name.replace('\'', "\\'"))
  }
}

fn decode_failed(message: &str, line: Option<u64>) -> Value {
  let mut details = json!({ "content-type": MEDIA_TYPE });
  if let Some(line) = line {
    details["line"] = json!(line);
  }
  error("decode-failed", "document", message, Some(details))
}

fn encode_failed(message: &str, path: Option<&str>) -> Value {
  let mut details = json!({ "content-type": MEDIA_TYPE });
  if let Some(path) = path {
    details["path"] = json!(path);
  }
  error("encode-failed", "document", message, Some(details))
}

fn error(code: &str, category: &str, message: &str, details: Option<Value>) -> Value {
  let mut error = json!({ "code": code, "category": category, "message": message });
  if let Some(details) = details {
    error["details"] = details;
  }
  error
}

fn response(id: &str, result: OpResult) -> Vec<u8> {
  let frame = match result {
    Ok(ok) => json!({ "type": "response", "id": id, "ok": ok }),
    Err(error) => json!({ "type": "response", "id": id, "error": error }),
  };
  serde_json::to_vec(&frame).unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
mod wasm {
  use std::cell::RefCell;

  wit_bindgen::generate!({ world: "component", path: "wit" });

  thread_local! {
    static INSTANCE: RefCell<super::Component> = RefCell::new(super::Component::default());
  }

  struct Csv;

  impl exports::pact::janus_component::pipe::Guest for Csv {
    fn call(request: Vec<u8>) -> Vec<u8> {
      INSTANCE.with(|instance| instance.borrow_mut().call(&request))
    }
  }

  export!(Csv);
}
