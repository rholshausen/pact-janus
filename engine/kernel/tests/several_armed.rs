//! ADR 0069 (staged plan S2.13): several interactions armed at once on one transport, each until its
//! variant is reported, and an arrival given to the arming it matches best. Against the real HTTP
//! transport, as a test that adds several interactions and then runs its client drives it.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "r", "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).unwrap());
  serde_json::from_slice(&bytes).unwrap()
}

/// A session over the real HTTP transport with `interactions` added, each one's base variant armed
/// in order: the engine, the session, each handle with its base variant, and the mock's address.
struct Session {
  engine: Engine,
  session: String,
  armed: Vec<(String, String)>,
  addr: String,
}

impl Session {
  fn arming(interactions: &[Value]) -> Session {
    let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
    transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
    let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
    engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
    send(
      &mut engine,
      "engine/hello",
      json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" },
              "capabilities": { "test-verdicts": {} } }),
    );
    let created = send(
      &mut engine,
      "consumer-session/create",
      json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "user-api" } } }),
    );
    let session = created["ok"]["session"].as_str().unwrap().to_string();
    let started = send(
      &mut engine,
      "consumer-session/start-transport",
      json!({ "session": session, "transport": "http" }),
    );
    let endpoint = &started["ok"]["endpoint"];
    let addr = format!(
      "{}:{}",
      endpoint["host"].as_str().unwrap(),
      endpoint["port"].as_u64().unwrap()
    );
    let mut armed = Vec::new();
    for interaction in interactions {
      let added = send(
        &mut engine,
        "consumer-session/add-interaction",
        json!({ "session": session, "interaction": interaction }),
      );
      let handle = added["ok"]["handle"]
        .as_str()
        .unwrap_or_else(|| panic!("{added}"))
        .to_string();
      let selected = send(
        &mut engine,
        "consumer-session/variants",
        json!({ "session": session, "handle": handle }),
      );
      let base = selected["ok"]["variants"][0]["id"].as_str().unwrap().to_string();
      let served = send(
        &mut engine,
        "consumer-session/serve-variant",
        json!({ "session": session, "handle": handle, "variant": base }),
      );
      assert_eq!(served["ok"], json!({}), "{served}");
      armed.push((handle, base));
    }
    Session {
      engine,
      session,
      armed,
      addr,
    }
  }

  /// Report every armed variant passed, answering each one's result.
  fn report_all(&mut self) -> Vec<Value> {
    let armed = self.armed.clone();
    armed
      .iter()
      .map(|(handle, variant)| {
        let reported = send(
          &mut self.engine,
          "consumer-session/report-variant",
          json!({ "session": self.session, "handle": handle, "variant": variant, "status": "passed" }),
        );
        reported["ok"]["result"].clone()
      })
      .collect()
  }

  fn finalise(&mut self) -> Value {
    send(
      &mut self.engine,
      "consumer-session/finalise",
      json!({ "session": self.session }),
    )["ok"]
      .clone()
  }
}

/// A GET of `path` answering `body`, `accept` the `Accept` header it requires when it names one.
fn user(description: &str, path: &str, accept: Option<&str>, body: Value) -> Value {
  let mut request = json!({
    "method": { "shape": "equality", "example": "GET" },
    "path": { "shape": "equality", "example": path }
  });
  if let Some(accept) = accept {
    request["headers"] = json!({ "shape": "object", "members": {
      "accept": { "shape": "equality", "example": [accept] } } });
  }
  json!({
    "description": description,
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": request,
      "response": { "status": { "shape": "equality", "example": 200 },
                    "body": { "shape": "equality", "example": body } }
    }
  })
}

