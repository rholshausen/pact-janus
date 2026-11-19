//! Staged plan S5.4, ADR 0100: a content component says which of its actions stands for a value
//! operator under its content type, and the engine compiles every slot itself (component-interfaces
//! spec §6.3). A stub content component, loaded by a stub loader, declares whatever substitutions a
//! test gives it in its handshake; the load is where each is accepted or refused, naming why. The real
//! substituter — the CSV component, over the WASM binding — is
//! `engine/component-host/tests/csv_component.rs`.

use pact_janus_kernel::component::{
  self, Apply, ApplyResult, Compile, CompileResult, ComponentDeclaration, ComponentError, ComponentLoader,
  ContentComponent, Decode, DecodeResult, Detect, DetectResult, Encode, EncodeResult, Loaded,
  MatcherComponent, TransportComponent,
};
use pact_janus_kernel::plan::{self, Assignment, RuntimeValue, SlotSubstitutions, Substitution};
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

/// Content for `x/*`: decodes a slot's JSON content as the document. `<name>:even` is its one action,
/// and holds of an even number; `refuse` makes `compile` refuse every shape.
struct Stub {
  refuse: bool,
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
  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError> {
    if self.refuse {
      return Err(ComponentError::encode_failed(
        &req.content_type,
        "this stub carries no shape",
      ));
    }
    Ok(CompileResult::default())
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
  /// The `content-types` entries its handshake declares.
  content_types: Value,
  interfaces: Value,
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
                     "interfaces": self.interfaces,
                     "contributes": { "content-types": self.content_types,
                                      "actions": [ { "name": format!("{}:even", self.name) } ] } }),
      content: Some(self.stub.clone()),
      transport: None,
      matcher: Some(self.stub.clone()),
      fetched: None,
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

/// `integer` under every `x/*` type is `<name>:even`.
fn substituting(name: &str) -> Value {
  json!([ { "media-type": "x/*", "substitutions": { "integer": format!("{name}:even") } } ])
}

struct Setup {
  name: &'static str,
  content_types: Value,
  interfaces: Value,
  refuse: bool,
}

impl Setup {
  fn named(name: &'static str) -> Setup {
    Setup {
      name,
      content_types: substituting(name),
      interfaces: json!(["content", "matcher"]),
      refuse: false,
    }
  }
}

fn engine_with(setup: &Setup, answer: Value) -> (Engine, Arc<Stub>) {
  let stub = Arc::new(Stub {
    refuse: setup.refuse,
    applied: Mutex::new(Vec::new()),
  });
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("line".to_string(), Arc::new(Provider { answer }));
  let mut engine = Engine::with_components(transports, None);
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
  engine.register_component_loader(Arc::new(StubLoader {
    stub: stub.clone(),
    name: setup.name.to_string(),
    content_types: setup.content_types.clone(),
    interfaces: setup.interfaces.clone(),
  }));
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  (engine, stub)
}

fn interaction_with(body: Value) -> Value {
  json!({ "description": "a number",
          "transport": { "kind": "line", "mode": "passive" },
          "content-types": { "response": { "body": "x/number" } },
          "parts": { "request": { "line": { "shape": "equality", "example": "N?" } },
                     "response": { "body": body } } })
}

