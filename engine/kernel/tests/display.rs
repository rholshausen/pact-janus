//! Display text's segments on the wire (engine-protocol spec §2.7, ADR 0054): sent beside the text
//! to a host that declared the `segments` capability, never to one that did not, and always joining
//! to exactly the text.

use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "r-1", "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal serializes"));
  serde_json::from_slice(&bytes).expect("Engine::dispatch always returns valid JSON")
}

fn ok(engine: &mut Engine, op: &str, body: Value) -> Value {
  let response = send(engine, op, body);
  assert!(response.get("ok").is_some(), "{op} failed: {}", response["error"]);
  response["ok"].clone()
}

/// An engine past its handshake, with the host asking for segments or not.
fn engine(segments: bool) -> Engine {
  let mut engine = Engine::new();
  let capabilities = if segments {
    json!({ "segments": {} })
  } else {
    json!({})
  };
  let hello = ok(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "capabilities": capabilities }),
  );
  assert_eq!(
    hello["capabilities"]["segments"],
    json!({}),
    "the engine says it writes them"
  );
  engine
}

/// A validator for `reference` among the protocol's schemas, each found by its `$id`.
fn validator(reference: &str) -> jsonschema::Validator {
  struct InMemory(std::collections::HashMap<String, Value>);
  impl jsonschema::Retrieve for InMemory {
    fn retrieve(
      &self,
      uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
      self
        .0
        .get(uri.as_str())
        .cloned()
        .ok_or_else(|| format!("schema not registered: {uri}").into())
    }
  }
  let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../Documentation/specs/engine-protocol/schemas/v1");
  let mut schemas = std::collections::HashMap::new();
  for entry in std::fs::read_dir(&dir).expect("the protocol's schemas") {
    let path = entry.expect("a directory entry").path();
    if path.to_string_lossy().ends_with(".schema.json") {
      let document: Value = serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("JSON");
      schemas.insert(document["$id"].as_str().expect("an $id").to_string(), document);
    }
  }
  jsonschema::options()
    .with_retriever(InMemory(schemas))
    .build(&json!({ "$ref": format!("https://pact.io/janus/protocol/v1/{reference}") }))
    .unwrap_or_else(|err| panic!("compiling {reference}: {err}"))
}

fn assert_valid(reference: &str, document: &Value) {
  let validator = validator(reference);
  let errors: Vec<String> = validator
    .iter_errors(document)
    .map(|err| err.to_string())
    .collect();
  assert!(errors.is_empty(), "{reference}: {errors:?}");
}

/// The segments' texts, joined.
fn joined(segments: &Value) -> String {
  segments
    .as_array()
    .expect("segments are a list")
    .iter()
    .map(|segment| segment["text"].as_str().expect("every segment has text"))
    .collect()
}

/// The texts of the segments of `kind`.
fn of_kind<'a>(segments: &'a Value, kind: &str) -> Vec<&'a str> {
  segments
    .as_array()
    .expect("segments are a list")
    .iter()
    .filter(|segment| segment["kind"] == json!(kind))
    .map(|segment| segment["text"].as_str().expect("every segment has text"))
    .collect()
}

fn order_pact() -> Value {
  let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../samples/order-service/pacts/web-app-order-service.json");
  serde_json::from_slice(&std::fs::read(&path).expect("the sample pact")).expect("JSON")
}

fn explain(engine: &mut Engine) -> Value {
  ok(
    engine,
    "verification/explain",
    json!({
      "interaction": { "kind": "contract-interaction", "contract": order_pact(), "index": 0 },
      "values": { "$.response.status": 401 },
    }),
  )
}

