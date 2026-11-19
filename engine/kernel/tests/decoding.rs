//! Staged plan S5.3, ADR 0101: what a reply's decoding says reaches the user. A stub content component
//! for `x/*` decodes whatever an in-tree `line` transport answers: it refuses `"ragged"`, dies on
//! `"trap"` as a binding says a trapped component did, and decodes anything else with a loss it
//! reports. The real component, over the WASM binding, is `engine/component-host/tests/csv_component.rs`.

use pact_janus_kernel::component::{
  self, Compile, CompileResult, ComponentError, ContentComponent, Decode, DecodeResult, Detect, DetectResult,
  Encode, EncodeResult, TransportComponent,
};
use pact_janus_kernel::plan::RuntimeValue;
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

/// Content for `x/*`, recording each `encode` it is asked for.
#[derive(Default)]
struct Stub {
  encoded: Mutex<Vec<Encode>>,
}

impl ContentComponent for Stub {
  fn handles(&self, content_type: &str) -> bool {
    content_type.starts_with("x/")
  }
  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    match req.value.content.as_str() {
      Some("ragged") => Err(ComponentError::decode_failed(
        &req.content_type,
        "row 2 is ragged",
      )),
      Some("trap") => Err(ComponentError::synthesised(
        "component-trapped",
        "unreachable executed",
      )),
      _ => Ok(DecodeResult {
        document: RuntimeValue::from_json(&req.value.content),
        degradations: vec![json!({ "code": "numeric-lexical", "path": "$.n", "message": "n is text" })],
      }),
    }
  }
  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    let value = component::SlotValue {
      content: req.document.to_json(),
      encoded: None,
      content_type: Some(req.content_type.clone()),
    };
    self.encoded.lock().unwrap().push(req);
    Ok(EncodeResult { value })
  }
  fn compile(&self, _: Compile) -> Result<CompileResult, ComponentError> {
    Ok(CompileResult::default())
  }
  fn detect(&self, _: Detect) -> Result<DetectResult, ComponentError> {
    Ok(DetectResult {
      media_type: None,
      confidence: None,
    })
  }
}

/// An in-tree `line` transport whose `send` answers with `answer` as the response body, labelled
/// `label`, and remembers the request it was sent.
struct Provider {
  answer: Value,
  label: &'static str,
  sent: Mutex<Vec<component::Parts>>,
}

impl TransportComponent for Provider {
  fn content_slots(&self) -> component::ContentSlots {
    component::ContentSlots::from([
      ("request".to_string(), vec!["body".to_string()]),
      ("response".to_string(), vec!["body".to_string()]),
    ])
  }
  fn exchange_parts(&self) -> component::ExchangeParts {
    component::ExchangeParts {
      initiating: "request".to_string(),
      answering: Some("response".to_string()),
    }
  }
  fn start(&self, _: component::Start) -> Result<component::StartResult, ComponentError> {
    Ok(component::StartResult { endpoint: json!({}) })
  }
  fn stop(&self, _: component::Stop) -> Result<component::StopResult, ComponentError> {
    Ok(component::StopResult {})
  }
  fn send(&self, req: component::Send) -> Result<component::SendResult, ComponentError> {
    self.sent.lock().unwrap().push(req.parts);
    let mut part = component::Part::new();
    part.insert(
      "body".to_string(),
      component::SlotValue {
        content: self.answer.clone(),
        encoded: None,
        content_type: Some(self.label.to_string()),
      },
    );
    Ok(component::SendResult {
      reply: Some(component::Parts::from([("response".to_string(), part)])),
    })
  }
  fn poll_inbound(&self, _: component::PollInbound) -> Result<component::PollInboundResult, ComponentError> {
    Ok(component::PollInboundResult { inbound: None })
  }
  fn reply(&self, _: component::Reply) -> Result<component::ReplyResult, ComponentError> {
    Ok(component::ReplyResult {})
  }
  fn dispose(&self, _: component::Dispose) -> Result<component::DisposeResult, ComponentError> {
    Ok(component::DisposeResult {})
  }
}

fn engine(answer: Value, label: &'static str) -> (Engine, Arc<Stub>, Arc<Provider>) {
  let stub = Arc::new(Stub::default());
  let provider = Arc::new(Provider {
    answer,
    label,
    sent: Mutex::new(Vec::new()),
  });
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("line".to_string(), provider.clone());
  let mut engine = Engine::with_components(transports, Some(stub.clone()));
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  (engine, stub, provider)
}

fn request_shape() -> Value {
  json!({ "shape": "object", "members": { "q": { "shape": "string", "example": "n" } } })
}

