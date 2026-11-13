//! JSON Schema to a shape: the mapping table both type documents' schemas go through (ADR 0088
//! decision 5, spike 7.3 §3). OpenAPI's Schema Object and AsyncAPI's are each JSON Schema with
//! keywords of their own, so one table reads both, and each front end adds only what its document
//! wraps a schema in (ADR 0091).

use super::{Loss, Options};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// How deep a schema is followed before it is written as `any`: deeper than any document a person
/// wrote, shallow enough that a pathological one cannot exhaust the stack.
const MAX_DEPTH: usize = 64;

/// What one derivation is reading and has lost so far, and the schema mapping over it.
pub(super) struct Deriver<'a> {
  pub(super) document: &'a Value,
  pub(super) options: &'a Options,
  losses: &'a mut Vec<Loss>,
  /// The entry being derived, as a report names it: `GET /orders/{id} 200`.
  pub(super) entry: String,
  /// The references being followed, innermost last, so a reference that recurs is caught.
  refs: Vec<String>,
}

impl<'a> Deriver<'a> {
  pub(super) fn new(document: &'a Value, options: &'a Options, losses: &'a mut Vec<Loss>) -> Self {
    Deriver {
      document,
      options,
      losses,
      entry: String::new(),
      refs: Vec::new(),
    }
  }
}

