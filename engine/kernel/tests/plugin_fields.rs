//! Staged plan S5.2g (ADR 0113): a matching rule or request generator a pact plugin provides at the field
//! level, a matcher-only plugin's (pact-plugins' `creditcard`), is its bridge's `match-field` or
//! `generate-field` action on the one value it names, and an interaction whose plugin rule no bridge
//! provides is refused by the rule's name.
//!
//! The bridge is a stub: a component named `pact-plugin-creditcard` whose handshake describes a `matcher`
//! and a `generator` entry under the `pact-plugin` capability, as `janus-plugin-bridge` does for a plugin of
//! interface version 2, and which records what each application was given. `plugin-bridge/` calls the real
//! plugin, and S1.1's harness holds the whole path to today's verifier (`corpora/verifier/plugins/creditcard*`).

use pact_janus_kernel::component::{
  Apply, ApplyResult, ComponentDeclaration, ComponentError, ComponentLoader, Loaded, MatcherComponent,
  TransportComponent,
};
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const BAD_NUMBER: &str = "4111111111111112";
const GENERATED: &str = "5555555555554444";

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

/// The stub bridge: every application it was given, by action. A card number is fine unless it is
/// [`BAD_NUMBER`], and every generated one is [`GENERATED`].
#[derive(Default)]
struct StubBridge {
  applied: Mutex<Vec<Apply>>,
}

impl MatcherComponent for StubBridge {
  fn apply(&self, req: Apply) -> Result<ApplyResult, ComponentError> {
    self.applied.lock().unwrap().push(req.clone());
    let results = req
      .values
      .iter()
      .map(|value| match req.action.as_str() {
        "pact-plugin-creditcard:match-field" if value["content"] == json!(BAD_NUMBER) => {
          json!({ "status": "error", "message": "fails the Luhn check" })
        }
        "pact-plugin-creditcard:match-field" => json!({ "status": "ok" }),
        _ => json!({ "status": "value", "value": { "value": GENERATED } }),
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
        "component": { "name": "pact-plugin-creditcard", "version": "0.0.0" },
        "interfaces": ["matcher"],
        "contributes": { "actions": [
          { "name": "pact-plugin-creditcard:match-field" },
          { "name": "pact-plugin-creditcard:generate-field" }
        ] },
        "capabilities": { "pact-plugin": {
          "name": "creditcard", "version": "0.0.0", "interface-version": 2,
          "catalogue": [
            { "type": "generator", "key": "creditcard", "action": "pact-plugin-creditcard:generate-field",
              "values": { "config-key": "brand" } },
            { "type": "matcher", "key": "creditcard", "action": "pact-plugin-creditcard:match-field",
              "values": { "config-key": "brand" } }
          ]
        } }
      }),
      content: None,
      transport: None,
      matcher: Some(self.bridge.clone()),
      fetched: None,
    })
  }
}

fn engine(bridge: &Arc<StubBridge>) -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert(
    "http".to_string(),
    Arc::new(pact_janus_component_http::HttpTransport::new()),
  );
  let mut engine = Engine::with_components(
    transports,
    Some(Arc::new(pact_janus_component_json::JsonContent::new())),
  );
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

/// One request the provider received: its request line and its body.
struct Received {
  line: String,
  body: String,
}

/// A provider that answers every request with `reply`, a JSON body, and reports each request it receives.
fn provider(reply: String) -> (String, Receiver<Received>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("an OS-assigned port always binds");
  let base_url = format!("http://{}", listener.local_addr().unwrap());
  let (tx, rx) = mpsc::channel();
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(stream) = stream else { return };
      stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
      let mut reader = BufReader::new(stream);
      let mut line = String::new();
      if reader.read_line(&mut line).is_err() {
        return;
      }
      let mut length = 0usize;
      loop {
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() {
          return;
        }
        let header = header.trim_end();
        if header.is_empty() {
          break;
        }
        if let Some((name, value)) = header.split_once(':')
          && name.trim().eq_ignore_ascii_case("content-length")
        {
          length = value.trim().parse().unwrap_or(0);
        }
      }
      let mut body = vec![0u8; length];
      let _ = reader.read_exact(&mut body);
      let _ = tx.send(Received {
        line: line.trim_end().to_string(),
        body: String::from_utf8_lossy(&body).to_string(),
      });
      let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
        reply.len()
      );
      let stream = reader.get_mut();
      let _ = stream.write_all(response.as_bytes());
      let _ = stream.flush();
    }
  });
  (base_url, rx)
}

/// The credit card example's pact, its response body's card number ruled by `rule`, and its request
/// carrying `request_generators`.
fn pact(rule: &str, request_generators: Value) -> Value {
  json!({
    "consumer": { "name": "CreditCardClient" }, "provider": { "name": "CreditCardServer" },
    "interactions": [ {
      "type": "Synchronous/HTTP", "description": "a request for a Visa card",
      "request": {
        "method": "POST", "path": "/cards/visa",
        "headers": { "Content-Type": ["application/json"] },
        "body": { "content": { "number": "4111111111111111" }, "contentType": "application/json", "encoded": false },
        "generators": request_generators
      },
      "response": {
        "status": 200,
        "body": { "content": { "card": { "number": "4111111111111111", "brand": "visa" } },
                  "contentType": "application/json", "encoded": false },
        "matchingRules": { "body": { "$.card.number": { "combine": "AND",
          "matchers": [ { "match": rule, "brand": "visa" } ] } } }
      }
    } ],
    "metadata": { "pactSpecification": { "version": "4.0" }, "plugins": [ { "name": "creditcard", "version": "0.0.0" } ] }
  })
}

