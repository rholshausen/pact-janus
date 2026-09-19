//! The binding-generation pipeline's language-independent half (plan task 6.1).
//!
//! The spec schemas are authored for people and for the schema-compat checker: several types per
//! file under `$defs`, files that are pure `$defs` containers, recursive `"$ref": "#"`, and
//! relative references between files of one version directory. Binding generators handle that
//! unevenly — jsonschema2pojo only generates what is reachable from a schema's root, so a
//! container file produces nothing, and names classes after files rather than titles. So the
//! pipeline normalises first: one standalone schema per titled type (a non-container root, or a
//! `$defs` entry), named by its title, every `$ref` rewritten to the file of the type it names.
//! Both generators read the same normalised set, so neither sees a schema the other doesn't.
//!
//! Neither generator does anything with `x-known-values` (engine-protocol spec §2.2 rule 1): an
//! open vocabulary is a plain string to both, so an SDK would have to hand-copy operation names
//! and error codes out of the specs. [`vocabularies`] collects them instead, and the renderers
//! write them as constants — generated like the rest of the bindings, never hand-edited.

use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

/// `sdks/bindings.json`: which spec schema directories the SDKs are built from, and what each
/// language calls the result.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
  pub sets: Vec<SchemaSet>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchemaSet {
  /// The set's short name, used for its staging directory.
  pub name: String,
  /// A schema version directory, relative to the repository root.
  pub schemas: PathBuf,
  /// The TypeScript module the set generates, `src/generated/<module>.ts`.
  pub typescript: String,
  /// The Java package the set generates.
  pub jvm: String,
}

/// One standalone schema: a single type, named by its title, with every `$ref` pointing at
/// another standalone schema's file.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedSchema {
  pub name: String,
  pub schema: Value,
}

impl NamedSchema {
  pub fn file_name(&self) -> String {
    file_name(&self.name)
  }
}

fn file_name(type_name: &str) -> String {
  format!("{type_name}.json")
}

/// One open vocabulary: the values an `x-known-values` annotation lists, named after the type
/// and member that carry it (`RequestFrame` + `op` → `RequestFrameOp`).
#[derive(Debug, Clone, PartialEq)]
pub struct Vocabulary {
  pub name: String,
  /// Where it was found, `Type.member.path`, for the rendered doc comment.
  pub source: String,
  pub values: Vec<String>,
}

/// Is this file a type in its own right, or only a container for its `$defs`?
fn is_container(root: &Value) -> bool {
  [
    "type",
    "properties",
    "allOf",
    "anyOf",
    "oneOf",
    "$ref",
    "const",
    "items",
  ]
  .iter()
  .all(|k| root.get(k).is_none())
}

fn title_or(schema: &Value, fallback: &str) -> String {
  schema
    .get("title")
    .and_then(Value::as_str)
    .unwrap_or(fallback)
    .to_string()
}