/// One GET, with an `Accept` header when one is given: its status and body.
fn get(addr: &str, path: &str, accept: Option<&str>) -> (u16, Value) {
  let mut stream = TcpStream::connect(addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  let accept = accept
    .map(|accept| format!("Accept: {accept}\r\n"))
    .unwrap_or_default();
  let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\n{accept}Connection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let text = String::from_utf8_lossy(&response).to_string();
  let status = text
    .split_whitespace()
    .nth(1)
    .and_then(|s| s.parse().ok())
    .unwrap_or_else(|| panic!("no status in: {text}"));
  let body = text.split("\r\n\r\n").nth(1).unwrap_or_default();
  (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

fn statuses(results: &[Value]) -> Vec<&str> {
  results
    .iter()
    .map(|result| result["status"].as_str().unwrap())
    .collect()
}

#[test]
fn two_armed_interactions_are_each_served_their_own_reply() {
  let mut session = Session::arming(&[
    user("user 1", "/users/1", None, json!({ "id": 1 })),
    user("user 2", "/users/2", None, json!({ "id": 2 })),
  ]);
  // In the other order from their arming: each arrival goes to the arming it matches.
  assert_eq!(get(&session.addr, "/users/2", None), (200, json!({ "id": 2 })));
  assert_eq!(get(&session.addr, "/users/1", None), (200, json!({ "id": 1 })));
  assert_eq!(statuses(&session.report_all()), ["verified", "verified"]);
  assert!(
    session.finalise()["contract"].is_object(),
    "both verified, so the contract is written"
  );
}

#[test]
fn an_arming_serves_every_call_until_its_variant_is_reported() {
  let mut session = Session::arming(&[user("user 1", "/users/1", None, json!({ "id": 1 }))]);
  for _ in 0..3 {
    assert_eq!(get(&session.addr, "/users/1", None), (200, json!({ "id": 1 })));
  }
  assert_eq!(statuses(&session.report_all()), ["verified"]);
  // Reported, so disarmed: a later call matches nothing.
  assert_eq!(get(&session.addr, "/users/1", None).0, 500);
}

#[test]
fn an_arrival_two_armings_match_goes_to_the_more_specific() {
  let mut session = Session::arming(&[
    user("any user 1", "/users/1", None, json!({ "kind": "any" })),
    user(
      "user 1 as JSON",
      "/users/1",
      Some("application/json"),
      json!({ "kind": "json" }),
    ),
  ]);
  assert_eq!(
    get(&session.addr, "/users/1", Some("application/json")),
    (200, json!({ "kind": "json" })),
    "the arming that also checks the header passes more of its plan"
  );
  assert_eq!(
    get(&session.addr, "/users/1", Some("text/plain")),
    (200, json!({ "kind": "any" })),
    "only the general one matches"
  );
  assert_eq!(statuses(&session.report_all()), ["verified", "verified"]);
}

#[test]
fn an_arrival_no_arming_matches_fails_the_closest_and_only_that_one() {
  let mut session = Session::arming(&[
    user("user 1", "/users/1", None, json!({ "id": 1 })),
    user(
      "user 2 as JSON",
      "/users/2",
      Some("application/json"),
      json!({ "id": 2 }),
    ),
  ]);
  assert_eq!(get(&session.addr, "/users/1", None).0, 200);
  // The right path without the header: one mismatch against the second, two against the first.
  assert_eq!(get(&session.addr, "/users/2", Some("text/plain")).0, 500);
  let results = session.report_all();
  assert_eq!(statuses(&results), ["verified", "failed"], "{results:#?}");
  let paths: Vec<&str> = results[1]["mismatches"]
    .as_array()
    .unwrap()
    .iter()
    .map(|mismatch| mismatch["path"].as_str().unwrap_or_default())
    .collect();
  assert_eq!(paths, ["$.request.headers.accept"], "{results:#?}");
}

#[test]
fn a_failed_exchange_fails_its_variant_though_a_later_one_matched() {
  let mut session = Session::arming(&[user(
    "user 1 as JSON",
    "/users/1",
    Some("application/json"),
    json!({ "id": 1 }),
  )]);
  assert_eq!(get(&session.addr, "/users/1", Some("text/plain")).0, 500);
  assert_eq!(get(&session.addr, "/users/1", Some("application/json")).0, 200);
  let results = session.report_all();
  assert_eq!(statuses(&results), ["failed"], "{results:#?}");
  assert!(session.finalise()["contract"].is_null());
}

#[test]
fn an_armed_interaction_never_called_withholds_the_contract_naming_it() {
  let mut session = Session::arming(&[
    user("user 1", "/users/1", None, json!({ "id": 1 })),
    user("user 2", "/users/2", None, json!({ "id": 2 })),
  ]);
  assert_eq!(get(&session.addr, "/users/1", None).0, 200);
  assert_eq!(statuses(&session.report_all()), ["verified", "not-exercised"]);
  let finalised = session.finalise();
  assert!(finalised["contract"].is_null(), "{finalised}");
  let statuses: Vec<(&str, &str)> = finalised["results"]
    .as_array()
    .unwrap()
    .iter()
    .map(|result| {
      (
        result["handle"].as_str().unwrap(),
        result["status"].as_str().unwrap(),
      )
    })
    .collect();
  assert_eq!(statuses[1].1, "not-exercised", "{finalised}");
}

#[test]
fn arming_a_handle_again_replaces_its_arming_and_leaves_the_others() {
  let order = json!({
    "description": "an order",
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders/1" } },
      "response": { "status": { "shape": "equality", "example": 200 },
                    "body": { "shape": "object", "members": {
                      "note": { "shape": "optional", "of": { "shape": "string", "example": "fragile" } } } } }
    }
  });
  let mut session = Session::arming(&[order, user("user 1", "/users/1", None, json!({ "id": 1 }))]);
  let (order_handle, base) = session.armed[0].clone();
  let selected = send(
    &mut session.engine,
    "consumer-session/variants",
    json!({ "session": session.session, "handle": order_handle }),
  );
  let absent = selected["ok"]["variants"][1]["id"].as_str().unwrap().to_string();
  assert_ne!(absent, base);
  send(
    &mut session.engine,
    "consumer-session/serve-variant",
    json!({ "session": session.session, "handle": order_handle, "variant": absent }),
  );
  assert_eq!(
    get(&session.addr, "/orders/1", None),
    (200, json!({})),
    "the note-absent variant's reply"
  );
  assert_eq!(
    get(&session.addr, "/users/1", None),
    (200, json!({ "id": 1 })),
    "still armed"
  );
}