/// Runs a verification of `pact` against `base_url`, and answers its interaction results.
fn verify(engine: &mut Engine, pact: Value, base_url: &str) -> Vec<Value> {
  let started = send(
    engine,
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [pact] },
            "target": { "transports": [ { "transport": "http", "options": { "base-url": base_url } } ],
                        "components": [ { "name": "pact-plugin-creditcard", "source": { "kind": "stub" } } ] } }),
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
        return results;
      }
    }
  }
  panic!("the run does not finish");
}

fn card(number: &str) -> String {
  json!({ "card": { "number": number, "brand": "visa" } }).to_string()
}

#[test]
fn a_plugin_rule_is_its_bridges_match_field_on_the_value_it_names() {
  let bridge = Arc::new(StubBridge::default());
  let mut engine = engine(&bridge);
  let (base_url, _) = provider(card("4012888888881881"));
  let results = verify(&mut engine, pact("creditcard", json!({})), &base_url);
  assert_eq!(results.len(), 1);
  assert_eq!(results[0]["status"], "verified", "{}", results[0]);

  let applied = bridge.applied.lock().unwrap();
  let matched: Vec<&Apply> = applied
    .iter()
    .filter(|apply| apply.action == "pact-plugin-creditcard:match-field")
    .collect();
  assert_eq!(matched.len(), 1);
  // The value under test, from where it was read; the pact's example; and the rule, as the bridge reads it.
  assert_eq!(matched[0].values[0]["content"], "4012888888881881");
  assert_eq!(matched[0].values[0]["path"], "$.response.body.card.number");
  assert_eq!(
    matched[0].config,
    Some(json!({ "arguments": [
      "4111111111111111",
      { "rule": "creditcard", "values": { "brand": "visa" }, "mismatch-type": "body", "root": "$.response.body" }
    ] }))
  );
}

#[test]
fn what_the_plugin_finds_is_a_mismatch_at_the_values_path() {
  let bridge = Arc::new(StubBridge::default());
  let mut engine = engine(&bridge);
  let (base_url, _) = provider(card(BAD_NUMBER));
  let results = verify(&mut engine, pact("matcher/creditcard", json!({})), &base_url);
  assert_eq!(results[0]["status"], "failed", "{}", results[0]);
  let mismatches = results[0]["mismatches"].as_array().expect("mismatches");
  assert_eq!(mismatches.len(), 1, "{mismatches:?}");
  assert_eq!(mismatches[0]["path"], "$.response.body.card.number");
  assert_eq!(mismatches[0]["message"], "fails the Luhn check");
}

#[test]
fn a_plugin_generator_fills_the_request_through_its_bridges_generate_field() {
  let bridge = Arc::new(StubBridge::default());
  let mut engine = engine(&bridge);
  let (base_url, received) = provider(card("4012888888881881"));
  let generators = json!({ "body": { "$.number": { "type": "creditcard", "brand": "mastercard" } } });
  let results = verify(&mut engine, pact("creditcard", generators), &base_url);
  assert_eq!(results[0]["status"], "verified", "{}", results[0]);

  let sent: Vec<Received> = received.try_iter().collect();
  assert_eq!(sent.len(), 1);
  assert_eq!(sent[0].line, "POST /cards/visa HTTP/1.1");
  let body: Value = serde_json::from_str(&sent[0].body).expect("a JSON body");
  assert_eq!(body, json!({ "number": GENERATED }));

  let applied = bridge.applied.lock().unwrap();
  let generated: Vec<&Apply> = applied
    .iter()
    .filter(|apply| apply.action == "pact-plugin-creditcard:generate-field")
    .collect();
  assert_eq!(generated.len(), 1);
  assert_eq!(generated[0].values[0]["content"], "4111111111111111");
  assert_eq!(
    generated[0].config,
    Some(json!({ "arguments": [
      { "generator": { "type": "creditcard", "values": { "brand": "mastercard" } }, "path": "$.number",
        "test-context": {} }
    ] }))
  );
}

#[test]
fn a_plugin_rule_no_bridge_provides_is_refused_by_its_name() {
  let bridge = Arc::new(StubBridge::default());
  let mut engine = engine(&bridge);
  let (base_url, received) = provider(card("4012888888881881"));
  let results = verify(&mut engine, pact("luhn", json!({})), &base_url);
  assert_eq!(results[0]["status"], "failed", "{}", results[0]);
  let error = &results[0]["error"];
  assert_eq!(error["details"]["why"], "rules", "{error}");
  assert!(error["message"].as_str().unwrap().contains("'luhn'"), "{error}");
  assert_eq!(
    received.try_iter().count(),
    0,
    "nothing is sent for a refused interaction"
  );
}
