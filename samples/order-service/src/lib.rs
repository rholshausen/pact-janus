//! The sample order-service provider (plan task 5.6): a small HTTP provider with **deliberate
//! variance**, bearer auth and a v3-protocol provider-state endpoint. It is the thing M3 verifies
//! against, the thing M5's undeclared-variance scenario is built on, and the thing a demo points
//! at.
//!
//! Three properties are the reason it exists, and each is a thing under test elsewhere:
//!
//! - **Variance driven by state.** Whether an order carries `shippedAt`, how many items it has and
//!   what `status` it holds are set by provider-state setup, not by the request — so a verification
//!   run that replays several variants of one interaction can make each one meaningful (variant-
//!   semantics spec §5.2, §6).
//! - **A v3 state endpoint, unchanged.** `POST /_pact/provider-states` speaks the protocol
//!   providers already implement, because B5's claim is that an existing provider verifies with no
//!   changes (lifecycle-hooks spec §8.4's `pact-state-change`). Nothing Janus-specific is required
//!   of it.
//! - **More than the consumer asked for.** The provider can produce a `CANCELLED` status and a
//!   `channel` member no consumer contract declares. That is not sloppiness: it is the undeclared
//!   variance Phase 7's subsumption check exists to find, and it has to exist here before it can
//!   be found there.
//!
//! Auth is bearer-token and on by default, so a verification run has to acquire and present a
//! credential — that is what makes plan task 5.3's `before-request` hook a real test and not a
//! decoration.

use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// How long the accept loop blocks before re-checking whether it has been asked to stop.
const ACCEPT_TIMEOUT: Duration = Duration::from_millis(200);

/// The token the binary requires when none is configured. A constant rather than a generated
/// secret: this is a sample, and a demo that cannot be copy-pasted is not a demo.
pub const DEFAULT_TOKEN: &str = "janus-demo-token";

/// The client credentials `POST /oauth/token` accepts. The provider has a token endpoint because a
/// real one does: "acquire a credential, then present it on every request" is the shape plan task
/// 5.3's oauth2 hook component exists for, and a provider that simply accepted a fixed string
/// would let that component look simpler than it is.
pub const DEFAULT_CLIENT_ID: &str = "janus-demo";
pub const DEFAULT_CLIENT_SECRET: &str = "janus-demo-secret";

#[derive(Debug, Clone)]
pub struct Config {
  pub host: String,
  pub port: u16,
  /// `Some` requires `Authorization: Bearer <token>` on every `/orders` route; `None` turns auth
  /// off entirely, which is only for demonstrating what the auth hook *adds*.
  pub token: Option<String>,
}

impl Default for Config {
  fn default() -> Self {
    Config {
      host: "127.0.0.1".to_string(),
      port: 0,
      token: Some(DEFAULT_TOKEN.to_string()),
    }
  }
}

/// One order, as the provider holds it. `shipped_at` being an `Option` is the variance the order
/// payload's `optional` shape describes from the other side.
#[derive(Debug, Clone)]
pub struct Order {
  pub id: String,
  pub status: String,
  pub shipped_at: Option<String>,
  pub items: u64,
  /// Present on every order and declared by no consumer contract — deliberately (see the module
  /// docs).
  pub channel: String,
}

impl Order {
  fn to_json(&self) -> Value {
    let mut order = Map::new();
    order.insert("id".to_string(), json!(self.id));
    order.insert("status".to_string(), json!(self.status));
    // Omitted, never null, when the order has not shipped: absence is the variance, and a null
    // would be a different value the consumer's `optional` shape does not admit.
    if let Some(shipped_at) = &self.shipped_at {
      order.insert("shippedAt".to_string(), json!(shipped_at));
    }
    order.insert(
      "items".to_string(),
      Value::Array(
        (0..self.items)
          .map(|n| json!({ "sku": format!("sku-{n}"), "quantity": n + 1 }))
          .collect(),
      ),
    );
    order.insert("channel".to_string(), json!(self.channel));
    Value::Object(order)
  }
}

/// The provider's whole world: orders by id. Provider-state setup writes it; the routes read it.
#[derive(Debug, Default)]
pub struct Store {
  orders: BTreeMap<String, Order>,
  /// Every state request the provider received, newest last — what a demo prints to show that
  /// setup really did run once per variant (variant-semantics spec §6.6).
  pub state_log: Vec<Value>,
}

impl Store {
  /// The order every run starts with, so a `GET /orders/66` against a freshly started provider
  /// answers something sensible before any state has been set up.
  fn seeded() -> Self {
    let mut store = Store::default();
    store.orders.insert(
      "66".to_string(),
      Order {
        id: "66".to_string(),
        status: "PENDING".to_string(),
        shipped_at: None,
        items: 1,
        channel: "web".to_string(),
      },
    );
    store
  }
}

