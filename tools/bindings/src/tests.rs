use super::*;
use expectest::prelude::*;
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::json;

fn files(docs: Vec<(&str, Value)>) -> Vec<(String, Value)> {
  docs.into_iter().map(|(n, v)| (n.to_string(), v)).collect()
}

fn named<'a>(schemas: &'a [NamedSchema], name: &str) -> &'a Value {
  &schemas
    .iter()
    .find(|s| s.name == name)
    .unwrap_or_else(|| panic!("no type {name}"))
    .schema
}

#[test]
fn a_container_file_contributes_only_its_definitions() {
  let schemas = normalise(&files(vec![(
    "ops.schema.json",
    json!({ "$id": "https://example/ops.schema.json", "title": "Ops",
            "$defs": { "Create": { "title": "Create", "type": "object" },
                       "Untitled": { "type": "string" } } }),
  )]))
  .unwrap();
  let names: Vec<_> = schemas.iter().map(|s| s.name.as_str()).collect();
  assert_eq!(names, vec!["Create", "Untitled"]);
}

#[test]
fn a_typed_root_is_a_type_and_loses_its_id_and_defs() {
  let schemas = normalise(&files(vec![(
    "frame.schema.json",
    json!({ "$schema": "https://json-schema.org/draft/2020-12/schema", "$id": "https://example/frame.schema.json",
            "title": "Frame", "type": "object",
            "$defs": { "Inner": { "title": "Inner", "type": "object" } } }),
  )]))
  .unwrap();
  assert_eq!(
    named(&schemas, "Frame"),
    &json!({ "$schema": "https://json-schema.org/draft/2020-12/schema", "title": "Frame", "type": "object" })
  );
}

#[rstest]
#[case::recursive_root("#", "#")]
#[case::own_file("shape.schema.json", "#")]
#[case::local_definition("#/$defs/Inner", "Inner.json")]
#[case::other_file_root("error.schema.json", "EngineError.json")]
#[case::other_file_definition("error.schema.json#/$defs/Detail", "Detail.json")]
fn refs_are_rewritten_to_the_named_types_file(#[case] reference: &str, #[case] expected: &str) {
  let schemas = normalise(&files(vec![
    (
      "shape.schema.json",
      json!({ "title": "Shape", "type": "object",
              "properties": { "of": { "$ref": reference } },
              "$defs": { "Inner": { "title": "Inner", "type": "object" } } }),
    ),
    (
      "error.schema.json",
      json!({ "title": "EngineError", "type": "object",
              "$defs": { "Detail": { "title": "Detail", "type": "object" } } }),
    ),
  ]))
  .unwrap();
  expect!(named(&schemas, "Shape")["properties"]["of"]["$ref"].as_str()).to(be_some().value(expected));
}

#[test]
fn a_titled_nested_schema_is_hoisted_into_a_type_of_its_own() {
  let schemas = normalise(&files(vec![(
    "ops.schema.json",
    json!({ "title": "Create", "type": "object", "properties": {
      "config": { "title": "SessionConfig", "type": "object", "properties": {
        "self": { "$ref": "#" },
        "party": { "$ref": "#/$defs/Party" },
        "example": { "const": { "title": "not a type" } } } } },
      "$defs": { "Party": { "title": "Party", "type": "object" } } }),
  )]))
  .unwrap();
  assert_eq!(
    named(&schemas, "Create")["properties"]["config"],
    json!({ "$ref": "SessionConfig.json" })
  );
  assert_eq!(
    named(&schemas, "SessionConfig")["properties"],
    json!({ "self": { "$ref": "Create.json" }, "party": { "$ref": "Party.json" },
            "example": { "const": { "title": "not a type" } } })
  );
}

#[test]
fn a_ref_to_anything_but_a_type_is_an_error() {
  let result = normalise(&files(vec![(
    "a.schema.json",
    json!({ "title": "A", "type": "object", "properties": { "b": { "$ref": "#/properties/c" } } }),
  )]));
  assert_eq!(
    result.unwrap_err(),
    vec![
      "a.schema.json: $ref '#/properties/c' names no type: only a file's root or one of its $defs can be referenced"
    ]
  );
}

#[test]
fn duplicate_titles_within_a_set_are_an_error() {
  let result = normalise(&files(vec![
    (
      "a.schema.json",
      json!({ "$defs": { "Party": { "title": "Party", "type": "object" } } }),
    ),
    (
      "b.schema.json",
      json!({ "$defs": { "Party": { "title": "Party", "type": "object" } } }),
    ),
  ]));
  expect!(result).to(be_err());
}

#[test]
fn a_string_const_gains_the_type_it_implies() {
  let schemas = normalise(&files(vec![(
    "f.schema.json",
    json!({ "title": "F", "type": "object",
            "properties": { "type": { "const": "request" }, "n": { "const": 1 } } }),
  )]))
  .unwrap();
  let props = &named(&schemas, "F")["properties"];
  assert_eq!(props["type"], json!({ "const": "request", "type": "string" }));
  assert_eq!(props["n"], json!({ "const": 1 }));
}

fn vocabularies_of(schema: Value) -> Vec<Vocabulary> {
  vocabularies(&normalise(&files(vec![("s.schema.json", schema)])).unwrap()).unwrap()
}