fn interaction() -> Value {
  interaction_with(json!({ "shape": "integer", "example": 4 }))
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

/// The plan `verification/explain` prints for `spec`, with the stub declared.
fn explained(engine: &mut Engine, name: &str, spec: Value) -> String {
  let explained = send(
    engine,
    "verification/explain",
    json!({ "interaction": { "kind": "spec", "spec": spec },
            "components": [ { "name": name, "source": { "kind": "stub" } } ] }),
  );
  explained["ok"]["text"]
    .as_str()
    .unwrap_or_else(|| panic!("explain: {explained}"))
    .to_string()
}

#[test]
fn a_substitution_the_engine_cannot_honour_fails_the_load_naming_why() {
  let cases = [
    // (content-types, interfaces, words the message must carry)
    (
      json!([ { "media-type": "x/*", "substitutions": { "integer": "stub:odd" } } ]),
      json!(["content", "matcher"]),
      "does not contribute that action",
    ),
    (
      json!([ { "media-type": "x/*", "substitutions": { "integer": "csv:integer" } } ]),
      json!(["content", "matcher"]),
      "does not contribute that action",
    ),
    (
      substituting("stub"),
      json!(["content"]),
      "does not implement 'matcher'",
    ),
    (
      json!([ { "media-type": "x/*", "substitutions": { "each-like": "stub:even" } } ]),
      json!(["content", "matcher"]),
      "only a value operator can be substituted",
    ),
    (
      json!([ { "media-type": "x/*", "substitutions": { "content-type": "stub:even" } } ]),
      json!(["content", "matcher"]),
      "only a value operator can be substituted",
    ),
    (
      json!([ { "media-type": "x/*", "substitutions": { "optional": "stub:even" } } ]),
      json!(["content", "matcher"]),
      "only a value operator can be substituted",
    ),
    (
      json!([ { "media-type": "x/*", "substitutions": { "integer": ["stub:even"] } } ]),
      json!(["content", "matcher"]),
      "a substitution names one of its actions",
    ),
  ];
  for (content_types, interfaces, words) in cases {
    let setup = Setup {
      content_types: content_types.clone(),
      interfaces,
      ..Setup::named("stub")
    };
    let (mut engine, _) = engine_with(&setup, json!(4));
    let refused = add(&mut engine, "stub");
    assert_eq!(
      refused["error"]["code"], "component-unavailable",
      "{content_types}: {refused}"
    );
    assert_eq!(
      refused["error"]["details"]["reason"], "component-invalid",
      "{content_types}: {refused}"
    );
    assert_eq!(refused["error"]["details"]["component"], "stub");
    let message = refused["error"]["message"].as_str().unwrap();
    assert!(message.contains(words), "{content_types}: {message}");
  }
}

/// An operator name the shape language does not have is a newer shape language's, perhaps: the
/// component still loads, on an engine that will never compile one.
#[test]
fn an_operator_this_engine_does_not_have_is_passed_over() {
  let setup = Setup {
    content_types: json!([ { "media-type": "x/*",
                             "substitutions": { "integer": "stub:even", "widget": "stub:even" } } ]),
    ..Setup::named("stub")
  };
  let (mut engine, _) = engine_with(&setup, json!(4));
  assert!(add(&mut engine, "stub")["ok"]["handle"].is_string());
}

/// Plan-grammar spec §4.1, ADR 0026 decision 2: the six core namespaces are the specification's, and
/// a component's actions are where a plan shows them.
#[test]
fn a_component_declaring_a_reserved_namespace_is_refused_at_load_by_name() {
  assert_eq!(
    pact_janus_kernel::plan::RESERVED_NAMESPACES,
    ["flow", "value", "expect", "check", "match", "legacy"]
  );
  for namespace in pact_janus_kernel::plan::RESERVED_NAMESPACES {
    let (mut engine, _) = engine_with(&Setup::named(namespace), json!(4));
    let refused = add(&mut engine, namespace);
    assert_eq!(refused["error"]["code"], "component-unavailable", "{refused}");
    assert_eq!(
      refused["error"]["details"]["reason"], "component-invalid",
      "{refused}"
    );
    assert_eq!(refused["error"]["details"]["component"], *namespace, "{refused}");
    let message = refused["error"]["message"].as_str().unwrap();
    assert!(
      message.contains(&format!(
        "'{namespace}' is a namespace reserved for core plan actions"
      )),
      "{message}"
    );
  }
}

/// v0 reserved `convert` as a family that never had an action. v1 reserves six namespaces and that
/// is not one of them (ADR 0026: further ones need a new major).
#[test]
fn a_namespace_v1_does_not_reserve_is_a_components_to_take() {
  let (mut engine, _) = engine_with(&Setup::named("convert"), json!(4));
  assert!(add(&mut engine, "convert")["ok"]["handle"].is_string());
}

/// `content/compile` is still how a component refuses a shape it cannot carry, when the interaction
/// arrives (spec §6.3).
#[test]
fn a_component_that_refuses_a_slots_shape_fails_the_interaction_as_compile_failed() {
  let setup = Setup {
    refuse: true,
    ..Setup::named("stub")
  };
  let (mut engine, _) = engine_with(&setup, json!(4));
  let refused = add(&mut engine, "stub");
  assert_eq!(refused["error"]["code"], "component-unavailable", "{refused}");
  assert_eq!(
    refused["error"]["details"]["reason"], "compile-failed",
    "{refused}"
  );
  assert_eq!(refused["error"]["details"]["component"], "stub", "{refused}");
  let message = refused["error"]["message"].as_str().unwrap();
  assert!(message.contains("this stub carries no shape"), "{message}");
}

/// What `explain` prints: the engine's own structure, the component's action where `integer` would
/// be `match:integer`, and a note at the top of the slot saying so.
#[test]
fn explain_shows_the_substituted_action_and_says_why() {
  let (mut engine, _) = engine_with(&Setup::named("stub"), json!(4));
  let text = explained(
    &mut engine,
    "stub",
    interaction_with(json!({ "shape": "object", "members": {
      "count": { "shape": "integer", "example": 4 },
      "label": { "shape": "string", "example": "four" } } })),
  );
  assert!(text.contains("%stub:even"), "{text}");
  assert!(!text.contains("match:integer"), "{text}");
  assert!(
    text.contains("%match:string"),
    "an operator it does not substitute is the engine's: {text}"
  );
  assert!(
    text.contains("%expect:object"),
    "structure is the engine's: {text}"
  );
  assert!(
    text.contains("decoded as x/number by stub: integer is stub:even"),
    "{text}"
  );
}

/// A substitution names the media type it is for: a slot of another type the component decodes is
/// compiled with the core actions.
#[test]
fn a_slot_whose_type_no_entry_names_has_no_substitution() {
  let setup = Setup {
    content_types: json!([ { "media-type": "x/number", "substitutions": { "integer": "stub:even" } },
                           { "media-type": "x/*" } ]),
    ..Setup::named("stub")
  };
  let (mut engine, _) = engine_with(&setup, json!(4));
  let mut other = interaction();
  other["content-types"]["response"]["body"] = json!("x/other");
  let text = explained(&mut engine, "stub", other);
  assert!(text.contains("%match:integer"), "{text}");
  assert!(
    !text.contains("stub:even") && !text.contains("decoded as"),
    "{text}"
  );
  assert!(explained(&mut engine, "stub", interaction()).contains("%stub:even"));
}

/// A contract whose one variant's response is `{body: <answer>}`, as a consumer run would record it.
fn contract(body: Value, answer: &Value) -> Value {
  let mut interaction = interaction_with(body);
  interaction["selection"] = json!({ "variants": [ { "id": "base", "origin": "base", "assignment": [],
    "parts": { "request": { "line": { "content": "N?" } },
               "response": { "body": { "content": answer, "content-type": "x/number" } } } } ], "report": {} });
  json!({ "$format": "janus-contract/1", "consumer": { "name": "c" }, "provider": { "name": "p" },
          "interactions": [ interaction ] })
}

fn verify(engine: &mut Engine, name: &str, contract: Value) -> Value {
  let started = send(
    engine,
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [contract] },
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
fn a_substituted_operator_is_checked_by_the_components_action_through_apply() {
  // The core action would say `match:integer`, which both answers pass; this component says "even".
  for (answer, status) in [(json!(4), "verified"), (json!(3), "failed")] {
    let (mut engine, stub) = engine_with(&Setup::named("stub"), answer.clone());
    let events = verify(
      &mut engine,
      "stub",
      contract(json!({ "shape": "integer", "example": 4 }), &answer),
    );
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
    assert_eq!(applied[0].config, None, "integer takes no argument");
    let text = events.to_string();
    assert!(
      text.contains("stub:even"),
      "the executed plan names the component's action: {text}"
    );
    assert!(!text.contains("match:integer"), "{text}");
  }
}

/// ADR 0100 decision 4: the substituted action gets the core action's arguments, as
/// `config.arguments`, after the value under test.
#[test]
fn a_substituted_action_is_given_the_core_actions_arguments() {
  let setup = Setup {
    content_types: json!([ { "media-type": "x/*",
                             "substitutions": { "equality": "stub:even", "regex": "stub:even" } } ]),
    ..Setup::named("stub")
  };
  for (body, argument) in [
    (json!({ "shape": "equality", "example": 4 }), json!(4)),
    (
      json!({ "shape": "regex", "pattern": "^[0-9]+$", "example": "4" }),
      json!("^[0-9]+$"),
    ),
  ] {
    let (mut engine, stub) = engine_with(&setup, json!(4));
    let events = verify(&mut engine, "stub", contract(body.clone(), &json!(4)));
    let summary = events.as_array().unwrap().last().unwrap()["payload"].clone();
    assert_eq!(summary["status"], "verified", "{body}: {events}");
    let applied = stub.applied.lock().unwrap();
    assert_eq!(
      applied[0].config,
      Some(json!({ "arguments": [argument] })),
      "{body}"
    );
  }
}

/// Phase 9 finding 21, closed. The engine compiles a substituted slot as it compiles any other, so a
/// variant that pins a cardinality is checked at that cardinality, and a variant that pins an
/// `any-of` to one literal is checked as `equality`, by that operator's substitution.
#[test]
fn a_substituted_slot_is_pinned_to_its_variant() {
  let spec = pact_janus_kernel::interaction_spec::parse(&interaction_with(json!({
    "shape": "each-like", "min": 1, "max": 3,
    "items": { "shape": "object", "members": {
      "count": { "shape": "integer", "example": 4 },
      "size": { "shape": "any-of", "options": [2, 4] } } } })))
  .unwrap();
  let substitutions = SlotSubstitutions::from([(
    ("response".to_string(), "body".to_string()),
    Substitution {
      component: "stub".to_string(),
      content_type: "x/number".to_string(),
      operators: BTreeMap::from([
        ("integer".to_string(), "stub:even".to_string()),
        ("equality".to_string(), "stub:same".to_string()),
        ("any-of".to_string(), "stub:one-of".to_string()),
      ]),
    },
  )]);
  let pretty = |assignment: &Assignment| {
    plan::render_pretty(&plan::compile_substituted(
      &spec,
      assignment,
      None,
      &substitutions,
    ))
  };

  let unpinned = pretty(&Assignment::new());
  assert!(unpinned.contains("%expect:size"), "{unpinned}");
  assert!(unpinned.contains("%stub:one-of"), "{unpinned}");
  assert!(unpinned.contains("%stub:even"), "{unpinned}");

  let pinned = pretty(&Assignment::from([
    ("response.body#cardinality".to_string(), "max".to_string()),
    ("response.body[*].size#value".to_string(), "4".to_string()),
  ]));
  assert!(
    pinned.contains("%expect:count") && pinned.contains("3"),
    "the maximal variant is checked at three: {pinned}"
  );
  assert!(!pinned.contains("%expect:size"), "{pinned}");
  assert!(
    pinned.contains("%stub:same") && !pinned.contains("%stub:one-of"),
    "a pinned any-of is an equality: {pinned}"
  );
  assert!(pinned.contains("%stub:even"), "{pinned}");
}
