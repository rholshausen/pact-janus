//! OpenAPI 3.0/3.1 -> provider shape (design 2.8 §2), provenance `derived`.
//!
//! Spike code. The mapping is the interesting part; the plumbing is not, and both are disposable.
//! Every place the mapping loses something, widens something, or cannot express something is
//! recorded as a `Gap` rather than silently absorbed — the count and kind of those gaps is half of
//! what this spike exists to measure.

use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// One place the mapping could not carry a schema across exactly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Gap {
  pub code: &'static str,
  pub path: String,
  pub detail: String,
}

pub struct Derived {
  pub provider_shape: Value,
  pub gaps: Vec<Gap>,
}

/// How an operation in the spec is named on the consumer's side. See FINDINGS §2: a derived shape
/// has no way to know this by itself, and needing the file at all is the finding.
#[derive(Debug, Clone)]
pub struct OperationName {
  pub description: String,
  pub states: Vec<String>,
}

pub fn derive(
  spec: &Value,
  provider: &str,
  names: &BTreeMap<String, OperationName>,
) -> Derived {
  let mut ctx = Ctx {
    spec,
    gaps: Vec::new(),
    depth: 0,
  };
  let mut interactions = Vec::new();

  let paths = spec.get("paths").and_then(Value::as_object).cloned().unwrap_or_default();
  for (path, item) in &paths {
    let Some(methods) = item.as_object() else {
      continue;
    };
    for (method, operation) in methods {
      if !matches!(
        method.as_str(),
        "get" | "put" | "post" | "delete" | "patch" | "head" | "options"
      ) {
        continue;
      }
      let operation_id = operation
        .get("operationId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
      let Some((status, schema)) = success_response(operation) else {
        ctx.gap(
          "no-json-response",
          &format!("{method} {path}"),
          "no 2xx response with an application/json schema",
        );
        continue;
      };

      let name = names.get(&operation_id);
      if name.is_none() {
        ctx.gap(
          "operation-unnamed",
          &format!("{method} {path}"),
          format!(
            "operationId '{operation_id}' has no consumer-side interaction description; the \
             derived interaction can never be matched (design 2.8 §2.2)"
          ),
        );
      }

      let body = ctx.shape_of(&schema, &format!("{method} {path} -> {status}"));
      let mut parts = Map::new();
      let mut response = Map::new();
      response.insert("status".to_string(), json!({ "shape": "equality", "example": status }));
      response.insert("body".to_string(), body);
      parts.insert("response".to_string(), Value::Object(response));

      let mut interaction = Map::new();
      interaction.insert(
        "description".to_string(),
        Value::from(
          name
            .map(|name| name.description.clone())
            .unwrap_or_else(|| operation_id.clone()),
        ),
      );
      if let Some(states) = name.map(|name| &name.states).filter(|s| !s.is_empty()) {
        interaction.insert(
          "states".to_string(),
          Value::Array(states.iter().map(|name| json!({ "name": name })).collect()),
        );
      }
      interaction.insert(
        "source".to_string(),
        json!({ "openapi": format!("{method} {path}"), "operation-id": operation_id }),
      );
      interaction.insert("parts".to_string(), Value::Object(parts));
      interactions.push(Value::Object(interaction));
    }
  }

  let mut gaps = ctx.gaps;
  gaps.sort();
  gaps.dedup();
  Derived {
    provider_shape: json!({
      "$format": "janus-provider-shape/1",
      "provider": { "name": provider },
      "provenance": "derived",
      "interactions": interactions
    }),
    gaps,
  }
}

fn success_response(operation: &Value) -> Option<(u64, Value)> {
  let responses = operation.get("responses")?.as_object()?;
  responses.iter().find_map(|(code, response)| {
    let status: u64 = code.parse().ok()?;
    if !(200..300).contains(&status) {
      return None;
    }
    let schema = response
      .get("content")?
      .get("application/json")?
      .get("schema")?
      .clone();
    Some((status, schema))
  })
}

struct Ctx<'a> {
  spec: &'a Value,
  gaps: Vec<Gap>,
  depth: usize,
}

