//! The scripted provider: a real HTTP server, on a real socket, that answers a fixture's routes.
//!
//! It recognises a request strictly, by example — method, path, query, the headers the example
//! names, and the body when the example has one — the way the compatibility suite's provider (pact's
//! own mock server) does. A request it does not recognise gets a 404 saying why, so a verifier that
//! sends the wrong request fails the way it would against a real provider.
//!
//! Provider states arrive on `POST /_pact/provider-states`, in the body both verifiers send (`state`,
//! `params`, `action`). The states set up since the last request choose between routes that
//! recognise the same request; a provider whose callback is configured to fail answers every setup
//! with a 500.

use crate::fixture::{Body, Route, StateCallback};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub const STATE_PATH: &str = "/_pact/provider-states";

pub struct ScriptedProvider {
  server: Arc<tiny_http::Server>,
  stop: Arc<AtomicBool>,
  thread: Option<JoinHandle<()>>,
  port: u16,
}

impl ScriptedProvider {
  pub fn start(routes: Vec<Route>, callback: StateCallback) -> Result<ScriptedProvider, String> {
    let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").map_err(|err| err.to_string())?);
    let port = server
      .server_addr()
      .to_ip()
      .map(|addr| addr.port())
      .ok_or("the scripted provider has no IP address")?;
    let stop = Arc::new(AtomicBool::new(false));
    let thread = {
      let server = Arc::clone(&server);
      let stop = Arc::clone(&stop);
      let states = Mutex::new(Vec::<String>::new());
      let failed_once = AtomicBool::new(false);
      std::thread::spawn(move || {
        for request in server.incoming_requests() {
          if stop.load(Ordering::Relaxed) {
            break;
          }
          handle(request, &routes, &callback, &states, &failed_once);
        }
      })
    };
    Ok(ScriptedProvider {
      server,
      stop,
      thread: Some(thread),
      port,
    })
  }

  pub fn base_url(&self) -> String {
    format!("http://127.0.0.1:{}", self.port)
  }

  pub fn port(&self) -> u16 {
    self.port
  }
}

impl Drop for ScriptedProvider {
  fn drop(&mut self) {
    self.stop.store(true, Ordering::Relaxed);
    self.server.unblock();
    if let Some(thread) = self.thread.take() {
      let _ = thread.join();
    }
  }
}

fn handle(
  mut request: tiny_http::Request,
  routes: &[Route],
  callback: &StateCallback,
  states: &Mutex<Vec<String>>,
  failed_once: &AtomicBool,
) {
  let mut body = Vec::new();
  let _ = request.as_reader().read_to_end(&mut body);
  let (path, query) = split_url(request.url());

  if request.method() == &tiny_http::Method::Post && path == STATE_PATH {
    let response = match callback {
      StateCallback::Fail => text(500, "the provider state callback is configured to fail"),
      StateCallback::FailOnce if !failed_once.swap(true, Ordering::Relaxed) => {
        text(503, "the provider state callback is not ready yet; try again")
      }
      StateCallback::None | StateCallback::Ok | StateCallback::FailOnce | StateCallback::Values(_) => {
        let call: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
        let teardown = call["action"].as_str() == Some("teardown");
        if let Some(state) = call["state"]
          .as_str()
          .filter(|state| !teardown && !state.is_empty())
        {
          states.lock().expect("states").push(state.to_string());
        }
        match callback {
          // Today's verifier reads values only from a reply that says it is JSON.
          StateCallback::Values(values) if !teardown => {
            json(200, &serde_json::Value::Object(values.clone()).to_string())
          }
          _ => text(200, "{}"),
        }
      }
    };
    let _ = request.respond(response);
    return;
  }

  let incoming = Incoming {
    method: request.method().as_str().to_uppercase(),
    path,
    query,
    headers: request
      .headers()
      .iter()
      .map(|header| {
        (
          header.field.as_str().as_str().to_ascii_lowercase(),
          header.value.as_str().to_string(),
        )
      })
      .collect(),
    body,
  };
  let set_up = std::mem::take(&mut *states.lock().expect("states"));
  let response = match choose(routes, &incoming, &set_up) {
    Some(route) => respond(route),
    None => text(
      404,
      &format!(
        "the scripted provider recognises no {} {} with this query, these headers and this body",
        incoming.method, incoming.path
      ),
    ),
  };
  let _ = request.respond(response);
}

