//! Plan task 8.3's kernel half: a transport a project *declares* — contributed by a loaded
//! component, not compiled into the embedding — is started by the kind its handshake contributed,
//! armed for a passive interaction of that kind, and driven by a verification. The kind here is
//! deliberately not `http`: until 8.3 the kernel armed only `"http"` transports, so any other kind
//! would have served nothing (kernel-boundary review, finding 7).
//!
//! The component is a stub loaded by a stub loader; the real out-of-process one, a Node `tcp`
//! transport over the subprocess binding, is `spikes/8.3-subprocess-transport`.

use pact_janus_kernel::component::{
  self, ComponentDeclaration, ComponentError, ComponentLoader, Loaded, SlotValue, TransportComponent,
};
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

fn slot(value: Value) -> SlotValue {
  SlotValue {
    content: value,
    encoded: None,
    content_type: None,
  }
}

fn part(slots: &[(&str, Value)]) -> component::Parts {
  let mut part = component::Part::new();
  for (name, value) in slots {
    part.insert(name.to_string(), slot(value.clone()));
  }
  part_named("request", part)
}

fn part_named(name: &str, part: component::Part) -> component::Parts {
  let mut parts = component::Parts::new();
  parts.insert(name.to_string(), part);
  parts
}

/// A `line` transport held in memory: one arrival on request, replies recorded; `send` answers
/// with whatever `answer` holds.
#[derive(Default)]
struct LineTransport {
  arrival: Mutex<Option<String>>,
  replies: Mutex<Vec<component::Parts>>,
  answer: Mutex<String>,
}

impl TransportComponent for LineTransport {
  fn content_slots(&self) -> component::ContentSlots {
    component::ContentSlots::new()
  }
  fn start(&self, req: component::Start) -> Result<component::StartResult, ComponentError> {
    Ok(component::StartResult {
      endpoint: json!({ "kind": req.kind, "role": req.role }),
    })
  }
  fn stop(&self, _: component::Stop) -> Result<component::StopResult, ComponentError> {
    Ok(component::StopResult {})
  }
  fn send(&self, _: component::Send) -> Result<component::SendResult, ComponentError> {
    let mut response = component::Part::new();
    response.insert(
      "line".to_string(),
      slot(json!(self.answer.lock().unwrap().clone())),
    );
    Ok(component::SendResult {
      reply: Some(part_named("response", response)),
    })
  }
  fn poll_inbound(
    &self,
    req: component::PollInbound,
  ) -> Result<component::PollInboundResult, ComponentError> {
    let Some(line) = self.arrival.lock().unwrap().take() else {
      std::thread::sleep(Duration::from_millis(req.timeout_ms.min(10)));
      return Ok(component::PollInboundResult { inbound: None });
    };
    Ok(component::PollInboundResult {
      inbound: Some(component::Inbound {
        event: "e-1".to_string(),
        parts: part(&[("line", json!(line))]),
        expects_reply: true,
      }),
    })
  }
  fn reply(&self, req: component::Reply) -> Result<component::ReplyResult, ComponentError> {
    self.replies.lock().unwrap().push(req.parts);
    Ok(component::ReplyResult {})
  }
  fn dispose(&self, _: component::Dispose) -> Result<component::DisposeResult, ComponentError> {
    Ok(component::DisposeResult {})
  }
}

/// Loads `stub` sources: the one transport above, under whatever hello the test gives it.
struct StubLoader {
  transport: Arc<LineTransport>,
  hello: Value,
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
      hello: self.hello.clone(),
      content: None,
      transport: Some(self.transport.clone()),
      matcher: None,
    })
  }
}

fn line_hello() -> Value {
  json!({ "component-protocol-version": 1, "component": { "name": "line", "version": "1.0.0" },
          "interfaces": ["transport"], "contributes": { "transports": [ { "kind": "line", "roles": ["serve", "drive"] } ] } })
}