impl Ctx<'_> {
  fn gap(&mut self, code: &'static str, path: &str, detail: impl Into<String>) {
    self.gaps.push(Gap {
      code,
      path: path.to_string(),
      detail: detail.into(),
    });
  }

  fn resolve<'b>(&self, schema: &'b Value) -> Value {
    let Some(reference) = schema.get("$ref").and_then(Value::as_str) else {
      return schema.clone();
    };
    let mut cursor = self.spec;
    for segment in reference.trim_start_matches("#/").split('/') {
      match cursor.get(segment) {
        Some(next) => cursor = next,
        None => return json!({}),
      }
    }
    cursor.clone()
  }

  /// The mapping table (FINDINGS §3).
  fn shape_of(&mut self, schema: &Value, path: &str) -> Value {
    if self.depth > 24 {
      self.gap("recursive-schema", path, "schema nests deeper than this spike follows");
      return json!({ "shape": "any" });
    }
    let schema = self.resolve(schema);

    // OpenAPI 3.1 spells nullability as a union with 'null'; 3.0 as `nullable: true`. Both mean
    // the same thing, and both are the RFC's worry when a generator applies them everywhere.
    let (schema, nullable) = strip_null(&schema);
    let inner = self.shape_of_non_null(&schema, path);
    if nullable {
      return json!({ "shape": "nullable", "of": inner });
    }
    inner
  }

  fn shape_of_non_null(&mut self, schema: &Value, path: &str) -> Value {
    if let Some(all_of) = schema.get("allOf").and_then(Value::as_array) {
      let merged = self.merge_all_of(all_of, path);
      return self.shape_of_non_null(&merged, path);
    }
    if let Some(one_of) = schema
      .get("oneOf")
      .or_else(|| schema.get("anyOf"))
      .and_then(Value::as_array)
    {
      return self.union_shape(schema, one_of, path);
    }

    let type_name = schema.get("type").and_then(Value::as_str);
    if let Some(enumeration) = schema.get("enum").and_then(Value::as_array) {
      let options: Vec<Value> = enumeration.iter().filter(|v| !v.is_null()).cloned().collect();
      if let Some(first) = options.first() {
        return json!({ "shape": "any-of", "options": options, "example": first });
      }
    }

    match type_name {
      Some("object") => self.object_shape(schema, path),
      Some("array") => self.array_shape(schema, path),
      Some("string") => self.string_shape(schema),
      Some("integer") => json!({ "shape": "integer", "example": 1 }),
      Some("number") => json!({ "shape": "number", "example": 1 }),
      Some("boolean") => json!({ "shape": "boolean" }),
      Some("null") => json!({ "shape": "null" }),
      Some(other) => {
        self.gap("unknown-type", path, format!("type '{other}' has no shape operator"));
        json!({ "shape": "any" })
      }
      None => {
        // A schema with no `type` constrains nothing. Generators emit these constantly.
        self.gap(
          "untyped-schema",
          path,
          "schema declares no type; every value is admitted",
        );
        json!({ "shape": "any" })
      }
    }
  }

  fn object_shape(&mut self, schema: &Value, path: &str) -> Value {
    let properties = schema.get("properties").and_then(Value::as_object);
    let Some(properties) = properties else {
      // `type: object` with no properties: a map of anything, or a model the generator gave up on.
      self.gap(
        "unconstrained-object",
        path,
        "type: object with no properties; admits every object",
      );
      return json!({ "shape": "object", "members": { } });
    };
    let required: Vec<&str> = schema
      .get("required")
      .and_then(Value::as_array)
      .map(|names| names.iter().filter_map(Value::as_str).collect())
      .unwrap_or_default();

    if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
      // ADR 0007 commitment 4 refuses closed objects, and shape spec §4.3 says no operator will be
      // added. The claim is simply not expressible, and dropping it is the only option.
      self.gap(
        "closed-object-dropped",
        path,
        "additionalProperties: false cannot be expressed (ADR 0007); extra members stay admitted",
      );
    }

    let mut members = Map::new();
    for (name, property) in properties {
      self.depth += 1;
      let member = self.shape_of(property, &format!("{path}.{name}"));
      self.depth -= 1;
      members.insert(
        name.clone(),
        if required.contains(&name.as_str()) {
          member
        } else {
          json!({ "shape": "optional", "of": member })
        },
      );
    }
    json!({ "shape": "object", "members": members })
  }

  fn array_shape(&mut self, schema: &Value, path: &str) -> Value {
    let items = schema.get("items").cloned().unwrap_or_else(|| {
      self.gap("untyped-items", path, "array with no items schema");
      json!({})
    });
    self.depth += 1;
    let item_shape = self.shape_of(&items, &format!("{path}[*]"));
    self.depth -= 1;
    // JSON Schema's default for minItems is 0 — the empty array is admitted unless the author
    // said otherwise, and authors almost never do.
    let min = schema.get("minItems").and_then(Value::as_u64).unwrap_or(0);
    let mut node = Map::from_iter([
      ("shape".to_string(), Value::from("each-like")),
      ("min".to_string(), Value::from(min)),
      ("items".to_string(), item_shape),
    ]);
    if let Some(max) = schema.get("maxItems").and_then(Value::as_u64) {
      node.insert("max".to_string(), Value::from(max));
    }
    Value::Object(node)
  }

  fn string_shape(&mut self, schema: &Value) -> Value {
    if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
      return json!({ "shape": "regex", "pattern": pattern, "example": "" });
    }
    match schema.get("format").and_then(Value::as_str) {
      // The shape language's `datetime`/`date` with no `format` admit any ISO-8601 string, which
      // is exactly what OpenAPI's formats mean (RFC 3339).
      Some("date-time") => json!({ "shape": "datetime", "example": "2026-07-30T10:00:00Z" }),
      Some("date") => json!({ "shape": "date", "example": "2026-07-30" }),
      _ => json!({ "shape": "string", "example": "x" }),
    }
  }

  /// `oneOf`/`anyOf`, with and without a discriminator — the sharpest mapping gap (FINDINGS §3).
  fn union_shape(&mut self, schema: &Value, alternatives: &[Value], path: &str) -> Value {
    let discriminator = schema
      .get("discriminator")
      .and_then(|d| d.get("propertyName"))
      .and_then(Value::as_str);
    let Some(discriminator) = discriminator else {
      self.gap(
        "undiscriminated-union",
        path,
        format!(
          "{} alternatives with no discriminator; the shape language has no undiscriminated \
           union (shape spec §4.4), so the whole subtree widens to `any`",
          alternatives.len()
        ),
      );
      return json!({ "shape": "any" });
    };

    let mut mapped = Map::new();
    for alternative in alternatives {
      let resolved = self.resolve(alternative);
      let name = alternative
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|reference| reference.rsplit('/').next())
        .map(str::to_string)
        .unwrap_or_else(|| format!("alt{}", mapped.len()));
      self.depth += 1;
      let shape = self.shape_of_non_null(&resolved, &format!("{path}@{name}"));
      self.depth -= 1;
      // The alternative must bind the discriminator to a literal (shape spec §5.4). A generator
      // that emitted the discriminator as a plain string gives nothing to bind.
      let bound = bind_discriminator(&shape, discriminator, &name);
      match bound {
        Some(bound) => {
          mapped.insert(name, bound);
        }
        None => {
          self.gap(
            "discriminator-unbound",
            &format!("{path}@{name}"),
            format!("alternative does not bind '{discriminator}' to a literal"),
          );
          return json!({ "shape": "any" });
        }
      }
    }
    if mapped.len() < 2 {
      return json!({ "shape": "any" });
    }
    json!({ "shape": "one-of", "discriminator": discriminator, "alternatives": mapped })
  }

  fn merge_all_of(&mut self, all_of: &[Value], path: &str) -> Value {
    let mut merged = Map::new();
    let mut properties = Map::new();
    let mut required = Vec::new();
    for part in all_of {
      let resolved = self.resolve(part);
      if let Some(part_properties) = resolved.get("properties").and_then(Value::as_object) {
        for (name, property) in part_properties {
          properties.insert(name.clone(), property.clone());
        }
      }
      if let Some(part_required) = resolved.get("required").and_then(Value::as_array) {
        required.extend(part_required.iter().cloned());
      }
      if let Some(type_name) = resolved.get("type") {
        merged.insert("type".to_string(), type_name.clone());
      }
      if resolved.get("oneOf").is_some() || resolved.get("anyOf").is_some() {
        self.gap(
          "all-of-with-union",
          path,
          "allOf branch carries its own union; this spike merges properties only",
        );
      }
    }
    merged.insert("properties".to_string(), Value::Object(properties));
    merged.insert("required".to_string(), Value::Array(required));
    merged.entry("type").or_insert_with(|| Value::from("object"));
    Value::Object(merged)
  }
}