#[test]
fn a_plan_is_marked_only_for_a_host_that_asked() {
  let marked = explain(&mut engine(true));
  assert_valid("verification.schema.json#/$defs/ExplainResult", &marked);
  for text in [&marked, &marked["executed"]] {
    let segments = &text["text-segments"];
    assert_eq!(joined(segments), text["text"].as_str().unwrap_or_default());
    assert!(
      of_kind(segments, "path").contains(&"$.response.status"),
      "{segments}"
    );
    assert!(
      of_kind(segments, "action").contains(&"%match:equality"),
      "{segments}"
    );
  }
  assert!(
    of_kind(&marked["executed"]["text-segments"], "fail").contains(&"BOOL(false)"),
    "{}",
    marked["executed"]["text-segments"]
  );

  // A mismatch's message, with the values it quotes marked.
  let mismatch = marked["executed"]["mismatches"]
    .as_array()
    .and_then(|mismatches| {
      mismatches
        .iter()
        .find(|mismatch| mismatch["path"] == json!("$.response.status"))
    })
    .cloned()
    .expect("the status mismatch");
  assert_eq!(mismatch["message"], json!("Expected 401 to equal 200"));
  assert_eq!(
    mismatch["message-segments"],
    json!([
      { "text": "Expected " },
      { "text": "401", "kind": "number" },
      { "text": " to equal " },
      { "text": "200", "kind": "number" },
    ])
  );

  let plain = explain(&mut engine(false));
  assert!(
    plain["executed"]["mismatches"]
      .as_array()
      .is_some_and(|mismatches| mismatches
        .iter()
        .all(|mismatch| mismatch.get("message-segments").is_none())),
    "{plain}"
  );
  assert_eq!(plain["text"], marked["text"]);
  assert_eq!(plain["executed"]["text"], marked["executed"]["text"]);
  assert!(plain.get("text-segments").is_none(), "{plain}");
  assert!(plain["executed"].get("text-segments").is_none(), "{plain}");
}

fn decide(engine: &mut Engine) -> Value {
  let contract = json!({ "$format": "janus-contract/1",
    "consumer": { "name": "order-consumer" },
    "provider": { "name": "order-service" },
    "interactions": [
      { "description": "get an order",
        "parts": { "response": { "body": {
          "shape": "object",
          "members": {
            "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED"], "example": "PENDING" } } } } },
        "selection": { "variants": [], "report": {} } } ] });
  let shape = json!({ "$format": "janus-provider-shape/1",
    "provider": { "name": "order-service" },
    "provenance": "recorded",
    "interactions": [
      { "description": "get an order",
        "parts": { "response": { "body": {
          "shape": "object",
          "members": {
            "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED", "DELIVERED"],
                        "example": "PENDING" } } } } } } ] });
  let checked = ok(
    engine,
    "subsumption/check",
    json!({ "contract": contract, "provider-shape": shape }),
  );
  ok(
    engine,
    "subsumption/decide",
    json!({
      "pairs": [{
        "consumer": { "name": "order-consumer" },
        "provider": { "name": "order-service" },
        "format": checked["format"],
        "subsumption": checked["report"],
      }],
      "verification": [],
      "policy": [{ "on-finding": "block" }],
      "as-of": "2026-10-26",
    }),
  )
}

/// The `can-i-deploy` page's markers by what they say, its address as a path, and its policy quiet.
#[test]
fn the_page_is_marked_by_what_each_marker_says() {
  let decided = decide(&mut engine(true));
  assert_valid("subsumption.schema.json#/$defs/DecideResult", &decided);
  let segments = &decided["text-segments"];
  assert_eq!(joined(segments), decided["text"].as_str().unwrap_or_default());
  assert_eq!(segments[0], json!({ "text": "✗ ", "kind": "fail" }));
  assert_eq!(
    segments[1],
    json!({ "text": "order-consumer is not compatible with order-service", "kind": "heading" })
  );
  assert!(of_kind(segments, "path").contains(&"$.status"), "{segments}");
  assert!(
    of_kind(segments, "fail").contains(&"=> BLOCK: order-consumer -> order-service"),
    "{segments}"
  );
  assert!(
    of_kind(segments, "quiet")
      .iter()
      .any(|text| text.starts_with(" — policy on-finding block")),
    "{segments}"
  );

  let plain = decide(&mut engine(false));
  assert_eq!(plain["text"], decided["text"]);
  assert!(plain.get("text-segments").is_none(), "{plain}");
}
