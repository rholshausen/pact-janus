//! A content component for `x/wrapped`, whose octets are `<type>\n<payload>`: a payload of another
//! type, as a multipart part's body is (component-interfaces spec §6.6, ADR 0103). It decodes nothing
//! itself. `content/decode` asks the engine for the payload and answers `{ type, payload }`, or
//! `{ type, failed }` with the code of the engine's error, the payload's `encoded` kept beside it when
//! it is octets. `content/encode` asks the engine to encode a `{ type, payload }` document's payload,
//! at `/payload`, and writes the octets it is given after the type.
//!
//! Every answer refuses, as `invalid-config`, when the handshake did not say the engine answers
//! embedded content.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};

wit_bindgen::generate!({ world: "component", path: "wit" });

/// Whether this instance's handshake said the engine answers embedded content.
static ANSWERED: AtomicBool = AtomicBool::new(false);

struct Wrapping;

impl exports::pact::janus_component::pipe::Guest for Wrapping {
  fn call(request: Vec<u8>) -> Vec<u8> {
    let request: Value = serde_json::from_slice(&request).unwrap_or(Value::Null);
    let id = request["id"].clone();
    let body = &request["body"];
    let answer = match request["op"].as_str().unwrap_or_default() {
      "component/hello" => {
        ANSWERED.store(
          body["capabilities"].get("embedded-content").is_some(),
          Ordering::SeqCst,
        );
        Ok(json!({
          "component-protocol-version": 1,
          "component": { "name": "wrapping", "version": "1.0.0" },
          "interfaces": ["content"],
          "contributes": { "content-types": [ { "media-type": "x/wrapped" } ] }
        }))
      }
      _ if !ANSWERED.load(Ordering::SeqCst) => Err(error(
        "invalid-config",
        "this engine does not answer embedded content",
      )),
      "content/decode" => decode(body),
      "content/encode" => encode(body),
      op => Err(error("operation-unsupported", op)),
    };
    let frame = match answer {
      Ok(ok) => json!({ "type": "response", "id": id, "ok": ok }),
      Err(err) => json!({ "type": "response", "id": id, "error": err }),
    };
    serde_json::to_vec(&frame).unwrap()
  }
}

fn error(code: &str, message: &str) -> Value {
  json!({ "code": code, "category": "document", "message": message })
}

/// A slot value's octets.
fn octets(value: &Value) -> Result<Vec<u8>, Value> {
  let content = value["content"].as_str().unwrap_or_default();
  match value["encoded"].as_str() {
    Some("base64") => BASE64
      .decode(content)
      .map_err(|err| error("decode-failed", &err.to_string())),
    _ => Ok(content.as_bytes().to_vec()),
  }
}

fn decode(body: &Value) -> Result<Value, Value> {
  let octets = octets(&body["value"])?;
  let text = String::from_utf8(octets).map_err(|err| error("decode-failed", &err.to_string()))?;
  let (content_type, payload) = text
    .split_once('\n')
    .ok_or_else(|| error("decode-failed", "no type line"))?;
  let Some(answers) = body.get("embedded") else {
    // A question still carries the members an answer requires, which the engine does not read.
    return Ok(json!({ "document": null, "embedded": [ {
      "id": "payload",
      "content-type": content_type,
      "value": { "content": BASE64.encode(payload), "encoded": "base64", "content-type": content_type }
    } ] }));
  };
  let answer = &answers["payload"];
  let mut document = json!({ "type": content_type });
  match answer.get("error") {
    Some(error) => document["failed"] = error["code"].clone(),
    None => {
      document["payload"] = answer["document"].clone();
      if let Some(encoded) = answer.get("encoded") {
        document["encoded"] = encoded.clone();
      }
    }
  }
  Ok(json!({ "document": document }))
}

fn encode(body: &Value) -> Result<Value, Value> {
  let document = &body["document"];
  let content_type = document["type"]
    .as_str()
    .ok_or_else(|| error("encode-failed", "no type"))?;
  let Some(answers) = body.get("embedded") else {
    return Ok(json!({ "value": { "content": null }, "embedded": [ {
      "id": "payload",
      "content-type": content_type,
      "document": document["payload"],
      "at": "/payload"
    } ] }));
  };
  let answer = &answers["payload"];
  if let Some(error) = answer.get("error") {
    return Err(error.clone());
  }
  let payload = octets(&answer["value"])?;
  let mut written = format!("{content_type}\n").into_bytes();
  written.extend_from_slice(&payload);
  Ok(
    json!({ "value": { "content": BASE64.encode(written), "encoded": "base64", "content-type": "x/wrapped" } }),
  )
}

export!(Wrapping);
