//! Plan task 8.4: component-contributed plan fragments against the versioning policy (plan-grammar
//! spec §7.1, component-interfaces spec §6.3, §12.3). A stub content component, loaded by a stub
//! loader, answers `content/compile` with whatever fragment a test gives it; `add-interaction` is
//! where each one is accepted or refused, naming why. The real contributor — the CSV component, over
//! the WASM binding — is `engine/component-host/tests/csv_component.rs`.

use pact_janus_kernel::component::{
  self, Apply, ApplyResult, Compile, CompileResult, ComponentDeclaration, ComponentError, ComponentLoader,
  ContentComponent, Decode, DecodeResult, Detect, DetectResult, Encode, EncodeResult, Loaded,
  MatcherComponent, TransportComponent,
};
use pact_janus_kernel::plan::RuntimeValue;
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

/// Content for `x/*`: decodes a slot's JSON content as the document, and answers `compile` with the
/// fragment and grammar version it was given. `stub:even` is its one action.
struct Stub {
  fragment: Option<Value>,
  grammar: Option<String>,
  applied: Mutex<Vec<Apply>>,
}

impl ContentComponent for Stub {
  fn handles(&self, content_type: &str) -> bool {
    content_type.starts_with("x/")
  }
  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    Ok(DecodeResult {
      document: RuntimeValue::from_json(&req.value.content),
      degradations: Vec::new(),
    })
  }
  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    Ok(EncodeResult {
      value: component::SlotValue {
        content: req.document.to_json(),
        encoded: None,
        content_type: Some(req.content_type),
      },
    })
  }
  fn compile(&self, _: Compile) -> Result<CompileResult, ComponentError> {
    Ok(CompileResult {
      fragment: self.fragment.clone(),
      grammar_version: self.grammar.clone(),
    })
  }
  fn detect(&self, _: Detect) -> Result<DetectResult, ComponentError> {
    Ok(DetectResult {
      media_type: None,
      confidence: None,
    })
  }
}

impl MatcherComponent for Stub {
  fn apply(&self, req: Apply) -> Result<ApplyResult, ComponentError> {
    let even = req.values[0]["content"].as_i64().is_some_and(|n| n % 2 == 0);
    self.applied.lock().unwrap().push(req);
    Ok(ApplyResult {
      results: vec![if even {
        json!({ "status": "ok" })
      } else {
        json!({ "status": "error", "message": "odd" })
      }],
    })
  }
}

struct StubLoader {
  stub: Arc<Stub>,
  name: String,
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
      hello: json!({ "component-protocol-version": 1, "component": { "name": self.name, "version": "1.0.0" },
                     "interfaces": ["content", "matcher"],
                     "contributes": { "content-types": [ { "media-type": "x/*" } ],
                                      "actions": [ { "name": format!("{}:even", self.name) } ] } }),
      content: Some(self.stub.clone()),
      transport: None,
      matcher: Some(self.stub.clone()),
    })
  }
}

/// An in-tree `line` transport whose `send` answers with `answer` as the response body.
struct Provider {
  answer: Value,
}

