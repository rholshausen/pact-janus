//! Staged plan S5.2e (ADR 0111): a v1–v4 interaction carried over a pact plugin's transport is one send
//! through the transport kind its bridge contributes, and the plugin's verdict, its reply, is read by the
//! bridge's `verdict` action and nothing else.
//!
//! The bridge is a stub: a component named `pact-plugin-protobuf` whose handshake describes a `grpc`
//! transport under the `pact-plugin` capability, as `janus-plugin-bridge` does, and which answers each send
//! with the verdict the test gives it. `plugin-bridge/` drives the real protobuf plugin, and S1.1's harness
//! holds the whole path to today's verifier (`corpora/verifier/plugins/grpc-area-calculator*`).

use pact_janus_kernel::component::{
  self, Apply, ApplyResult, ComponentDeclaration, ComponentError, ComponentLoader, Loaded, MatcherComponent,
  SlotValue, TransportComponent,
};
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

/// The stub bridge's transport and matcher: what each start and send was given, and the verdict it
/// answers with.
#[derive(Default)]
struct StubBridge {
  started: Mutex<Vec<component::Start>>,
  sent: Mutex<Vec<component::Parts>>,
  verdict: Mutex<Value>,
}

impl TransportComponent for StubBridge {
  fn content_slots(&self) -> component::ContentSlots {
    component::ContentSlots::new()
  }
  fn exchange_parts(&self) -> component::ExchangeParts {
    component::ExchangeParts {
      initiating: "request".to_string(),
      answering: Some("response".to_string()),
    }
  }
  fn start(&self, req: component::Start) -> Result<component::StartResult, ComponentError> {
    self.started.lock().unwrap().push(req);
    Ok(component::StartResult { endpoint: json!({}) })
  }
  fn stop(&self, _: component::Stop) -> Result<component::StopResult, ComponentError> {
    Ok(component::StopResult {})
  }
  fn send(&self, req: component::Send) -> Result<component::SendResult, ComponentError> {
    self.sent.lock().unwrap().push(req.parts);
    let mut response = component::Part::new();
    response.insert(
      "verdict".to_string(),
      SlotValue {
        content: self.verdict.lock().unwrap().clone(),
        encoded: None,
        content_type: None,
      },
    );
    Ok(component::SendResult {
      reply: Some(component::Parts::from([("response".to_string(), response)])),
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

impl MatcherComponent for StubBridge {
  /// `verdict`, as the bridge reads one: `ok`, or an error with each mismatch located `at`.
  fn apply(&self, req: Apply) -> Result<ApplyResult, ComponentError> {
    assert_eq!(req.action, "pact-plugin-protobuf:verdict");
    let results = req
      .values
      .iter()
      .map(|value| {
        let verdict = &value["content"];
        if verdict["ok"] == json!(true) {
          json!({ "status": "ok" })
        } else {
          json!({ "status": "error", "message": "the plugin failed it", "mismatches": verdict["mismatches"] })
        }
      })
      .collect();
    Ok(ApplyResult { results })
  }
}

struct StubLoader {
  bridge: Arc<StubBridge>,
}

impl ComponentLoader for StubLoader {
  fn name(&self) -> &str {
    "stub"
  }
  fn sources(&self) -> &[&str] {
    &["stub"]
  }
  fn load(&self, _: &ComponentDeclaration) -> Result<Loaded, ComponentError> {
    Ok(Loaded {
      hello: json!({
        "component-protocol-version": 1,
        "component": { "name": "pact-plugin-protobuf", "version": "0.8.1" },
        "interfaces": ["matcher", "transport"],
        "contributes": {
          "actions": [ { "name": "pact-plugin-protobuf:verdict" } ],
          "transports": [ { "kind": "grpc", "roles": ["drive"],
                            "exchange-parts": { "initiating": "request", "answering": "response" } } ]
        },
        "capabilities": { "pact-plugin": {
          "name": "protobuf", "version": "0.8.1", "interface-version": 1,
          "catalogue": [ { "type": "transport", "key": "grpc", "kind": "grpc",
                           "action": "pact-plugin-protobuf:verdict", "values": {} } ]
        } }
      }),
      content: None,
      transport: Some(self.bridge.clone()),
      matcher: Some(self.bridge.clone()),
      fetched: None,
    })
  }
}

fn engine(bridge: &Arc<StubBridge>) -> Engine {
  let mut engine = Engine::with_components(HashMap::new(), None);
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
  engine.register_component_loader(Arc::new(StubLoader {
    bridge: bridge.clone(),
  }));
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

/// pact-plugins' area calculator pact, cut down: one gRPC interaction, with a provider state.
fn grpc_pact() -> Value {
  json!({
    "consumer": { "name": "grpc-consumer" }, "provider": { "name": "area-calculator-provider" },
    "interactions": [ {
      "type": "Synchronous/Messages", "description": "calculate rectangle area request",
      "transport": "grpc",
      "providerStates": [ { "name": "a calculator", "params": { "units": "cm" } } ],
      "pluginConfiguration": { "protobuf": { "descriptorKey": "d1", "service": ".area_calculator.Calculator/calculateOne" } },
      "request": { "contents": { "content": "EgoNAABAQBUAAIBA", "contentType": "application/protobuf;message=.area_calculator.ShapeMessage", "encoded": "base64" } },
      "response": [ { "contents": { "content": "CgQAAEBB", "contentType": "application/protobuf;message=.area_calculator.AreaResponse", "encoded": "base64" } } ]
    } ],
    "metadata": {
      "pactSpecification": { "version": "4.0" },
      "plugins": [ { "name": "protobuf", "version": "0.8.1", "configuration": { "d1": { "protoFile": "syntax = \"proto3\";" } } } ]
    }
  })
}

/// Runs a verification of the gRPC pact with these transport bindings, and answers its summary and
/// interaction results.
fn verify(engine: &mut Engine, transports: Value) -> (Value, Vec<Value>) {
  let started = send(
    engine,
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [grpc_pact()] },
            "target": { "transports": transports,
                        "components": [ { "name": "pact-plugin-protobuf", "source": { "kind": "stub" } } ] } }),
  );
  let stream = started["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("{started}"))
    .to_string();
  let mut results = Vec::new();
  for _ in 0..100 {
    let polled = send(
      engine,
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    for event in polled["ok"]["events"].as_array().unwrap() {
      if event["kind"] == "verification/interaction-result" {
        results.push(event["payload"].clone());
      }
      if event["last"] == json!(true) {
        return (event["payload"].clone(), results);
      }
    }
  }
  panic!("the run does not finish");
}

/// The provider's binding. This engine has no HTTP transport, so it is optional, and held: its options
/// are still where a plugin transport with no binding of its own is reached.
fn provider() -> Value {
  json!({ "transport": "http", "optional": true,
          "options": { "base-url": "http://127.0.0.1:8222", "headers": { "x-trace": "1" } } })
}

#[test]
fn an_interaction_over_a_plugin_transport_is_one_send_reached_at_the_provider() {
  let bridge = Arc::new(StubBridge::default());
  *bridge.verdict.lock().unwrap() = json!({ "ok": true, "mismatches": [], "errors": [], "output": [] });
  let mut engine = engine(&bridge);
  let (summary, results) = verify(&mut engine, json!([provider()]));
  assert_eq!(summary["status"], "verified", "{summary}");
  assert_eq!(results.len(), 1);

  // With no binding of its own, started with the provider's options, as today's verifier reaches it.
  let started = bridge.started.lock().unwrap();
  assert_eq!(started.len(), 1);
  assert_eq!(started[0].kind, "grpc");
  assert_eq!(started[0].role, "drive");
  assert_eq!(started[0].options, Some(provider()["options"].clone()));

  // One send: the pact holding the interaction alone, its key, and the provider-state values.
  let sent = bridge.sent.lock().unwrap();
  assert_eq!(sent.len(), 1);
  let request = &sent[0]["request"];
  let pact = &request["pact"].content;
  let key = request["key"].content.as_str().expect("a key");
  assert_eq!(pact["interactions"].as_array().map(Vec::len), Some(1));
  assert_eq!(pact["interactions"][0]["key"], key);
  assert_eq!(
    pact["metadata"]["plugins"][0]["configuration"]["d1"]["protoFile"],
    "syntax = \"proto3\";"
  );
  assert_eq!(request["provider-state"].content, json!({ "units": "cm" }));
}

#[test]
fn the_plugins_verdict_fails_the_interaction_with_each_mismatch_it_found() {
  let bridge = Arc::new(StubBridge::default());
  *bridge.verdict.lock().unwrap() = json!({
    "ok": false,
    "mismatches": [
      { "message": "Expected message 'AreaResponse' but was missing or empty", "at": "$" },
      { "message": "Expected 12.0 but got 0.0", "at": "$.value[0]" }
    ],
    "errors": [], "output": []
  });
  let mut engine = engine(&bridge);
  let (summary, results) = verify(&mut engine, json!([provider()]));
  assert_eq!(summary["status"], "failed", "{summary}");
  let mismatches = results[0]["mismatches"].as_array().expect("mismatches");
  assert_eq!(mismatches.len(), 2, "{}", results[0]);
  assert_eq!(mismatches[0]["path"], "$.response.verdict");
  assert_eq!(mismatches[0]["at"], "$");
  assert_eq!(mismatches[1]["at"], "$.value[0]");
}

#[test]
fn an_optional_binding_of_the_plugin_transports_kind_says_where_it_is_reached() {
  let bridge = Arc::new(StubBridge::default());
  *bridge.verdict.lock().unwrap() = json!({ "ok": true });
  let mut engine = engine(&bridge);
  let grpc =
    json!({ "transport": "grpc", "optional": true, "options": { "base-url": "http://127.0.0.1:5555" } });
  let (summary, _) = verify(&mut engine, json!([provider(), grpc]));
  assert_eq!(summary["status"], "verified", "{summary}");
  let started = bridge.started.lock().unwrap();
  assert_eq!(started.len(), 1);
  assert_eq!(
    started[0].options,
    Some(json!({ "base-url": "http://127.0.0.1:5555" }))
  );
}

#[test]
fn an_optional_binding_no_component_provides_is_no_error_and_a_required_one_is() {
  let bridge = Arc::new(StubBridge::default());
  *bridge.verdict.lock().unwrap() = json!({ "ok": true });
  let mut engine = engine(&bridge);
  let kafka = |optional: bool| json!({ "transport": "kafka", "optional": optional });
  let (summary, _) = verify(&mut engine, json!([provider(), kafka(true)]));
  assert_eq!(summary["status"], "verified", "{summary}");

  let refused = send(
    &mut engine,
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [grpc_pact()] },
            "target": { "transports": [provider(), kafka(false)],
                        "components": [ { "name": "pact-plugin-protobuf", "source": { "kind": "stub" } } ] } }),
  );
  assert_eq!(refused["error"]["code"], "component-unavailable", "{refused}");
  assert_eq!(refused["error"]["details"]["component"], "transport/kafka");
}