impl Deriver<'_> {
  pub(super) fn lose(&mut self, code: &'static str, pointer: &str, message: impl Into<String>) {
    self.losses.push(Loss {
      code,
      pointer: pointer.to_string(),
      entry: (!self.entry.is_empty()).then(|| self.entry.clone()),
      message: message.into(),
    });
  }

  /// A schema's shape: the mapping table.
  pub(super) fn schema(&mut self, schema: &Value, pointer: &str, depth: usize) -> Value {
    if depth > MAX_DEPTH {
      self.lose(
        "schema-too-deep",
        pointer,
        format!("the schema nests more than {MAX_DEPTH} levels deep; below that, any value is admitted"),
      );
      return json!({ "shape": "any" });
    }
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
      return self.reference(reference, pointer, depth);
    }
    let (schema, nullable) = strip_null(schema);
    let inner = self.non_null(&schema, pointer, depth);
    if nullable {
      return json!({ "of": inner, "shape": "nullable" });
    }
    inner
  }

  /// A `$ref`, followed within the document. One that recurs, or that names another document, is
  /// `any` where it is met.
  fn reference(&mut self, reference: &str, pointer: &str, depth: usize) -> Value {
    if self.refs.iter().any(|seen| seen == reference) {
      self.lose(
        "recursive-schema",
        pointer,
        format!("'{reference}' refers to itself through this schema; here any value is admitted"),
      );
      return json!({ "shape": "any" });
    }
    let Some(target) = self.lookup(reference) else {
      let code = if reference.starts_with('#') {
        "reference-unresolved"
      } else {
        "external-reference"
      };
      self.lose(
        code,
        pointer,
        format!("'{reference}' is not a schema in this document; any value is admitted"),
      );
      return json!({ "shape": "any" });
    };
    let target = target.clone();
    self.refs.push(reference.to_string());
    let shape = self.schema(&target, &reference[1..], depth + 1);
    self.refs.pop();
    shape
  }

  pub(super) fn lookup(&self, reference: &str) -> Option<&Value> {
    let pointer = reference.strip_prefix('#')?;
    self.document.pointer(pointer)
  }

  /// A path item, response or header that may be a `$ref` to one, with the pointer to what it is.
  pub(super) fn resolve_at(&self, value: &Value, pointer: &str) -> (Value, String) {
    match value.get("$ref").and_then(Value::as_str) {
      Some(reference) => match self.lookup(reference) {
        Some(target) => (target.clone(), reference[1..].to_string()),
        None => (Value::Null, pointer.to_string()),
      },
      None => (value.clone(), pointer.to_string()),
    }
  }

  fn non_null(&mut self, schema: &Value, pointer: &str, depth: usize) -> Value {
    if let Some(all_of) = schema.get("allOf").and_then(Value::as_array) {
      // `allOf` around one reference is how OpenAPI 3.0 annotates one (a description, `nullable`
      // beside a `$ref`): the schema is what it names, an enum or a string as much as an object.
      if let [only] = all_of.as_slice()
        && schema.get("properties").is_none()
      {
        return self.schema(only, &format!("{pointer}/allOf/0"), depth + 1);
      }
      let merged = self.merge_all_of(schema, all_of, pointer);
      return self.non_null(&merged, pointer, depth);
    }
    for keyword in ["oneOf", "anyOf"] {
      if let Some(alternatives) = schema.get(keyword).and_then(Value::as_array) {
        return self.union(schema, alternatives, &format!("{pointer}/{keyword}"), depth);
      }
    }
    if let Some(constant) = schema.get("const") {
      return json!({ "example": constant, "shape": "equality" });
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array) {
      let options: Vec<Value> = options
        .iter()
        .filter(|option| !option.is_null())
        .cloned()
        .collect();
      return match options.as_slice() {
        [] => json!({ "shape": "null" }),
        [only] => json!({ "example": only, "shape": "equality" }),
        _ => json!({ "example": options[0], "options": options, "shape": "any-of" }),
      };
    }
    self.bounds(schema, pointer);
    match schema.get("type").and_then(Value::as_str) {
      Some("object") => self.object(schema, pointer, depth),
      Some("array") => self.array(schema, pointer, depth),
      Some("string") => self.string(schema, pointer),
      Some("integer") => json!({ "example": example_or(schema, json!(1)), "shape": "integer" }),
      Some("number") => json!({ "example": example_or(schema, json!(1.5)), "shape": "number" }),
      Some("boolean") => json!({ "example": example_or(schema, json!(true)), "shape": "boolean" }),
      Some("null") => json!({ "shape": "null" }),
      Some(other) => {
        self.lose(
          "untyped-schema",
          pointer,
          format!("'{other}' is not a JSON Schema type; any value is admitted"),
        );
        json!({ "shape": "any" })
      }
      // An object's keywords with no `type` say it is one.
      None if schema.get("properties").is_some() => self.object(schema, pointer, depth),
      None => {
        let types = schema.get("type").and_then(Value::as_array);
        let message = match types {
          Some(types) => format!(
            "the value may be any of {}, and the shape language has no union of kinds; any value is \
             admitted",
            types
              .iter()
              .filter_map(Value::as_str)
              .collect::<Vec<_>>()
              .join(", ")
          ),
          None => "the schema names no type, so any value is admitted".to_string(),
        };
        self.lose("untyped-schema", pointer, message);
        json!({ "shape": "any" })
      }
    }
  }

  /// The constraints the shape language has no operator for: each one dropped is a widening.
  pub(super) fn bounds(&mut self, schema: &Value, pointer: &str) {
    let dropped: Vec<&str> = [
      "minimum",
      "maximum",
      "exclusiveMinimum",
      "exclusiveMaximum",
      "multipleOf",
      "minLength",
      "maxLength",
      "uniqueItems",
      "minProperties",
      "maxProperties",
    ]
    .into_iter()
    .filter(|keyword| schema.get(*keyword).is_some())
    .collect();
    if !dropped.is_empty() {
      self.lose(
        "bounds-dropped",
        pointer,
        format!(
          "{} cannot be expressed in a shape, so a value outside it is admitted",
          dropped.join(", ")
        ),
      );
    }
  }

  fn object(&mut self, schema: &Value, pointer: &str, depth: usize) -> Value {
    let properties = schema.get("properties").and_then(Value::as_object);
    let additional = schema.get("additionalProperties");
    if additional == Some(&Value::Bool(false)) {
      self.lose(
        "closed-object",
        pointer,
        "additionalProperties: false cannot be expressed (ADR 0007), so other members stay admitted",
      );
    }
    let schema_for_rest = additional.filter(|value| value.is_object());
    let Some(properties) = properties.filter(|properties| !properties.is_empty()) else {
      if let Some(values) = schema_for_rest {
        let values = self.schema(values, &format!("{pointer}/additionalProperties"), depth + 1);
        return json!({ "min": 0, "shape": "each-entry", "values": values });
      }
      self.lose(
        "unconstrained-object",
        pointer,
        "the object names no properties, so every object is admitted",
      );
      return json!({ "members": {}, "shape": "object" });
    };
    if schema_for_rest.is_some() {
      self.lose(
        "additional-properties-dropped",
        &format!("{pointer}/additionalProperties"),
        "the schema for members the object does not name is dropped beside its properties, so any \
         value is admitted for them",
      );
    }
    let required: BTreeSet<&str> = schema
      .get("required")
      .and_then(Value::as_array)
      .map(|names| names.iter().filter_map(Value::as_str).collect())
      .unwrap_or_default();
    let mut members = Map::new();
    for (name, property) in properties {
      // The provider never sends a write-only property (OpenAPI 3.0 §4.7.25, 3.1 by JSON Schema).
      if property.get("writeOnly") == Some(&Value::Bool(true)) {
        continue;
      }
      let member = self.schema(
        property,
        &format!("{pointer}/properties/{}", escape(name)),
        depth + 1,
      );
      members.insert(
        name.clone(),
        if required.contains(name.as_str()) {
          member
        } else {
          json!({ "of": member, "shape": "optional" })
        },
      );
    }
    json!({ "members": Value::Object(members), "shape": "object" })
  }

  fn array(&mut self, schema: &Value, pointer: &str, depth: usize) -> Value {
    let items = match schema.get("items") {
      Some(items) => self.schema(items, &format!("{pointer}/items"), depth + 1),
      None => {
        self.lose(
          "untyped-schema",
          pointer,
          "the array has no items schema, so any element is admitted",
        );
        json!({ "shape": "any" })
      }
    };
    let min = match schema.get("minItems").and_then(Value::as_u64) {
      Some(min) => min,
      None if self.options.assume_non_empty => {
        self.lose(
          "cardinality-assumed",
          pointer,
          "the array states no minItems, and --assume-non-empty claims it is never empty",
        );
        1
      }
      None => {
        self.lose(
          "cardinality-unstated",
          pointer,
          "the array states no minItems, so the empty array is admitted, as JSON Schema says; a \
           consumer that declared at least one element is told so",
        );
        0
      }
    };
    let mut node = Map::new();
    node.insert("items".to_string(), items);
    if let Some(max) = schema.get("maxItems").and_then(Value::as_u64) {
      node.insert("max".to_string(), Value::from(max));
    }
    node.insert("min".to_string(), Value::from(min));
    node.insert("shape".to_string(), Value::from("each-like"));
    Value::Object(node)
  }

  pub(super) fn string(&mut self, schema: &Value, pointer: &str) -> Value {
    let format = schema.get("format").and_then(Value::as_str);
    if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
      if regex::Regex::new(pattern).is_ok() {
        if format.is_some() {
          self.lose(
            "format-dropped",
            pointer,
            format!("'{}' is dropped beside the pattern", format.unwrap_or_default()),
          );
        }
        // An example the pattern does not admit would be a claim the shape contradicts, and
        // `regex` needs none: the document's own, when the pattern admits it.
        let mut node = json!({ "pattern": pattern, "shape": "regex" });
        if let Some(example) = schema
          .get("example")
          .filter(|example| example.as_str().is_some_and(|text| regex_admits(pattern, text)))
        {
          node["example"] = example.clone();
        }
        return node;
      }
      self.lose(
        "pattern-unreadable",
        pointer,
        format!(
          "'{pattern}' is not a pattern the shape language's regex dialect reads, so any string is admitted"
        ),
      );
    }
    // OpenAPI's formats are RFC 3339's, which the shape language's format-less forms read.
    match format {
      Some("date-time") => {
        json!({ "example": example_or(schema, json!("2026-07-30T10:00:00Z")), "shape": "datetime" })
      }
      Some("date") => json!({ "example": example_or(schema, json!("2026-07-30")), "shape": "date" }),
      Some(other) => {
        self.lose(
          "format-dropped",
          pointer,
          format!("format '{other}' has no shape, so any string is admitted"),
        );
        json!({ "example": example_or(schema, json!("string")), "shape": "string" })
      }
      None => json!({ "example": example_or(schema, json!("string")), "shape": "string" }),
    }
  }

  /// `oneOf` and `anyOf`: a `one-of` when a discriminator names each alternative's literal, and the
  /// whole subtree `any` otherwise (shape spec §4.4 has no undiscriminated union).
  fn union(&mut self, schema: &Value, alternatives: &[Value], pointer: &str, depth: usize) -> Value {
    // OpenAPI's discriminator is an object naming the property; AsyncAPI's is the property's name.
    let discriminator = schema.get("discriminator");
    let Some(property) = discriminator.and_then(|discriminator| {
      discriminator
        .get("propertyName")
        .and_then(Value::as_str)
        .or_else(|| discriminator.as_str())
    }) else {
      self.lose(
        "undiscriminated-union",
        pointer,
        format!(
          "{} alternatives with no discriminator, and the shape language has no undiscriminated \
           union, so everything below is admitted",
          alternatives.len()
        ),
      );
      return json!({ "shape": "any" });
    };
    // `mapping` names a literal for a reference; otherwise a reference's last segment is it.
    let mut literals: BTreeMap<String, String> = BTreeMap::new();
    if let Some(mapping) = discriminator
      .and_then(|discriminator| discriminator.get("mapping"))
      .and_then(Value::as_object)
    {
      for (literal, target) in mapping {
        if let Some(target) = target.as_str() {
          literals
            .entry(target.to_string())
            .or_insert_with(|| literal.clone());
        }
      }
    }
    let mut mapped = Map::new();
    for (index, alternative) in alternatives.iter().enumerate() {
      let alternative_pointer = format!("{pointer}/{index}");
      let reference = alternative.get("$ref").and_then(Value::as_str);
      let literal = reference.and_then(|reference| {
        literals
          .get(reference)
          .cloned()
          .or_else(|| reference.rsplit('/').next().map(str::to_string))
      });
      let shape = self.schema(alternative, &alternative_pointer, depth + 1);
      let bound = literal
        .as_deref()
        .and_then(|literal| bind_discriminator(&shape, property, literal))
        .or_else(|| already_bound(&shape, property).then(|| shape.clone()));
      let Some(bound) = bound else {
        self.lose(
          "discriminator-unbound",
          &alternative_pointer,
          format!("the alternative does not bind '{property}' to a literal, so everything below is admitted"),
        );
        return json!({ "shape": "any" });
      };
      let name = literal.unwrap_or_else(|| format!("alternative-{index}"));
      mapped.insert(name, bound);
    }
    json!({ "alternatives": Value::Object(mapped), "discriminator": property, "shape": "one-of" })
  }

  /// `allOf`'s branches merged into one object: their properties and required lists, the first type.
  fn merge_all_of(&mut self, schema: &Value, branches: &[Value], pointer: &str) -> Value {
    let mut merged = schema.as_object().cloned().unwrap_or_default();
    merged.remove("allOf");
    let mut properties = merged
      .get("properties")
      .and_then(Value::as_object)
      .cloned()
      .unwrap_or_default();
    let mut required: Vec<Value> = merged
      .get("required")
      .and_then(Value::as_array)
      .cloned()
      .unwrap_or_default();
    for (index, branch) in branches.iter().enumerate() {
      let branch = match branch.get("$ref").and_then(Value::as_str) {
        Some(reference) => self.lookup(reference).cloned().unwrap_or(Value::Null),
        None => branch.clone(),
      };
      if branch.get("oneOf").is_some() || branch.get("anyOf").is_some() || branch.get("allOf").is_some() {
        self.lose(
          "all-of-union",
          &format!("{pointer}/allOf/{index}"),
          "a branch of allOf holds its own composition, which is not merged; its properties are",
        );
      }
      for (name, property) in branch
        .get("properties")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
      {
        properties.insert(name.clone(), property.clone());
      }
      required.extend(
        branch
          .get("required")
          .and_then(Value::as_array)
          .into_iter()
          .flatten()
          .cloned(),
      );
      if !merged.contains_key("type")
        && let Some(kind) = branch.get("type")
      {
        merged.insert("type".to_string(), kind.clone());
      }
      for keyword in ["additionalProperties", "nullable"] {
        if !merged.contains_key(keyword)
          && let Some(value) = branch.get(keyword)
        {
          merged.insert(keyword.to_string(), value.clone());
        }
      }
    }
    merged.insert("properties".to_string(), Value::Object(properties));
    merged.insert("required".to_string(), Value::Array(required));
    merged.entry("type").or_insert_with(|| Value::from("object"));
    Value::Object(merged)
  }
}