/// A running provider. Dropping it stops the server — a test that forgets to stop one still leaves
/// no thread behind, which matters because 5.2's tests start one per case.
pub struct Provider {
  base_url: String,
  stop: Arc<AtomicBool>,
  thread: Option<JoinHandle<()>>,
  store: Arc<Mutex<Store>>,
}

impl Provider {
  pub fn base_url(&self) -> &str {
    &self.base_url
  }

  /// The live store, for a test that wants to assert what setup actually did rather than infer it
  /// from a response.
  pub fn store(&self) -> &Arc<Mutex<Store>> {
    &self.store
  }

  pub fn stop(&mut self) {
    self.stop.store(true, Ordering::Relaxed);
    if let Some(thread) = self.thread.take() {
      let _ = thread.join();
    }
  }
}

impl Drop for Provider {
  fn drop(&mut self) {
    self.stop();
  }
}

/// Start the provider on its own thread. Binds before returning, so the caller's first request
/// cannot race the listener — with `port: 0` the bound port is in the returned base URL.
pub fn start(config: Config) -> Result<Provider, String> {
  let server = tiny_http::Server::http((config.host.as_str(), config.port))
    .map_err(|err| format!("could not bind {}:{}: {err}", config.host, config.port))?;
  let port = match server.server_addr() {
    tiny_http::ListenAddr::IP(addr) => addr.port(),
    tiny_http::ListenAddr::Unix(_) => config.port,
  };
  let base_url = format!("http://{}:{port}", config.host);

  let store = Arc::new(Mutex::new(Store::seeded()));
  let stop = Arc::new(AtomicBool::new(false));
  let thread_store = Arc::clone(&store);
  let thread_stop = Arc::clone(&stop);
  let token = config.token.clone();
  let thread = thread::spawn(move || {
    while !thread_stop.load(Ordering::Relaxed) {
      match server.recv_timeout(ACCEPT_TIMEOUT) {
        Ok(Some(request)) => handle(request, &thread_store, token.as_deref()),
        Ok(None) => continue,
        Err(_) => break,
      }
    }
  });

  Ok(Provider {
    base_url,
    stop,
    thread: Some(thread),
    store,
  })
}

fn handle(mut request: tiny_http::Request, store: &Mutex<Store>, token: Option<&str>) {
  let method = request.method().as_str().to_string();
  let (path, _query) = match request.url().split_once('?') {
    Some((path, query)) => (path.to_string(), query.to_string()),
    None => (request.url().to_string(), String::new()),
  };
  let mut body = Vec::new();
  let _ = request.as_reader().read_to_end(&mut body);
  let authorized = token.is_none_or(|token| {
    request
      .headers()
      .iter()
      .any(|header| header.field.equiv("Authorization") && header.value.as_str() == format!("Bearer {token}"))
  });

  let (status, payload) = route(&method, &path, &body, store, authorized, token);
  respond(request, status, payload);
}

fn route(
  method: &str,
  path: &str,
  body: &[u8],
  store: &Mutex<Store>,
  authorized: bool,
  token: Option<&str>,
) -> (u16, Value) {
  match (method, path) {
    // Unauthenticated on purpose: a health check a verification run can reach before it has a
    // credential is what makes "is the provider up" a different question from "is my auth right".
    ("GET", "/health") => (200, json!({ "status": "up" })),
    ("POST", "/oauth/token") => oauth_token(body, token),
    ("POST", "/_pact/provider-states") => provider_state(body, store),
    (_, path) if path.starts_with("/orders") => {
      if !authorized {
        return (
          401,
          json!({ "error": "unauthorized", "message": "a bearer token is required" }),
        );
      }
      orders(method, path, body, store)
    }
    _ => (404, json!({ "error": "not-found", "path": path })),
  }
}

/// A client-credentials token endpoint, in the shape the grant actually has: form-encoded
/// `grant_type`/`client_id`/`client_secret` in, `{access_token, token_type, expires_in}` out. It
/// hands back whatever token this provider is configured to require, so a run that fetches one and
/// presents it gets through — and one that presents the wrong credentials does not.
fn oauth_token(body: &[u8], token: Option<&str>) -> (u16, Value) {
  let form = String::from_utf8_lossy(body);
  let mut fields: BTreeMap<&str, &str> = BTreeMap::new();
  for pair in form.split('&').filter(|pair| !pair.is_empty()) {
    if let Some((name, value)) = pair.split_once('=') {
      fields.insert(name, value);
    }
  }
  let id = fields.get("client_id").copied().unwrap_or_default();
  let secret = fields.get("client_secret").copied().unwrap_or_default();
  if id != DEFAULT_CLIENT_ID || secret != DEFAULT_CLIENT_SECRET {
    return (
      401,
      json!({ "error": "invalid_client", "error_description": "unknown client credentials" }),
    );
  }
  (
    200,
    json!({
      "access_token": token.unwrap_or(DEFAULT_TOKEN),
      "token_type": "Bearer",
      "expires_in": 3600,
    }),
  )
}