/// Normalises one schema version directory (`(file name, document)` pairs) into one standalone
/// schema per type. Errors are every problem found, not just the first.
pub fn normalise(files: &[(String, Value)]) -> Result<Vec<NamedSchema>, Vec<String>> {
  let mut errors = Vec::new();

  // (file, pointer) -> type name, pointer "" for a root and "/$defs/<key>" for a definition.
  let mut index: BTreeMap<(String, String), String> = BTreeMap::new();
  // type name -> (file, body), in file then definition order.
  let mut types: Vec<(String, String, Value)> = Vec::new();

  for (file, root) in files {
    if !is_container(root) {
      let name = title_or(root, file.trim_end_matches(".schema.json"));
      index.insert((file.clone(), String::new()), name.clone());
      let mut body = root.clone();
      if let Some(obj) = body.as_object_mut() {
        obj.remove("$defs");
      }
      types.push((name, file.clone(), body));
    }
    if let Some(defs) = root.get("$defs").and_then(Value::as_object) {
      for (key, def) in defs {
        let name = title_or(def, key);
        index.insert((file.clone(), format!("/$defs/{key}")), name.clone());
        types.push((name, file.clone(), def.clone()));
      }
    }
  }

  // A titled schema nested inside a type is a type too — both generators name it by its title — so
  // it is hoisted to a file of its own, exactly like a `$defs` entry.
  let mut rewritten: Vec<(String, String, Value)> = Vec::new();
  for (name, file, mut body) in types {
    rewrite(&mut body, &name, &file, &index, &mut errors);
    let mut hoisted = Vec::new();
    hoist(&mut body, &name, &mut hoisted);
    rewritten.push((name, file.clone(), body));
    rewritten.extend(hoisted.into_iter().map(|(n, b)| (n, file.clone(), b)));
  }

  let mut seen: BTreeMap<String, String> = BTreeMap::new();
  for (name, file, _) in &rewritten {
    if let Some(first) = seen.insert(name.clone(), file.clone()) {
      errors.push(format!(
        "type name '{name}' is defined in both {first} and {file}: titles must be unique within a set"
      ));
    }
  }

  let mut out = Vec::new();
  for (name, _, mut body) in rewritten {
    if let Some(obj) = body.as_object_mut() {
      obj.remove("$id");
      obj.remove("$schema");
      obj.insert("title".to_string(), Value::String(name.clone()));
      let mut ordered = Map::new();
      ordered.insert(
        "$schema".to_string(),
        Value::String("https://json-schema.org/draft/2020-12/schema".to_string()),
      );
      ordered.extend(std::mem::take(obj));
      *obj = ordered;
    }
    out.push(NamedSchema { name, schema: body });
  }

  if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// The subschemas directly under a schema node — never its `const`, `default`, `examples` or other
/// value positions, where a `title` member is data rather than a type name.
fn subschemas_mut(node: &mut Value) -> Vec<&mut Value> {
  let Some(obj) = node.as_object_mut() else {
    return Vec::new();
  };
  let mut out = Vec::new();
  for (key, value) in obj.iter_mut() {
    match key.as_str() {
      "properties" | "patternProperties" | "dependentSchemas" => {
        if let Some(map) = value.as_object_mut() {
          out.extend(map.values_mut());
        }
      }
      "allOf" | "anyOf" | "oneOf" | "prefixItems" => {
        if let Some(items) = value.as_array_mut() {
          out.extend(items.iter_mut());
        }
      }
      "items" | "additionalProperties" | "not" | "if" | "then" | "else" | "contains" | "propertyNames"
        if value.is_object() =>
      {
        out.push(value);
      }
      _ => {}
    }
  }
  out
}

/// Moves every titled subschema under `node` out into `hoisted`, leaving a `$ref` to it. A `#`
/// inside a hoisted subschema meant the type it was nested in, `root`, and is pointed there.
fn hoist(node: &mut Value, root: &str, hoisted: &mut Vec<(String, Value)>) {
  for child in subschemas_mut(node) {
    match child.get("title").and_then(Value::as_str).map(str::to_string) {
      Some(title) => {
        let mut body = std::mem::replace(child, serde_json::json!({ "$ref": file_name(&title) }));
        retarget_self_references(&mut body, root);
        hoist(&mut body, root, hoisted);
        hoisted.push((title, body));
      }
      None => hoist(child, root, hoisted),
    }
  }
}

fn retarget_self_references(node: &mut Value, root: &str) {
  match node {
    Value::Object(obj) => {
      if obj.get("$ref").and_then(Value::as_str) == Some("#") {
        obj.insert("$ref".to_string(), Value::String(file_name(root)));
      }
      obj.values_mut().for_each(|v| retarget_self_references(v, root));
    }
    Value::Array(items) => items.iter_mut().for_each(|v| retarget_self_references(v, root)),
    _ => {}
  }
}

/// Rewrites every `$ref` under `node` to the standalone file of the type it names — or to `#` when
/// that is the type being written, since a generator reads a reference to another file as another
/// type and numbers the clash (`Shape1`) — and gives a
/// string `const` without a `type` the `type` it implies (jsonschema2pojo types an untyped member
/// as `Object`; `const: "request"` admits only strings, so saying so loses nothing).
fn rewrite(
  node: &mut Value,
  current: &str,
  file: &str,
  index: &BTreeMap<(String, String), String>,
  errors: &mut Vec<String>,
) {
  match node {
    Value::Object(obj) => {
      if let Some(Value::String(reference)) = obj.get("$ref") {
        let (target_file, pointer) = match reference.split_once('#') {
          Some((f, p)) => (f, p),
          None => (reference.as_str(), ""),
        };
        let target_file = if target_file.is_empty() { file } else { target_file };
        match index.get(&(target_file.to_string(), pointer.to_string())) {
          Some(name) => {
            let target = if name == current { "#".to_string() } else { file_name(name) };
            obj.insert("$ref".to_string(), Value::String(target));
          }
          None => errors.push(format!(
            "{file}: $ref '{reference}' names no type: only a file's root or one of its $defs can be referenced"
          )),
        }
      }
      if obj.get("const").is_some_and(Value::is_string) && obj.get("type").is_none() {
        obj.insert("type".to_string(), Value::String("string".to_string()));
      }
      for value in obj.values_mut() {
        rewrite(value, current, file, index, errors);
      }
    }
    Value::Array(items) => {
      for item in items {
        rewrite(item, current, file, index, errors);
      }
    }
    _ => {}
  }
}

/// Applies `fix` to `node` and to every schema beneath it, children first.
fn each_schema(node: &mut Value, fix: &dyn Fn(&mut Map<String, Value>)) {
  for child in subschemas_mut(node) {
    each_schema(child, fix);
  }
  if let Some(obj) = node.as_object_mut() {
    fix(obj);
  }
}

/// Keywords that say what a schema's instances are. A schema with none admits any JSON value.
const TYPING: &[&str] = &[
  "type",
  "$ref",
  "allOf",
  "anyOf",
  "oneOf",
  "const",
  "enum",
  "not",
  "properties",
  "additionalProperties",
  "patternProperties",
  "items",
  "prefixItems",
  "required",
];

/// json-schema-to-typescript's reading of a normalised schema, corrected in two places where it
/// disagrees with the schema. Neither applies to jsonschema2pojo, so neither is normalisation.
///
/// 1. A schema carrying only annotations (`Shape.example`: a description and `x-tagged-by`) is typed
///    `{ [k: string]: unknown }`, though it admits any value; it is marked `tsType: "unknown"`.
/// 2. A recursive reference with annotations beside it (`{ "$ref": "#", "description": … }`) is
///    read as a new schema, named `Shape1` and never declared; a one-element `allOf` around the
///    reference keeps both the annotation and the type's own name.
pub fn prepare_for_typescript(schema: &NamedSchema) -> NamedSchema {
  let mut out = schema.clone();
  for child in subschemas_mut(&mut out.schema) {
    each_schema(child, &|obj| {
      if !TYPING.iter().any(|k| obj.contains_key(*k)) {
        obj.insert("tsType".to_string(), Value::String("unknown".to_string()));
      }
      if obj.get("$ref").and_then(Value::as_str) == Some("#") && obj.len() > 1 {
        let reference = obj.remove("$ref").unwrap_or_default();
        obj.insert("allOf".to_string(), serde_json::json!([{ "$ref": reference }]));
      }
    });
  }
  out
}

/// jsonschema2pojo's reading of a normalised schema, corrected in two places where it disagrees
/// with the schema.
///
/// 1. An untitled open object (`AddInteraction.interaction`, `RequestFrame.body`) is a map, not a
///    type — the schema names no member of it — but jsonschema2pojo makes it an empty class named
///    after the member and numbers the clashes (`Contract__1`, `Contract__2`). Such an object is
///    given `existingJavaType` `Map<String, V>`, `V` its `additionalProperties` type.
/// 2. `default` is an annotation — it tells a reader what absence means, it does not fill the
///    member in — but jsonschema2pojo initialises the field with it, so a POJO read from a
///    document without `last` writes `"last": false` back out. Bindings must not change a document
///    they pass through (spike 1.1 finding 14), so `default` is dropped; the prose keeps it.
pub fn prepare_for_jvm(schema: &NamedSchema, package: &str) -> NamedSchema {
  let mut out = schema.clone();
  let own = format!("{}.json", schema.name);
  for child in subschemas_mut(&mut out.schema) {
    each_schema(child, &|obj| {
      let open_object = obj.get("type").and_then(Value::as_str) == Some("object")
        && ![
          "title",
          "properties",
          "patternProperties",
          "allOf",
          "anyOf",
          "oneOf",
          "$ref",
          "existingJavaType",
        ]
        .iter()
        .any(|k| obj.contains_key(*k));
      if open_object {
        let values = java_type(obj.get("additionalProperties"), package, &own);
        obj.insert(
          "existingJavaType".to_string(),
          Value::String(format!("java.util.Map<String, {values}>")),
        );
      }
      obj.remove("default");
    });
  }
  out
}

fn java_type(schema: Option<&Value>, package: &str, own: &str) -> String {
  let Some(obj) = schema.and_then(Value::as_object) else {
    return "Object".to_string();
  };
  if let Some(java) = obj.get("existingJavaType").and_then(Value::as_str) {
    return java.to_string();
  }
  if let Some(reference) = obj.get("$ref").and_then(Value::as_str) {
    let reference = if reference == "#" { own } else { reference };
    return format!("{package}.{}", reference.trim_end_matches(".json"));
  }
  match obj.get("type").and_then(Value::as_str) {
    Some("string") => "String",
    Some("integer") => "Long",
    Some("number") => "Double",
    Some("boolean") => "Boolean",
    _ => "Object",
  }
  .to_string()
}

/// Collects every open vocabulary in a normalised set. A vocabulary is named after the nearest
/// titled schema above it plus the member names between, so the same member of the same type is
/// the same vocabulary wherever it is found; two different value lists under one name is an
/// error, since only one could be rendered.
pub fn vocabularies(schemas: &[NamedSchema]) -> Result<Vec<Vocabulary>, Vec<String>> {
  let mut found: BTreeMap<String, Vocabulary> = BTreeMap::new();
  let mut errors = Vec::new();
  for schema in schemas {
    walk(&schema.schema, &schema.name, &[], &mut found, &mut errors);
  }
  if errors.is_empty() {
    Ok(found.into_values().collect())
  } else {
    Err(errors)
  }
}

fn walk(
  node: &Value,
  title: &str,
  path: &[String],
  found: &mut BTreeMap<String, Vocabulary>,
  errors: &mut Vec<String>,
) {
  let Some(obj) = node.as_object() else { return };

  // A nested title starts a new name, exactly as the generators start a new type there.
  let (title, path) = match obj.get("title").and_then(Value::as_str) {
    Some(nested) if !path.is_empty() => (nested, &[][..]),
    _ => (title, path),
  };

  if let Some(values) = obj.get("x-known-values").and_then(Value::as_array) {
    let values: Vec<String> = values
      .iter()
      .filter_map(|v| v.as_str().map(str::to_string))
      .collect();
    let name = format!("{title}{}", path.iter().map(|p| pascal(p)).collect::<String>());
    let source = std::iter::once(title)
      .chain(path.iter().map(String::as_str))
      .collect::<Vec<_>>()
      .join(".");
    match found.get(&name) {
      Some(existing) if existing.values != values => errors.push(format!(
        "vocabulary '{name}' has different values at {} and {source}",
        existing.source
      )),
      Some(_) => {}
      None => {
        found.insert(name.clone(), Vocabulary { name, source, values });
      }
    }
  }

  if let Some(props) = obj.get("properties").and_then(Value::as_object) {
    for (member, schema) in props {
      let nested: Vec<String> = path
        .iter()
        .cloned()
        .chain(std::iter::once(member.clone()))
        .collect();
      walk(schema, title, &nested, found, errors);
    }
  }
  for key in ["items", "additionalProperties", "not", "if", "then", "else"] {
    if let Some(schema) = obj.get(key) {
      walk(schema, title, path, found, errors);
    }
  }
  for key in ["allOf", "anyOf", "oneOf"] {
    if let Some(Value::Array(alternatives)) = obj.get(key) {
      for schema in alternatives {
        walk(schema, title, path, found, errors);
      }
    }
  }
}

/// The alphanumeric words of a vocabulary value or member name: `consumer-session/create` →
/// `consumer`, `session`, `create`; `problemPaths` → `problem`, `paths`.
fn words(text: &str) -> Vec<String> {
  let mut words = Vec::new();
  let mut current = String::new();
  let mut previous_lower = false;
  for c in text.chars() {
    if !c.is_ascii_alphanumeric() {
      if !current.is_empty() {
        words.push(std::mem::take(&mut current));
      }
      previous_lower = false;
      continue;
    }
    if c.is_ascii_uppercase() && previous_lower {
      words.push(std::mem::take(&mut current));
    }
    previous_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
    current.push(c.to_ascii_lowercase());
  }
  if !current.is_empty() {
    words.push(current);
  }
  words
}

fn pascal(text: &str) -> String {
  words(text)
    .iter()
    .map(|w| {
      let mut chars = w.chars();
      chars
        .next()
        .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
        .unwrap_or_default()
    })
    .collect()
}

fn upper_snake(text: &str) -> String {
  words(text).join("_").to_ascii_uppercase()
}

/// Keys for a vocabulary's values in one naming style, failing on a value that yields no
/// identifier or two values that yield the same one.
fn keys(vocabulary: &Vocabulary, style: fn(&str) -> String) -> Result<Vec<(String, &str)>, String> {
  let mut seen = BTreeMap::new();
  let mut out = Vec::new();
  for value in &vocabulary.values {
    let mut key = style(value);
    if key.is_empty() {
      return Err(format!(
        "vocabulary '{}': value '{value}' has no identifier characters",
        vocabulary.name
      ));
    }
    if key.starts_with(|c: char| c.is_ascii_digit()) {
      key.insert_str(0, "V_");
    }
    if let Some(other) = seen.insert(key.clone(), value.as_str()) {
      return Err(format!(
        "vocabulary '{}': values '{other}' and '{value}' both render as '{key}'",
        vocabulary.name
      ));
    }
    out.push((key, value.as_str()));
  }
  Ok(out)
}

fn literal(value: &str) -> String {
  Value::String(value.to_string()).to_string()
}

/// Header every generated file carries, so nobody mistakes it for a file to edit.
pub const GENERATED_HEADER: &str = "Generated by tools/bindings from the spec schemas (plan task 6.1). Do not edit: regenerate with\n\
   `cargo run -p pact_janus_bindings -- generate`.";

/// A TypeScript module exporting one `as const` object per vocabulary.
pub fn render_typescript_vocabularies(vocabularies: &[Vocabulary]) -> Result<String, String> {
  let mut out = String::new();
  out.push_str("/*\n");
  for line in GENERATED_HEADER.lines() {
    let _ = writeln!(out, " * {line}");
  }
  out.push_str(" */\n");
  for vocabulary in vocabularies {
    let _ = write!(
      out,
      "\n/**\n * Known values of `{}` — an open vocabulary: values not listed here may appear and must\n * be handled by policy, never rejected (engine-protocol spec §2.2 rule 3).\n */\nexport const {} = {{\n",
      vocabulary.source, vocabulary.name
    );
    for (key, value) in keys(vocabulary, pascal)? {
      let _ = writeln!(out, "  {key}: {},", literal(value));
    }
    out.push_str("} as const;\n");
  }
  Ok(out)
}

/// A Java `Vocabulary` class nesting one constants class per vocabulary.
pub fn render_java_vocabularies(package: &str, vocabularies: &[Vocabulary]) -> Result<String, String> {
  let mut out = String::new();
  let _ = writeln!(out, "package {package};\n");
  out.push_str("import java.util.List;\n\n");
  out.push_str(
    "/**\n * Known values of this package's open vocabularies. Values not listed here may appear and\n * must be handled by policy, never rejected (engine-protocol spec §2.2 rule 3).\n */\n",
  );
  out.push_str("public final class Vocabulary {\n  private Vocabulary() {}\n");
  for vocabulary in vocabularies {
    let keys = keys(vocabulary, upper_snake)?;
    let _ = write!(
      out,
      "\n  /** Known values of {{@code {}}}. */\n  public static final class {} {{\n    private {}() {{}}\n\n",
      vocabulary.source, vocabulary.name, vocabulary.name
    );
    for (key, value) in &keys {
      let _ = writeln!(out, "    public static final String {key} = {};", literal(value));
    }
    let _ = writeln!(
      out,
      "\n    public static final List<String> KNOWN = List.of({});\n  }}",
      keys
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(", ")
    );
  }
  out.push_str("}\n");
  Ok(out)
}

#[cfg(test)]
mod tests;