/// OpenAPI 3.0's `nullable: true`, and 3.1's `type: [X, "null"]` and `anyOf: [X, {type: null}]`,
/// which all mean the same thing.
pub(super) fn strip_null(schema: &Value) -> (Value, bool) {
  let mut schema = schema.clone();
  let mut nullable = schema.get("nullable") == Some(&Value::Bool(true));
  if let Some(object) = schema.as_object_mut() {
    object.remove("nullable");
  }
  if let Some(types) = schema.get("type").and_then(Value::as_array).cloned() {
    let rest: Vec<Value> = types
      .iter()
      .filter(|kind| kind.as_str() != Some("null"))
      .cloned()
      .collect();
    nullable |= rest.len() != types.len();
    if let Some(object) = schema.as_object_mut() {
      match rest.as_slice() {
        [only] => {
          object.insert("type".to_string(), only.clone());
        }
        [] => {
          object.insert("type".to_string(), Value::from("null"));
        }
        _ => {
          object.insert("type".to_string(), Value::Array(rest));
        }
      }
    }
  }
  if let Some(options) = schema.get("enum").and_then(Value::as_array)
    && options.iter().any(Value::is_null)
    && options.iter().any(|option| !option.is_null())
  {
    nullable = true;
  }
  for keyword in ["anyOf", "oneOf"] {
    if let Some(branches) = schema.get(keyword).and_then(Value::as_array).cloned() {
      let is_null = |branch: &Value| branch.get("type").and_then(Value::as_str) == Some("null");
      if branches.iter().any(is_null) {
        let rest: Vec<Value> = branches
          .iter()
          .filter(|branch| !is_null(branch))
          .cloned()
          .collect();
        nullable = true;
        if let Some(object) = schema.as_object_mut() {
          if let [only] = rest.as_slice() {
            object.remove(keyword);
            return merge_into(schema, only, nullable);
          }
          object.insert(keyword.to_string(), Value::Array(rest));
        }
      }
    }
  }
  (schema, nullable)
}

