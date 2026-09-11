//! The built-in HTTP transport component, **serve role only** (component-interfaces spec §5, plan
//! task 4.2): the native binding for a real mock HTTP server, in-tree but through the same
//! interface a third-party transport would use (ADR 0012). Drive role (outbound requests, for
//! provider verification) is Phase 5's; `start`/`send` answer `operation-unsupported` for it.
//!
//! Purely synchronous — [`tiny_http`]'s blocking `recv_timeout` maps directly onto
//! `poll-inbound`'s `timeout-ms`, so no async runtime is needed at all.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_kernel::component::{
  ComponentError, Dispose, DisposeResult, Inbound, Part, Parts, PollInbound, PollInboundResult, Reply,
  ReplyResult, Send, SendResult, SlotValue, Start, StartResult, Stop, StopResult, TransportComponent,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::Duration;

struct Instance {
  server: tiny_http::Server,
  pending: HashMap<String, tiny_http::Request>,
  next_event: u64,
}

/// One live mock server per started instance (spec §3.4: the engine names instances, one
/// component answers all of them — never one component per instance).
#[derive(Default)]
pub struct HttpTransport {
  instances: Mutex<HashMap<String, Instance>>,
}

impl HttpTransport {
  pub fn new() -> Self {
    HttpTransport::default()
  }
}

/// Decode a slot's tagged `content` to raw bytes — the transport's own version of
/// `pact_janus_component_json`'s `slot_text` (a different domain, base64/text/absent rather than
/// JSON-vs-not, so not worth sharing a helper crate over).
fn slot_bytes(value: &SlotValue) -> Result<Vec<u8>, ComponentError> {
  match value.encoded.as_deref() {
    Some("base64") => {
      let text = value
        .content
        .as_str()
        .ok_or_else(|| ComponentError::transport_failed("a base64-tagged slot's content must be a string"))?;
      BASE64
        .decode(text)
        .map_err(|err| ComponentError::transport_failed(format!("invalid base64: {err}")))
    }
    Some("text") | None => match value.content.as_str() {
      Some(text) => Ok(text.as_bytes().to_vec()),
      None => Ok(Vec::new()),
    },
    Some(other) => Err(ComponentError::transport_failed(format!(
      "unrecognised content encoding '{other}'"
    ))),
  }
}

fn text_slot(text: impl Into<String>) -> SlotValue {
  SlotValue {
    content: Value::String(text.into()),
    encoded: None,
    content_type: None,
  }
}

/// Build a `{name: [values...]}` slot (headers, query — spec's own worked example shape, and
/// `legacy.rs`'s existing `query`/`headers` slot vocabulary) from possibly-repeated name/value
/// pairs, grouping and lower-casing names as it goes.
fn multi_map_slot<'a>(entries: impl Iterator<Item = (&'a str, &'a str)>) -> SlotValue {
  let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
  for (name, value) in entries {
    grouped
      .entry(name.to_ascii_lowercase())
      .or_default()
      .push(value.to_string());
  }
  let content = grouped
    .into_iter()
    .map(|(name, values)| {
      (
        name,
        Value::Array(values.into_iter().map(Value::String).collect()),
      )
    })
    .collect();
  SlotValue {
    content: Value::Object(content),
    encoded: None,
    content_type: None,
  }
}

fn request_parts(request: &mut tiny_http::Request) -> Parts {
  let (path, query) = match request.url().split_once('?') {
    Some((path, query)) => (path.to_string(), query.to_string()),
    None => (request.url().to_string(), String::new()),
  };

  let headers: Vec<(String, String)> = request
    .headers()
    .iter()
    .map(|h| {
      (
        h.field.as_str().as_str().to_string(),
        h.value.as_str().to_string(),
      )
    })
    .collect();
  let headers_slot = multi_map_slot(headers.iter().map(|(n, v)| (n.as_str(), v.as_str())));

  let query_pairs: Vec<(String, String)> = query
    .split('&')
    .filter(|pair| !pair.is_empty())
    .map(|pair| match pair.split_once('=') {
      Some((name, value)) => (name.to_string(), value.to_string()),
      None => (pair.to_string(), String::new()),
    })
    .collect();
  let query_slot = multi_map_slot(query_pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())));

  let mut body = Vec::new();
  let _ = request.as_reader().read_to_end(&mut body);
  let content_type = headers
    .iter()
    .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
    .map(|(_, value)| value.clone());
  let body_slot = SlotValue {
    content: Value::String(BASE64.encode(&body)),
    encoded: Some("base64".to_string()),
    content_type,
  };

  let mut request_part: Part = Part::new();
  request_part.insert("method".to_string(), text_slot(request.method().as_str()));
  request_part.insert("path".to_string(), text_slot(path));
  request_part.insert("query".to_string(), query_slot);
  request_part.insert("headers".to_string(), headers_slot);
  request_part.insert("body".to_string(), body_slot);

  let mut parts: Parts = Parts::new();
  parts.insert("request".to_string(), request_part);
  parts
}

