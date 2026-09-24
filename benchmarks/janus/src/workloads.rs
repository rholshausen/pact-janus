//! The documents the scenarios feed the engine. Where a baseline scenario exists, the workload is
//! the baseline's own, written the way Janus spells it: the same request bodies, the same order
//! documents (util.rs generates both), and v3 pacts shaped exactly as pact_ffi writes them.

use serde_json::{Value, json};

fn s(example: &str) -> Value {
  json!({ "shape": "string", "example": example })
}

fn int(example: i64) -> Value {
  json!({ "shape": "integer", "example": example })
}

/// A passive HTTP interaction: `POST /orders` with the given request body shape, answered `201`.
fn post_orders(description: &str, request_body: Value) -> Value {
  json!({
    "description": description,
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": {
        "method": { "shape": "equality", "example": "POST" },
        "path": { "shape": "equality", "example": "/orders" },
        "body": request_body,
      },
      "response": {
        "status": { "shape": "equality", "example": 201 },
        "body": { "shape": "equality", "example": { "id": "ORD-1", "status": "PENDING" } },
      },
    },
  })
}

/// The baseline's small request (`{sku, quantity}` under type matchers), as shapes.
pub fn small_request_interaction() -> Value {
  post_orders(
    "create an order",
    json!({ "shape": "object", "members": { "sku": s("widget-1"), "quantity": int(2) } }),
  )
}

/// The baseline's ~100 KB request. pact_ffi puts one `type` matcher at the root and cascades it to
/// every leaf. The shape language has no cascade (spec §4.1: `type` admits every value of the
/// example's *kind*, so at the root it would admit any object), and its one every-element operator,
/// `each-like`, contributes a cardinality dimension that a *request* is pinned to (variant-semantics
/// spec §4.1): the served variant accepts exactly `min` elements, so no variant admits the 310-order
/// document at all. The nearest spelling that checks every element and contributes no dimension is
/// positional: the document's own structure, every leaf typed (Phase 9 finding 25).
pub fn large_request_interaction(document: &Value) -> Value {
  post_orders("create an order history", typed(document))
}

/// `value`'s structure as shapes: objects and arrays positionally, every leaf by its kind.
pub fn typed(value: &Value) -> Value {
  match value {
    Value::Object(members) => json!({ "shape": "object",
      "members": members.iter().map(|(k, v)| (k.clone(), typed(v))).collect::<serde_json::Map<_, _>>() }),
    Value::Array(entries) => {
      json!({ "shape": "array", "entries": entries.iter().map(typed).collect::<Vec<_>>() })
    }
    Value::String(_) => json!({ "shape": "string", "example": value }),
    Value::Number(n) if n.is_i64() || n.is_u64() => json!({ "shape": "integer", "example": value }),
    Value::Number(_) => json!({ "shape": "number", "example": value }),
    Value::Bool(_) => json!({ "shape": "boolean", "example": value }),
    Value::Null => json!({ "shape": "null" }),
  }
}

/// The RFC's order payload (corpora/shapes/order-payload): `status` any-of three, `shippedAt`
/// optional, `payment` one-of two, `items` each-like — the "12-variant space" of M2 — as the
/// response of `GET /orders/66`.
pub fn rfc_order_interaction() -> Value {
  json!({
    "description": "get an order",
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": {
        "method": { "shape": "equality", "example": "GET" },
        "path": { "shape": "equality", "example": "/orders/66" },
      },
      "response": {
        "status": { "shape": "equality", "example": 200 },
        "body": { "shape": "object", "members": {
          "id": { "shape": "integer", "example": 42 },
          "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED", "DELIVERED"], "example": "PENDING" },
          "shippedAt": { "shape": "optional", "of": { "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssX",
                                                      "example": "2026-07-30T10:00:00Z" } },
          "payment": { "shape": "one-of", "discriminator": "type", "default": "card", "alternatives": {
            "card": { "shape": "object", "members": {
              "type": { "shape": "equality", "example": "card" },
              "last4": { "shape": "regex", "pattern": "\\d{4}", "example": "1234" } } },
            "invoice": { "shape": "object", "members": {
              "type": { "shape": "equality", "example": "invoice" },
              "dueDate": { "shape": "date", "format": "yyyy-MM-dd", "example": "2026-08-30" } } } } },
          "items": { "shape": "each-like", "min": 1, "items": { "shape": "object", "members": {
            "sku": s("SKU-1"), "qty": int(1) } } },
        } },
      },
    },
  })
}

/// An order interaction the sample provider (samples/order-service) can satisfy variant by
/// variant: `shippedAt` presence and `items` cardinality, each bound to the provider-state
/// parameter its v3 state endpoint already takes (variant-semantics spec §6.2).
pub fn bound_order_interaction() -> Value {
  json!({
    "description": "a request for an order",
    "transport": { "kind": "http", "mode": "passive" },
    "states": [{
      "name": "an order exists",
      "params": { "id": "66" },
      "variant-params": [
        { "name": "shipped", "dimension": "shippedAt",
          "cases": [ { "point": "present", "value": true }, { "point": "absent", "value": false } ] },
        { "name": "items", "dimension": "items",
          "cases": [ { "point": "min", "value": 1 }, { "point": "min+1", "value": 2 },
                     { "point": "max", "value": 3 } ] },
      ],
    }],
    "parts": {
      "request": {
        "method": { "shape": "equality", "example": "GET" },
        "path": { "shape": "equality", "example": "/orders/66" },
      },
      "response": {
        "status": { "shape": "equality", "example": 200 },
        "body": { "shape": "object", "members": {
          "id": s("66"),
          "status": s("PENDING"),
          "shippedAt": { "shape": "optional", "of": s("2026-07-30T09:00:00Z") },
          "items": { "shape": "each-like", "min": 1, "max": 3, "items": { "shape": "object", "members": {
            "sku": s("sku-0"), "quantity": int(1) } } },
        } },
      },
    },
  })
}

/// A v3 pact as the baseline's corpus writes it: `GET /orders/{p}/{i}` answered `200` with `body`
/// under a single root `type` matcher (pact_ffi's integration JSON, `pact:matcher:type`).
pub fn v3_pact(consumer: &str, pact: usize, interactions: usize, body: &Value) -> Value {
  let interactions: Vec<Value> = (0..interactions)
    .map(|i| {
      json!({
        "description": format!("get order {pact}/{i}"),
        "request": { "method": "GET", "path": format!("/orders/{pact}/{i}") },
        "response": {
          "status": 200,
          "headers": { "Content-Type": "application/json" },
          "body": body,
          "matchingRules": { "body": { "$": { "combine": "AND", "matchers": [ { "match": "type" } ] } } },
        },
      })
    })
    .collect();
  json!({
    "consumer": { "name": consumer },
    "provider": { "name": "bench-provider" },
    "interactions": interactions,
    "metadata": { "pactSpecification": { "version": "3.0.0" } },
  })
}