/// A request as it arrived: path and query decoded, header names lower-cased.
pub struct Incoming {
  pub method: String,
  pub path: String,
  pub query: BTreeMap<String, Vec<String>>,
  pub headers: Vec<(String, String)>,
  pub body: Vec<u8>,
}

/// The route that answers `incoming`: among those that recognise it, the one whose states were just
/// set up, or else the first.
pub fn choose<'a>(routes: &'a [Route], incoming: &Incoming, set_up: &[String]) -> Option<&'a Route> {
  let candidates: Vec<&Route> = routes
    .iter()
    .filter(|route| recognises(route, incoming))
    .collect();
  let mut wanted = set_up.to_vec();
  wanted.sort();
  candidates
    .iter()
    .find(|route| {
      let mut states = route.states.clone();
      states.sort();
      states == wanted
    })
    .or(candidates.first())
    .copied()
}

fn recognises(route: &Route, incoming: &Incoming) -> bool {
  let example = &route.request;
  let pattern = |position: &str| example.patterns.get(position).map(|pattern| whole(pattern));
  if !example.method.eq_ignore_ascii_case(&incoming.method) {
    return false;
  }
  let path_matches = match pattern("path") {
    Some(regex) => regex.is_match(&incoming.path),
    None => example.path == incoming.path,
  };
  if !path_matches {
    return false;
  }

  let mut expected_query = example.query.clone().unwrap_or_default();
  let mut actual_query = incoming.query.clone();
  for name in expected_query.keys().cloned().collect::<Vec<_>>() {
    if let Some(regex) = pattern(&format!("query.{name}")) {
      let actual = actual_query.remove(&name).unwrap_or_default();
      if actual.is_empty() || !actual.iter().all(|value| regex.is_match(value)) {
        return false;
      }
      expected_query.remove(&name);
    }
  }
  if expected_query != actual_query {
    return false;
  }

  for (name, values) in &example.headers {
    let regex = pattern(&format!("headers.{name}"));
    let name = name.to_ascii_lowercase();
    let actual: Vec<String> = incoming
      .headers
      .iter()
      .filter(|(field, _)| *field == name)
      .flat_map(|(_, value)| split_header(&name, value))
      .collect();
    if let Some(regex) = regex {
      if actual.is_empty() || !actual.iter().all(|value| regex.is_match(value)) {
        return false;
      }
      continue;
    }
    let expected: Vec<String> = values
      .iter()
      .flat_map(|value| split_header(&name, value))
      .collect();
    if actual != expected {
      return false;
    }
  }

  match &example.body {
    None => true,
    Some(Body::Json(expected)) => {
      let Ok(mut actual) = serde_json::from_slice::<serde_json::Value>(&incoming.body) else {
        return false;
      };
      let mut expected = expected.clone();
      for (position, regex) in &example.patterns {
        let Some(pointer) = position.strip_prefix("body") else {
          continue;
        };
        let Some(value) = actual.pointer_mut(pointer) else {
          return false;
        };
        let text = match &*value {
          serde_json::Value::String(text) => text.clone(),
          other => other.to_string(),
        };
        if !whole(regex).is_match(&text) {
          return false;
        }
        // Recognised by its pattern, so neither side's value is compared any further.
        *value = serde_json::Value::Null;
        if let Some(value) = expected.pointer_mut(pointer) {
          *value = serde_json::Value::Null;
        }
      }
      actual == expected
    }
    Some(body)
      if example
        .patterns
        .keys()
        .any(|position| position.starts_with("form.")) =>
    {
      form_matches(&body.bytes(), &incoming.body, &example.patterns)
    }
    Some(body) => body.bytes() == incoming.body,
  }
}