impl TransportComponent for HttpTransport {
  fn start(&self, req: Start) -> Result<StartResult, ComponentError> {
    if req.role != "serve" {
      return Err(ComponentError::operation_unsupported(
        "transport/start (role: drive)",
      ));
    }
    let options = req.options.unwrap_or_default();
    let host = options.get("host").and_then(Value::as_str).unwrap_or("127.0.0.1");
    let port = options.get("port").and_then(Value::as_u64).unwrap_or(0);

    let server = tiny_http::Server::http((host, port as u16))
      .map_err(|err| ComponentError::transport_failed(format!("could not bind {host}:{port}: {err}")))?;
    let bound_port = match server.server_addr() {
      tiny_http::ListenAddr::IP(addr) => addr.port(),
      tiny_http::ListenAddr::Unix(_) => port as u16,
    };

    let endpoint = serde_json::json!({
      "kind": "http",
      "host": host,
      "port": bound_port,
      "base-url": format!("http://{host}:{bound_port}"),
    });

    self.instances.lock().expect("instance lock poisoned").insert(
      req.instance,
      Instance {
        server,
        pending: HashMap::new(),
        next_event: 1,
      },
    );
    Ok(StartResult { endpoint })
  }

  fn stop(&self, req: Stop) -> Result<StopResult, ComponentError> {
    self
      .instances
      .lock()
      .expect("instance lock poisoned")
      .remove(&req.instance);
    Ok(StopResult {})
  }

  fn send(&self, _req: Send) -> Result<SendResult, ComponentError> {
    Err(ComponentError::operation_unsupported(
      "transport/send (drive role)",
    ))
  }

  fn poll_inbound(&self, req: PollInbound) -> Result<PollInboundResult, ComponentError> {
    let mut instances = self.instances.lock().expect("instance lock poisoned");
    let instance = instances
      .get_mut(&req.instance)
      .ok_or_else(|| ComponentError::transport_failed(format!("no such instance '{}'", req.instance)))?;

    let received = instance
      .server
      .recv_timeout(Duration::from_millis(req.timeout_ms))
      .map_err(|err| ComponentError::transport_failed(err.to_string()))?;

    let Some(mut request) = received else {
      return Ok(PollInboundResult { inbound: None });
    };

    let parts = request_parts(&mut request);
    let event = format!("e-{}", instance.next_event);
    instance.next_event += 1;
    instance.pending.insert(event.clone(), request);

    Ok(PollInboundResult {
      inbound: Some(Inbound {
        event,
        parts,
        expects_reply: true,
      }),
    })
  }

  fn reply(&self, req: Reply) -> Result<ReplyResult, ComponentError> {
    let mut instances = self.instances.lock().expect("instance lock poisoned");
    let instance = instances
      .get_mut(&req.instance)
      .ok_or_else(|| ComponentError::transport_failed(format!("no such instance '{}'", req.instance)))?;
    let request = instance.pending.remove(&req.event).ok_or_else(|| {
      ComponentError::transport_failed(format!("no pending arrival for event '{}'", req.event))
    })?;

    let response_part = req.parts.get("response").ok_or_else(|| {
      ComponentError::transport_failed("reply parts must carry a 'response' part for an HTTP arrival")
    })?;

    let status = response_part
      .get("status")
      .and_then(|slot| slot.content.as_u64())
      .unwrap_or(200) as u16;
    let body = match response_part.get("body") {
      Some(slot) => slot_bytes(slot)?,
      None => Vec::new(),
    };
    let mut response = tiny_http::Response::from_data(body).with_status_code(status);

    if let Some(headers_slot) = response_part.get("headers")
      && let Value::Object(headers) = &headers_slot.content
    {
      for (name, values) in headers {
        for value in values.as_array().into_iter().flatten().filter_map(Value::as_str) {
          if let Ok(header) = tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response = response.with_header(header);
          }
        }
      }
    }

    request
      .respond(response)
      .map_err(|err| ComponentError::transport_failed(err.to_string()))?;
    Ok(ReplyResult {})
  }

  fn dispose(&self, req: Dispose) -> Result<DisposeResult, ComponentError> {
    let mut instances = self.instances.lock().expect("instance lock poisoned");
    if let Some(instance) = instances.get_mut(&req.instance) {
      // HTTP has no disposition concept (spec §5.3: "a transport whose kind has no such concept
      // ignores it") — but an arrival disposed without a reply would otherwise hang its
      // connection, so answer it minimally before dropping it.
      if let Some(request) = instance.pending.remove(&req.event) {
        let _ = request.respond(tiny_http::Response::from_data(Vec::new()).with_status_code(500));
      }
    }
    Ok(DisposeResult {})
  }
}