fn contract() -> Value {
  json!({ "$format": "janus-contract/1", "consumer": { "name": "c" }, "provider": { "name": "p" },
          "interactions": [ {
            "description": "a number",
            "transport": { "kind": "line", "mode": "passive" },
            "content-types": { "request": { "body": "x/query" }, "response": { "body": "x/number" } },
            "parts": { "request": { "body": request_shape() },
                       "response": { "body": { "shape": "object", "members": {
                         "n": { "shape": "integer", "example": 4 } } } } },
            "selection": { "variants": [ { "id": "base", "origin": "base", "assignment": [],
              "parts": { "request": { "body": { "content": { "q": "n" }, "content-type": "x/query" } },
                         "response": { "body": { "content": { "n": 4 }, "content-type": "x/number" } } } } ],
              "report": {} } } ] })
}

fn verify(engine: &mut Engine) -> Vec<Value> {
  let started = send(
    engine,
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [contract()] },
            "target": { "transports": [ { "transport": "line" } ] } }),
  );
  let stream = started["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("verify: {started}"))
    .to_string();
  let mut events = Vec::new();
  for _ in 0..100 {
    let polled = send(
      engine,
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    for event in polled["ok"]["events"].as_array().unwrap() {
      events.push(event.clone());
      if event["last"] == json!(true) {
        return events;
      }
    }
  }
  panic!("the run never finished");
}

fn payloads<'a>(events: &'a [Value], kind: &str) -> Vec<&'a Value> {
  events
    .iter()
    .filter(|event| event["kind"] == kind)
    .map(|event| &event["payload"])
    .collect()
}

/// Phase 9 finding 14: the component's own words, at the slot, once, and nothing about `n` that the
/// plan would have said of a guess.
#[test]
fn a_reply_the_component_refuses_fails_at_the_slot_in_its_words() {
  let (mut engine, _, _) = engine(json!("ragged"), "x/number");
  let events = verify(&mut engine);
  let results = payloads(&events, "verification/interaction-result");
  assert_eq!(results[0]["status"], "failed", "{events:?}");
  assert_eq!(
    results[0]["mismatches"],
    json!([ { "path": "$.response.body", "message": "could not be decoded as 'x/number': row 2 is ragged",
              "action": null } ])
  );
}

/// Component-interfaces spec §11.3: a component that died is not the provider's mismatch, but
/// `component-failed`, carrying the binding's error verbatim and naming the slot.
#[test]
fn a_component_that_died_decoding_the_reply_fails_the_variant_as_component_failed() {
  let (mut engine, _, _) = engine(json!("trap"), "x/number");
  let events = verify(&mut engine);
  let results = payloads(&events, "verification/interaction-result");
  let error = &results[0]["error"];
  assert_eq!(results[0]["status"], "failed");
  assert_eq!(error["code"], "component-failed", "{error}");
  assert_eq!(error["details"]["component"], "content/x/number");
  assert_eq!(error["details"]["path"], "$.response.body");
  assert_eq!(error["details"]["error"]["code"], "component-trapped");
  assert_eq!(error["details"]["error"]["source"], "engine");
  assert!(results[0].get("mismatches").is_none(), "{}", results[0]);
}

/// Spec §6.4 and Phase 9 finding 13: a decode's losses and a slot labelled otherwise than declared are
/// warnings, located in the reply, before the result, and change no verdict.
#[test]
fn what_a_decode_says_beside_the_document_is_a_warning_before_the_result() {
  let (mut engine, _, _) = engine(json!({ "n": 4 }), "x/other");
  let events = verify(&mut engine);
  let results = payloads(&events, "verification/interaction-result");
  assert_eq!(results[0]["status"], "verified", "{events:?}");
  let warnings: Vec<&Value> = payloads(&events, "verification/warning")
    .into_iter()
    .filter(|warning| warning["code"] != "state-unhandled")
    .collect();
  assert_eq!(warnings.len(), 2, "{warnings:?}");
  let differs = warnings
    .iter()
    .find(|w| w["code"] == "content-type-differs")
    .unwrap();
  assert_eq!(differs["target"], "$.response.body");
  assert_eq!(
    differs["message"],
    "the contract declares 'x/number' and the provider labelled it 'x/other'; it was decoded as 'x/number'"
  );
  let degraded = warnings.iter().find(|w| w["code"] == "content-degraded").unwrap();
  assert_eq!(degraded["target"], "$.response.body.n");
  assert_eq!(degraded["degradation"]["code"], "numeric-lexical");
  let warned = events
    .iter()
    .position(|event| event["kind"] == "verification/warning")
    .unwrap();
  let reported = events
    .iter()
    .position(|event| event["kind"] == "verification/interaction-result")
    .unwrap();
  assert!(warned < reported);
}

/// Phase 9 finding 12: a verifier encoding a declared request slot hands the component the slot's
/// shape as the contract records it.
#[test]
fn a_declared_request_slot_is_encoded_with_its_shape() {
  let (mut engine, stub, provider) = engine(json!({ "n": 4 }), "x/number");
  verify(&mut engine);
  let encoded = stub.encoded.lock().unwrap();
  assert_eq!(encoded.len(), 1, "{encoded:?}");
  assert_eq!(encoded[0].content_type, "x/query");
  assert_eq!(encoded[0].shape, Some(request_shape()));
  assert_eq!(provider.sent.lock().unwrap().len(), 1);
}