/// A form body (`application/x-www-form-urlencoded`) recognised with `form.<name>` patterns: each
/// name's values compared in order, and every value of a patterned name matching its pattern instead.
fn form_matches(expected: &[u8], actual: &[u8], patterns: &BTreeMap<String, String>) -> bool {
  let read = |body: &[u8]| -> Option<BTreeMap<String, Vec<String>>> {
    let pairs: Vec<(String, String)> = serde_urlencoded::from_bytes(body).ok()?;
    let mut names: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, value) in pairs {
      names.entry(name).or_default().push(value);
    }
    Some(names)
  };
  let (Some(mut expected), Some(mut actual)) = (read(expected), read(actual)) else {
    return false;
  };
  for (position, regex) in patterns {
    let Some(name) = position.strip_prefix("form.") else {
      continue;
    };
    let values = actual.remove(name).unwrap_or_default();
    if values.is_empty() || !values.iter().all(|value| whole(regex).is_match(value)) {
      return false;
    }
    expected.remove(name);
  }
  expected == actual
}

/// `pattern`, anchored so it must match the whole value. A fixture's pattern that does not parse is
/// a broken fixture, so it matches nothing and the route is never chosen.
fn whole(pattern: &str) -> regex::Regex {
  regex::Regex::new(&format!("^(?:{pattern})$"))
    .unwrap_or_else(|_| regex::Regex::new("[^\\s\\S]").expect("a valid regex"))
}

/// A header's values as pact compares them: split on commas, trimmed; a content type compared by
/// its media type and parameters with the spaces taken out.
fn split_header(name: &str, value: &str) -> Vec<String> {
  if name == "content-type" {
    return vec![value.replace(' ', "").to_ascii_lowercase()];
  }
  value.split(',').map(|part| part.trim().to_string()).collect()
}

fn respond(route: &Route) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
  if let Some(delay) = route.response.delay_ms {
    std::thread::sleep(std::time::Duration::from_millis(delay));
  }
  let body = route.response.body.as_ref().map(Body::bytes).unwrap_or_default();
  let mut response = tiny_http::Response::from_data(body).with_status_code(route.response.status);
  // One header line per value, as pact's mock server sends them: a header that is a list arrives
  // as a list, rather than as one value with commas in it.
  for (name, values) in &route.response.headers {
    for value in values {
      if let Ok(header) = tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()) {
        response.add_header(header);
      }
    }
  }
  response
}

fn text(status: u16, body: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
  tiny_http::Response::from_data(body.as_bytes().to_vec()).with_status_code(status)
}

fn json(status: u16, body: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
  let header = tiny_http::Header::from_bytes("Content-Type", "application/json").expect("a valid header");
  text(status, body).with_header(header)
}

/// `/a%20b?x=1&y` as `("/a b", {x: ["1"], y: [""]})`.
pub fn split_url(url: &str) -> (String, BTreeMap<String, Vec<String>>) {
  let (path, query) = url.split_once('?').unwrap_or((url, ""));
  let mut parsed: BTreeMap<String, Vec<String>> = BTreeMap::new();
  for pair in query.split('&').filter(|pair| !pair.is_empty()) {
    let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
    parsed
      .entry(decode(name, true))
      .or_default()
      .push(decode(value, true));
  }
  (decode(path, false), parsed)
}