impl TransportComponent for Provider {
  fn content_slots(&self) -> component::ContentSlots {
    component::ContentSlots::from([("response".to_string(), vec!["body".to_string()])])
  }
  fn start(&self, _: component::Start) -> Result<component::StartResult, ComponentError> {
    Ok(component::StartResult { endpoint: json!({}) })
  }
  fn stop(&self, _: component::Stop) -> Result<component::StopResult, ComponentError> {
    Ok(component::StopResult {})
  }
  fn send(&self, _: component::Send) -> Result<component::SendResult, ComponentError> {
    let mut part = component::Part::new();
    part.insert(
      "body".to_string(),
      component::SlotValue {
        content: self.answer.clone(),
        encoded: None,
        content_type: Some("x/number".to_string()),
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

fn engine(stub: &Arc<Stub>, name: &str, answer: Value) -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("line".to_string(), Arc::new(Provider { answer }));
  let mut engine = Engine::with_components(transports, None);
  engine.register_component_loader(Arc::new(StubLoader {
    stub: stub.clone(),
    name: name.to_string(),
  }));
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

fn stub(fragment: Option<Value>, grammar: Option<&str>) -> Arc<Stub> {
  Arc::new(Stub {
    fragment,
    grammar: grammar.map(str::to_string),
    applied: Mutex::new(Vec::new()),
  })
}

fn interaction() -> Value {
  json!({ "description": "a number",
          "transport": { "kind": "line", "mode": "passive" },
          "content-types": { "response": { "body": "x/number" } },
          "parts": { "request": { "line": { "shape": "equality", "example": "N?" } },
                     "response": { "body": { "shape": "integer", "example": 4 } } } })
}

fn add(engine: &mut Engine, name: &str) -> Value {
  let created = send(
    engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" },
                        "components": [ { "name": name, "source": { "kind": "stub" } } ] } }),
  );
  if created.get("error").is_some() {
    return created;
  }
  send(
    engine,
    "consumer-session/add-interaction",
    json!({ "session": created["ok"]["session"], "interaction": interaction() }),
  )
}

fn action(name: &str, path: &str) -> Value {
  json!({ "kind": "action", "name": name, "children": [ { "kind": "resolve", "path": path } ] })
}

#[test]
fn a_fragment_that_cannot_be_used_fails_the_interaction_naming_why() {
  let body = "$.response.body";
  let cases = [
    // (fragment, declared grammar, reason, words the message must carry)
    (
      action("expect:not-empty", body),
      None,
      "grammar-skew",
      "does not say which plan grammar",
    ),
    (
      action("expect:not-empty", body),
      Some("v0.1"),
      "grammar-skew",
      "'v0.1'",
    ),
    (
      action("expect:not-empty", body),
      Some("v1"),
      "grammar-skew",
      "'v1'",
    ),
    (
      action("expect:unique", body),
      Some("v0"),
      "fragment-invalid",
      "not a core action",
    ),
    (
      action("csv:integer", body),
      Some("v0"),
      "fragment-invalid",
      "another component's namespace",
    ),
    (
      action("stub:odd", body),
      Some("v0"),
      "fragment-invalid",
      "not among the actions",
    ),
    (
      action("expect:not-empty", "$.request.line"),
      Some("v0"),
      "fragment-invalid",
      "outside the slot",
    ),
    (
      json!({ "kind": "widget" }),
      Some("v0"),
      "fragment-invalid",
      "unknown plan node kind",
    ),
  ];
  for (fragment, grammar, reason, words) in cases {
    let stub = stub(Some(fragment.clone()), grammar);
    let mut engine = engine(&stub, "stub", json!(4));
    let refused = add(&mut engine, "stub");
    assert_eq!(
      refused["error"]["code"], "component-unavailable",
      "{fragment}: {refused}"
    );
    assert_eq!(
      refused["error"]["details"]["reason"], reason,
      "{fragment}: {refused}"
    );
    let message = refused["error"]["message"].as_str().unwrap();
    assert!(message.contains(words), "{fragment}: {message}");
    assert_eq!(refused["error"]["details"]["component"], "stub");
  }
}

#[test]
fn a_component_named_for_a_core_family_is_refused_at_load() {
  let stub = stub(None, None);
  for family in ["match", "expect", "check", "convert"] {
    let mut engine = engine(&stub, family, json!(4));
    let refused = add(&mut engine, family);
    assert_eq!(refused["error"]["code"], "component-unavailable", "{refused}");
    assert_eq!(
      refused["error"]["details"]["reason"], "component-invalid",
      "{refused}"
    );
  }
}

#[test]
fn no_fragment_is_the_generic_plan_and_that_is_fine() {
  let stub = stub(None, None);
  let mut engine = engine(&stub, "stub", json!(4));
  assert!(add(&mut engine, "stub")["ok"]["handle"].is_string());
}

/// A contract whose one variant's response is `{body: 4}`, as a consumer run would record it.
fn contract() -> Value {
  let mut interaction = interaction();
  interaction["selection"] = json!({ "variants": [ { "id": "base", "origin": "base", "assignment": [],
    "parts": { "request": { "line": { "content": "N?" } },
               "response": { "body": { "content": 4, "content-type": "x/number" } } } } ], "report": {} });
  json!({ "$format": "janus-contract/1", "consumer": { "name": "c" }, "provider": { "name": "p" },
          "interactions": [ interaction ] })
}

fn verify(engine: &mut Engine, name: &str) -> Value {
  let started = send(
    engine,
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [contract()] },
            "target": { "transports": [ { "transport": "line" } ],
                        "components": [ { "name": name, "source": { "kind": "stub" } } ] },
            "options": { "executed-plan": "always" } }),
  );
  let Some(stream) = started["ok"]["stream"].as_str().map(str::to_string) else {
    return started;
  };
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
        return json!(events);
      }
    }
  }
  panic!("the run never finished");
}

#[test]
fn a_readable_fragment_replaces_the_slots_plan_and_its_action_runs_through_apply() {
  // The generic plan would say `match:integer`; this component says "even", in its own action.
  let fragment =
    json!({ "kind": "container", "label": "stub", "children": [ action("stub:even", "$.response.body") ] });
  for (answer, status) in [(json!(4), "verified"), (json!(3), "failed")] {
    let stub = stub(Some(fragment.clone()), Some("v0"));
    let mut engine = engine(&stub, "stub", answer.clone());
    let events = verify(&mut engine, "stub");
    let summary = events.as_array().unwrap().last().unwrap()["payload"].clone();
    assert_eq!(summary["status"], status, "{answer}: {events}");
    let applied = stub.applied.lock().unwrap();
    assert_eq!(applied.len(), 1, "one application per variant");
    assert_eq!(applied[0].action, "stub:even");
    assert_eq!(
      applied[0].values,
      vec![json!({ "content": answer, "path": "$.response.body" })],
      "a MatchValue: content, and the path it was resolved from"
    );
    if status == "failed" {
      let text = events.to_string();
      assert!(
        text.contains("stub:even"),
        "the executed plan names the component's action: {text}"
      );
      assert!(
        !text.contains("match:integer"),
        "the generic check was replaced, not joined: {text}"
      );
    }
  }
}

#[test]
fn a_verification_whose_fragment_cannot_be_used_fails_before_it_starts() {
  let stub = stub(Some(action("expect:not-empty", "$.response.body")), Some("v0.1"));
  let mut engine = engine(&stub, "stub", json!(4));
  let refused = verify(&mut engine, "stub");
  assert_eq!(refused["error"]["code"], "component-unavailable", "{refused}");
  assert_eq!(refused["error"]["details"]["reason"], "grammar-skew", "{refused}");
}