/// The one non-null branch of a nullable union, keeping the keywords beside it (a `discriminator`, an
/// `example`).
fn merge_into(mut outer: Value, branch: &Value, nullable: bool) -> (Value, bool) {
  // A reference stands alone: what it names is the schema, and the keywords beside it annotate.
  if branch.get("$ref").is_some() {
    return (branch.clone(), nullable);
  }
  if let (Some(outer), Some(branch)) = (outer.as_object_mut(), branch.as_object()) {
    for (key, value) in branch {
      outer.entry(key.clone()).or_insert_with(|| value.clone());
    }
  }
  (outer, nullable)
}

/// The schema's own example, else one made up for its kind.
fn example_or(schema: &Value, made_up: Value) -> Value {
  schema
    .get("example")
    .or_else(|| {
      schema
        .get("examples")
        .and_then(Value::as_array)
        .and_then(|examples| examples.first())
    })
    .filter(|example| std::mem::discriminant(*example) == std::mem::discriminant(&made_up))
    .cloned()
    .unwrap_or(made_up)
}

fn regex_admits(pattern: &str, text: &str) -> bool {
  regex::Regex::new(pattern).is_ok_and(|regex| regex.is_match(text))
}

/// An alternative whose discriminator member is already a literal, which the walk keys it by.
fn already_bound(shape: &Value, property: &str) -> bool {
  matches!(
    shape
      .pointer(&format!("/members/{}/shape", escape(property)))
      .and_then(Value::as_str),
    Some("equality" | "any-of")
  )
}

/// The alternative with its discriminator member bound to `literal`, as shape spec §5.4 needs.
fn bind_discriminator(shape: &Value, property: &str, literal: &str) -> Option<Value> {
  let mut shape = shape.clone();
  let members = shape.get_mut("members")?.as_object_mut()?;
  let current = members.get(property)?;
  let bound = matches!(
    current.get("shape").and_then(Value::as_str),
    Some("equality" | "any-of")
  );
  if !bound {
    members.insert(
      property.to_string(),
      json!({ "example": literal, "shape": "equality" }),
    );
  }
  Some(shape)
}

/// A JSON pointer's escaping of one reference token (RFC 6901).
pub(super) fn escape(token: &str) -> String {
  token.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn every_spelling_of_nullable_is_one() {
    for schema in [
      json!({ "type": "string", "nullable": true }),
      json!({ "type": ["string", "null"] }),
      json!({ "anyOf": [{ "type": "string" }, { "type": "null" }] }),
    ] {
      let (stripped, nullable) = strip_null(&schema);
      assert!(nullable, "{schema}");
      assert_eq!(stripped["type"], "string", "{schema}");
    }
  }
}