fn engine(transport: &Arc<LineTransport>, hello: Value) -> Engine {
  // No transport of the embedding's own: `line` exists only because the project declared it.
  let mut engine = Engine::with_components(HashMap::new(), None);
  engine.register_component_loader(Arc::new(StubLoader {
    transport: transport.clone(),
    hello,
  }));
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

fn declared() -> Value {
  json!([ { "name": "line", "source": { "kind": "stub" } } ])
}

fn ping() -> Value {
  json!({
    "description": "ping, over a line",
    "transport": { "kind": "line", "mode": "passive" },
    "parts": { "request": { "line": { "shape": "equality", "example": "PING" } },
               "response": { "line": { "shape": "equality", "example": "PONG" } } }
  })
}

#[test]
fn a_declared_transport_of_another_kind_serves_a_passive_interaction() {
  let transport = Arc::new(LineTransport::default());
  let mut engine = engine(&transport, line_hello());
  let session = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" }, "components": declared() } }),
  )["ok"]["session"]
    .clone();
  let handle = send(
    &mut engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": ping() }),
  )["ok"]["handle"]
    .clone();
  let variant = send(
    &mut engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  )["ok"]["variants"][0]["id"]
    .clone();
  let started = send(
    &mut engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "line" }),
  );
  assert_eq!(
    started["ok"]["endpoint"],
    json!({ "kind": "line", "role": "serve" }),
    "{started}"
  );
  send(
    &mut engine,
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": variant }),
  );

  *transport.arrival.lock().unwrap() = Some("PING".to_string());
  let waited = Instant::now();
  while transport.replies.lock().unwrap().is_empty() {
    assert!(
      waited.elapsed() < Duration::from_secs(5),
      "never armed: nothing replied"
    );
    std::thread::sleep(Duration::from_millis(5));
  }
  assert_eq!(
    transport.replies.lock().unwrap()[0]["response"]["line"].content,
    json!("PONG")
  );
  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(finalised["ok"]["results"][0]["status"], "verified", "{finalised}");
}

#[test]
fn a_declared_transport_drives_a_verification() {
  let transport = Arc::new(LineTransport::default());
  let mut engine = engine(&transport, line_hello());
  let contract = json!({ "$format": "janus-contract/1",
    "consumer": { "name": "c" }, "provider": { "name": "p" },
    "interactions": [ { "description": "ping, over a line",
      "transport": { "kind": "line", "mode": "passive" },
      "parts": ping()["parts"],
      "selection": { "variants": [ { "id": "base", "origin": "base", "assignment": [],
        "parts": { "request": { "line": { "content": "PING" } }, "response": { "line": { "content": "PONG" } } } } ],
        "report": {} } } ] });
  for (answer, status) in [("PONG", "verified"), ("PANG", "failed")] {
    *transport.answer.lock().unwrap() = answer.to_string();
    let started = send(
      &mut engine,
      "verification/verify",
      json!({ "source": { "kind": "inline", "contracts": [contract] },
              "target": { "transports": [ { "transport": "line" } ], "components": declared() } }),
    );
    let stream = started["ok"]["stream"]
      .as_str()
      .unwrap_or_else(|| panic!("{started}"))
      .to_string();
    let summary = (0..100)
      .find_map(|_| {
        let polled = send(
          &mut engine,
          "events/poll",
          json!({ "streams": [stream], "wait-ms": 5_000 }),
        );
        polled["ok"]["events"]
          .as_array()
          .unwrap()
          .iter()
          .find(|event| event["last"] == json!(true))
          .map(|event| event["payload"].clone())
      })
      .expect("the run finishes");
    assert_eq!(summary["status"], status, "{answer}: {summary}");
  }
}

#[test]
fn a_kind_no_declared_component_contributes_is_unavailable() {
  let transport = Arc::new(LineTransport::default());
  let mut engine = engine(&transport, line_hello());
  let session = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" }, "components": declared() } }),
  )["ok"]["session"]
    .clone();
  let refused = send(
    &mut engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "tcp" }),
  );
  assert_eq!(refused["error"]["code"], "component-unavailable", "{refused}");
}

#[test]
fn a_transport_that_contributes_no_kind_is_invalid_at_load() {
  let transport = Arc::new(LineTransport::default());
  let mut hello = line_hello();
  hello["contributes"] = json!({});
  let mut engine = engine(&transport, hello);
  let refused = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" }, "components": declared() } }),
  );
  assert_eq!(refused["error"]["code"], "component-unavailable", "{refused}");
  assert_eq!(
    refused["error"]["details"]["reason"], "component-invalid",
    "{refused}"
  );
}