fn orders(method: &str, path: &str, body: &[u8], store: &Mutex<Store>) -> (u16, Value) {
  let id = path.strip_prefix("/orders/").map(str::to_string);
  match (method, id) {
    ("GET", Some(id)) => {
      let store = store.lock().expect("store lock poisoned");
      match store.orders.get(&id) {
        Some(order) => (200, order.to_json()),
        None => (404, json!({ "error": "not-found", "id": id })),
      }
    }
    ("GET", None) => {
      let store = store.lock().expect("store lock poisoned");
      (
        200,
        Value::Array(store.orders.values().map(Order::to_json).collect()),
      )
    }
    ("POST", None) => {
      let document: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
      let id = document
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("new")
        .to_string();
      let order = Order {
        id: id.clone(),
        status: "PENDING".to_string(),
        shipped_at: None,
        items: document.get("items").and_then(Value::as_u64).unwrap_or(1),
        channel: "web".to_string(),
      };
      let json = order.to_json();
      store
        .lock()
        .expect("store lock poisoned")
        .orders
        .insert(id, order);
      (201, json)
    }
    _ => (405, json!({ "error": "method-not-allowed", "method": method })),
  }
}

/// The v3 provider-state protocol, exactly as providers already implement it: a POST carrying
/// `{ state, params, action }`, answered 200 on success. Two answers beyond that matter for Janus:
///
/// - **422 with `unsupported`** — the provider cannot reach the state it was asked for. That is
///   variant-semantics spec §6.7's middle row, and it is a different thing from a broken handler:
///   the remedy is a contract change, not a code change.
/// - **500** — the handler itself failed, or the state is one this provider has never heard of.
fn provider_state(body: &[u8], store: &Mutex<Store>) -> (u16, Value) {
  let request: Value = match serde_json::from_slice(body) {
    Ok(request) => request,
    Err(err) => return (400, json!({ "error": "bad-request", "message": err.to_string() })),
  };
  let state = request.get("state").and_then(Value::as_str).unwrap_or_default();
  let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
  let action = request.get("action").and_then(Value::as_str).unwrap_or("setup");

  store
    .lock()
    .expect("store lock poisoned")
    .state_log
    .push(json!({ "state": state, "params": params, "action": action }));

  if action == "teardown" {
    return (200, json!({}));
  }

  match state {
    "an order exists" => {
      let id = params
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("66")
        .to_string();
      // `shipped` arrives as a bound parameter (variant-semantics spec §6.2's worked example), so
      // both the boolean and the point name a looser binding might send are accepted.
      let shipped = match params.get("shipped") {
        Some(Value::Bool(shipped)) => *shipped,
        Some(Value::String(point)) => point == "present" || point == "true",
        _ => false,
      };
      let status = params
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or(if shipped { "SHIPPED" } else { "PENDING" })
        .to_string();

      // The contradiction this provider genuinely cannot produce, and the reason §6.7 exists: an
      // order that has shipped always has a shipping date, so "SHIPPED with no shippedAt" is not a
      // state any amount of setup reaches.
      //
      // Answered `200` with an `unsupported` outcome rather than a 4xx, because that is what the
      // verifier reads as "cannot reach this state" (lifecycle-hooks spec §8.4): a non-2xx is a
      // *failed* state handler, and the difference between the two is the whole point — one is a
      // contract change, the other is a bug. This is the only Janus-shaped thing this endpoint
      // says, and a provider that never says it still verifies unchanged.
      if status == "SHIPPED" && !shipped {
        return (
          200,
          json!({
            "outcome": "unsupported",
            "error": {
              "code": "state-unreachable",
              "message": "an order with status SHIPPED always carries shippedAt",
            },
          }),
        );
      }

      let items = params.get("items").and_then(Value::as_u64).unwrap_or(1);
      let order = Order {
        id: id.clone(),
        status,
        shipped_at: shipped.then(|| "2026-07-30T09:00:00Z".to_string()),
        items,
        channel: "web".to_string(),
      };
      store
        .lock()
        .expect("store lock poisoned")
        .orders
        .insert(id, order);
      (200, json!({}))
    }
    "no orders exist" => {
      store.lock().expect("store lock poisoned").orders.clear();
      (200, json!({}))
    }
    other => (
      500,
      json!({ "error": "unknown-state", "state": other,
              "known": ["an order exists", "no orders exist"] }),
    ),
  }
}

fn respond(request: tiny_http::Request, status: u16, payload: Value) {
  let body = serde_json::to_vec(&payload).expect("a serde_json::Value always serializes");
  let header = tiny_http::Header::from_bytes(&b"content-type"[..], &b"application/json"[..])
    .expect("a constant header always parses");
  let response = tiny_http::Response::from_data(body)
    .with_status_code(status)
    .with_header(header);
  let _ = request.respond(response);
}