fn decode(text: &str, plus_is_space: bool) -> String {
  let bytes = text.as_bytes();
  let mut out = Vec::with_capacity(bytes.len());
  let mut index = 0;
  while index < bytes.len() {
    match bytes[index] {
      b'%' if index + 2 < bytes.len() => {
        let hex = |byte: u8| (byte as char).to_digit(16);
        match (hex(bytes[index + 1]), hex(bytes[index + 2])) {
          (Some(high), Some(low)) => {
            out.push((high * 16 + low) as u8);
            index += 3;
            continue;
          }
          _ => out.push(b'%'),
        }
      }
      b'+' if plus_is_space => out.push(b' '),
      other => out.push(other),
    }
    index += 1;
  }
  String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fixture::{RequestExample, ResponseSpec};
  use pretty_assertions::assert_eq;
  use serde_json::json;

  fn route(states: &[&str], path: &str, status: u16) -> Route {
    Route {
      states: states.iter().map(|state| state.to_string()).collect(),
      request: RequestExample {
        method: "GET".to_string(),
        path: path.to_string(),
        query: None,
        headers: BTreeMap::new(),
        body: None,
        patterns: BTreeMap::new(),
      },
      response: ResponseSpec {
        status,
        delay_ms: None,
        headers: BTreeMap::new(),
        body: None,
      },
    }
  }

  fn get(url: &str) -> Incoming {
    let (path, query) = split_url(url);
    Incoming {
      method: "GET".to_string(),
      path,
      query,
      headers: vec![],
      body: vec![],
    }
  }

  #[test]
  fn a_url_is_decoded_into_a_path_and_a_query() {
    let (path, query) = split_url("/a%20b?x=1&x=2&y&z=a+b%26c");
    assert_eq!(path, "/a b");
    assert_eq!(
      query,
      BTreeMap::from([
        ("x".to_string(), vec!["1".to_string(), "2".to_string()]),
        ("y".to_string(), vec![String::new()]),
        ("z".to_string(), vec!["a b&c".to_string()]),
      ])
    );
  }

  #[test]
  fn the_states_just_set_up_choose_between_routes_for_the_same_request() {
    let routes = vec![
      route(&["an order exists"], "/orders/1", 200),
      route(&["no orders exist"], "/orders/1", 404),
    ];
    let incoming = get("/orders/1");
    assert_eq!(
      choose(&routes, &incoming, &["no orders exist".to_string()])
        .unwrap()
        .response
        .status,
      404
    );
    assert_eq!(
      choose(&routes, &incoming, &["an order exists".to_string()])
        .unwrap()
        .response
        .status,
      200
    );
    assert_eq!(
      choose(&routes, &incoming, &[]).unwrap().response.status,
      200,
      "or the first"
    );
  }

  #[test]
  fn a_request_is_recognised_strictly_by_example() {
    let mut expected = route(&[], "/basic", 200);
    expected.request.query = Some(BTreeMap::from([("a".to_string(), vec!["1".to_string()])]));
    expected.request.headers = BTreeMap::from([("X-Test".to_string(), vec!["Compatibility".to_string()])]);
    expected.request.body = Some(Body::Json(json!({ "one": 1 })));
    let routes = vec![expected];

    let mut incoming = get("/basic?a=1");
    incoming.headers = vec![("x-test".to_string(), "Compatibility".to_string())];
    incoming.body = br#"{ "one": 1 }"#.to_vec();
    assert!(choose(&routes, &incoming, &[]).is_some());

    incoming.body = br#"{ "one": 2 }"#.to_vec();
    assert!(choose(&routes, &incoming, &[]).is_none(), "another body");
    incoming.body = br#"{ "one": 1 }"#.to_vec();
    incoming.headers.clear();
    assert!(choose(&routes, &incoming, &[]).is_none(), "a missing header");
    assert!(choose(&routes, &get("/basic"), &[]).is_none(), "a missing query");
  }

  #[test]
  fn a_generated_value_is_recognised_by_its_pattern() {
    let mut expected = route(&[], "/orders/1", 200);
    expected.request.query = Some(BTreeMap::from([("v1".to_string(), vec!["0".to_string()])]));
    expected.request.headers = BTreeMap::from([("X-Test".to_string(), vec!["0".to_string()])]);
    expected.request.body = Some(Body::Json(json!({ "one": "a", "two": "b" })));
    expected.request.patterns = BTreeMap::from([
      ("path".to_string(), "/orders/\\d+".to_string()),
      ("query.v1".to_string(), "\\d+".to_string()),
      ("headers.X-Test".to_string(), "\\d+".to_string()),
      ("body/one".to_string(), "\\d+".to_string()),
    ]);
    let routes = vec![expected];

    let mut incoming = get("/orders/1234?v1=7");
    incoming.headers = vec![("x-test".to_string(), "42".to_string())];
    incoming.body = br#"{ "one": 300, "two": "b" }"#.to_vec();
    assert!(choose(&routes, &incoming, &[]).is_some());

    incoming.body = br#"{ "one": "a", "two": "b" }"#.to_vec();
    assert!(
      choose(&routes, &incoming, &[]).is_none(),
      "the recorded body value is not generated"
    );
    incoming.body = br#"{ "one": 300, "two": "c" }"#.to_vec();
    assert!(
      choose(&routes, &incoming, &[]).is_none(),
      "the rest of the body is still exact"
    );
    incoming.body = br#"{ "one": 300, "two": "b" }"#.to_vec();
    incoming.headers = vec![("x-test".to_string(), "x".to_string())];
    assert!(
      choose(&routes, &incoming, &[]).is_none(),
      "a header not of the pattern's form"
    );
    incoming.headers = vec![("x-test".to_string(), "42".to_string())];
    incoming.path = "/orders/abc".to_string();
    assert!(
      choose(&routes, &incoming, &[]).is_none(),
      "a path not of the pattern's form"
    );
  }

  #[test]
  fn a_values_callback_answers_setup_with_json_values() {
    let values = serde_json::Map::from_iter([("id".to_string(), json!(1234))]);
    let provider = ScriptedProvider::start(vec![], StateCallback::Values(values)).unwrap();
    let agent = ureq::Agent::new_with_config(
      ureq::config::Config::builder()
        .http_status_as_error(false)
        .build(),
    );
    let call = |action: &str| {
      let mut response = agent
        .post(format!("{}{STATE_PATH}", provider.base_url()))
        .send(json!({ "state": "one", "params": {}, "action": action }).to_string())
        .unwrap();
      let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
      (content_type, response.body_mut().read_to_string().unwrap())
    };
    assert_eq!(
      call("setup"),
      (Some("application/json".to_string()), r#"{"id":1234}"#.to_string())
    );
    assert_eq!(call("teardown").1, "{}", "values are a setup's answer");
  }

  #[test]
  fn it_answers_on_a_socket_and_takes_provider_states() {
    let routes = vec![route(&["one"], "/x", 201), route(&[], "/x", 202)];
    let provider = ScriptedProvider::start(routes, StateCallback::Ok).unwrap();
    let agent = ureq::Agent::new_with_config(
      ureq::config::Config::builder()
        .http_status_as_error(false)
        .build(),
    );
    let status = |url: String| agent.get(url).call().unwrap().status().as_u16();
    assert_eq!(
      status(format!("{}/x", provider.base_url())),
      202,
      "the route with no states, when none were set up"
    );
    agent
      .post(format!("{}{STATE_PATH}", provider.base_url()))
      .send(json!({ "state": "one", "params": {}, "action": "setup" }).to_string())
      .unwrap();
    assert_eq!(status(format!("{}/x", provider.base_url())), 201);
    assert_eq!(status(format!("{}/nothing", provider.base_url())), 404);
  }

  #[test]
  fn a_callback_that_fails_once_succeeds_when_asked_again() {
    let provider = ScriptedProvider::start(vec![], StateCallback::FailOnce).unwrap();
    let agent = ureq::Agent::new_with_config(
      ureq::config::Config::builder()
        .http_status_as_error(false)
        .build(),
    );
    let setup = || {
      agent
        .post(format!("{}{STATE_PATH}", provider.base_url()))
        .send(json!({ "state": "one", "params": {}, "action": "setup" }).to_string())
        .unwrap()
        .status()
        .as_u16()
    };
    assert_eq!(setup(), 503);
    assert_eq!(setup(), 200);
    assert_eq!(setup(), 200);
  }
}