#[test]
fn a_vocabulary_is_named_after_its_type_and_member_path() {
  let found = vocabularies_of(json!({
    "$defs": {
      "RequestFrame": { "title": "RequestFrame", "type": "object",
        "properties": { "op": { "type": "string", "x-known-values": ["engine/hello"] } } },
      "Hello": { "title": "Hello", "type": "object",
        "properties": {
          "capabilities": { "type": "object", "properties": {
            "events": { "type": "array", "items": { "type": "string", "x-known-values": ["push-events"] } } } },
          "config": { "title": "SessionConfig", "type": "object", "properties": {
            "mode": { "type": "string", "x-known-values": ["strict"] } } } } },
      "EncodingName": { "title": "EncodingName", "type": "string", "x-known-values": ["json"] } }
  }));
  let names: Vec<_> = found
    .iter()
    .map(|v| (v.name.as_str(), v.source.as_str()))
    .collect();
  assert_eq!(
    names,
    vec![
      ("EncodingName", "EncodingName"),
      ("HelloCapabilitiesEvents", "Hello.capabilities.events"),
      ("RequestFrameOp", "RequestFrame.op"),
      ("SessionConfigMode", "SessionConfig.mode"),
    ]
  );
}

#[test]
fn conflicting_values_under_one_vocabulary_name_are_an_error() {
  let schemas = normalise(&files(vec![(
    "s.schema.json",
    json!({ "$defs": {
      "A": { "title": "A", "type": "object", "properties": {
        "bC": { "type": "string", "x-known-values": ["one"] } } },
      "AB": { "title": "AB", "type": "object", "properties": {
        "c": { "type": "string", "x-known-values": ["two"] } } } } }),
  )]))
  .unwrap();
  expect!(vocabularies(&schemas)).to(be_err());
}

fn vocabulary(values: &[&str]) -> Vocabulary {
  Vocabulary {
    name: "RequestFrameOp".to_string(),
    source: "RequestFrame.op".to_string(),
    values: values.iter().map(|v| v.to_string()).collect(),
  }
}

#[test]
fn typescript_vocabularies_are_const_objects_keyed_in_pascal_case() {
  let out =
    render_typescript_vocabularies(&[vocabulary(&["consumer-session/create", "janus:csv", "3xx"])]).unwrap();
  expect!(out.contains(
    "export const RequestFrameOp = {\n  ConsumerSessionCreate: \"consumer-session/create\",\n  JanusCsv: \"janus:csv\",\n  V_3xx: \"3xx\",\n} as const;\n"
  ))
  .to(be_true());
}

#[test]
fn java_vocabularies_are_nested_constant_classes_in_upper_snake_case() {
  let out = render_java_vocabularies(
    "io.pact.janus.bindings.protocol.v1",
    &[vocabulary(&["engine/hello", "requestTimeout"])],
  )
  .unwrap();
  expect!(out.starts_with("package io.pact.janus.bindings.protocol.v1;\n")).to(be_true());
  expect!(out.contains("    public static final String ENGINE_HELLO = \"engine/hello\";\n")).to(be_true());
  expect!(out.contains("    public static final String REQUEST_TIMEOUT = \"requestTimeout\";\n"))
    .to(be_true());
  expect!(
    out.contains("    public static final List<String> KNOWN = List.of(ENGINE_HELLO, REQUEST_TIMEOUT);\n")
  )
  .to(be_true());
}

#[rstest]
#[case(&["a-b", "a/b"])]
#[case(&["--"])]
fn values_that_cannot_be_told_apart_as_identifiers_are_an_error(#[case] values: &[&str]) {
  expect!(render_typescript_vocabularies(&[vocabulary(values)])).to(be_err());
  expect!(render_java_vocabularies("p", &[vocabulary(values)])).to(be_err());
}

#[test]
fn typescript_reads_an_annotation_only_schema_as_unknown_and_keeps_annotated_recursion_named() {
  let schema = NamedSchema {
    name: "Shape".to_string(),
    schema: json!({ "title": "Shape", "type": "object", "properties": {
      "example": { "description": "any value", "x-tagged-by": "encoded" },
      "of": { "$ref": "#", "description": "the wrapped shape" },
      "items": { "$ref": "#" } } }),
  };
  assert_eq!(
    prepare_for_typescript(&schema).schema["properties"],
    json!({
      "example": { "description": "any value", "x-tagged-by": "encoded", "tsType": "unknown" },
      "of": { "allOf": [{ "$ref": "#" }], "description": "the wrapped shape" },
      "items": { "$ref": "#" } })
  );
}

#[test]
fn jvm_reads_an_untitled_open_object_as_a_map_and_drops_defaults() {
  let schema = NamedSchema {
    name: "Shape".to_string(),
    schema: json!({ "title": "Shape", "type": "object", "properties": {
      "body": { "type": "object", "description": "open" },
      "members": { "type": "object", "additionalProperties": { "$ref": "#" } },
      "parts": { "type": "object", "additionalProperties": { "$ref": "ShapePart.json" } },
      "labels": { "type": "object", "additionalProperties": { "type": "string" } },
      "config": { "type": "object", "properties": { "a": { "type": "string" } } },
      "last": { "type": "boolean", "default": false } } }),
  };
  let props = &prepare_for_jvm(&schema, "p.v1").schema["properties"];
  expect!(props["body"]["existingJavaType"].as_str()).to(be_some().value("java.util.Map<String, Object>"));
  expect!(props["members"]["existingJavaType"].as_str())
    .to(be_some().value("java.util.Map<String, p.v1.Shape>"));
  expect!(props["parts"]["existingJavaType"].as_str())
    .to(be_some().value("java.util.Map<String, p.v1.ShapePart>"));
  expect!(props["labels"]["existingJavaType"].as_str()).to(be_some().value("java.util.Map<String, String>"));
  expect!(props["config"].get("existingJavaType")).to(be_none());
  assert_eq!(props["last"], json!({ "type": "boolean" }));
}