/// `{ "nullable": true }` (3.0) and `{ "type": ["string", "null"] }` / an `anyOf` with a `null`
/// branch (3.1) all mean the same thing.
fn strip_null(schema: &Value) -> (Value, bool) {
  let mut schema = schema.clone();
  let mut nullable = schema.get("nullable") == Some(&Value::Bool(true));
  if let Some(object) = schema.as_object_mut() {
    object.remove("nullable");
  }

  if let Some(types) = schema.get("type").and_then(Value::as_array) {
    let remaining: Vec<Value> = types.iter().filter(|t| t.as_str() != Some("null")).cloned().collect();
    nullable |= remaining.len() != types.len();
    if let Some(object) = schema.as_object_mut() {
      match remaining.as_slice() {
        [only] => {
          object.insert("type".to_string(), only.clone());
        }
        _ => {
          object.remove("type");
        }
      }
    }
  }

  if let Some(branches) = schema.get("anyOf").and_then(Value::as_array)
    && branches.len() == 2
  {
    let null_branch = branches
      .iter()
      .position(|branch| branch.get("type").and_then(Value::as_str) == Some("null"));
    if let Some(index) = null_branch {
      let other = branches[1 - index].clone();
      return (other, true);
    }
  }

  (schema, nullable)
}

/// Rewrite an alternative's discriminator member as the `equality` shape spec §5.4 requires.
fn bind_discriminator(shape: &Value, discriminator: &str, literal: &str) -> Option<Value> {
  let mut shape = shape.clone();
  let members = shape.get_mut("members")?.as_object_mut()?;
  let current = members.get(discriminator)?;
  // Already a literal set: leave it. Otherwise bind it to the alternative's own name, which is
  // what an OpenAPI discriminator mapping means.
  let already_literal = matches!(
    current.get("shape").and_then(Value::as_str),
    Some("equality") | Some("any-of")
  );
  if !already_literal {
    members.insert(
      discriminator.to_string(),
      json!({ "shape": "equality", "example": literal }),
    );
  }
  Some(shape)
}
