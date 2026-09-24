//! The built-in HTTP transport component (component-interfaces spec §5): the native binding for a
//! real mock HTTP server in the **serve** role (plan task 4.2) and a real HTTP client in the
//! **drive** role (plan task 5.1), in-tree but through the same interface a third-party transport
//! would use (ADR 0012).
//!
//! One component, two roles, five role-neutral primitives (spec §5.1): a served instance answers
//! `poll-inbound`/`reply`/`dispose`, a driven one answers `send`, and each answers
//! `operation-unsupported` for the other's — spec §5.2's rule that "no reply expected" and "reply
//! expected but absent" are different observations applies to whole operations too, so a driven
//! instance asked to poll says so by name rather than returning "nothing arrived" forever.
//!
//! Purely synchronous — [`tiny_http`]'s blocking `recv_timeout` maps directly onto
//! `poll-inbound`'s `timeout-ms`, and a [`ureq`] call is one request and its reply, which is
//! exactly what `send` asks for — so no async runtime is needed at all.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_kernel::component::{
  ComponentError, ContentSlots, Dispose, DisposeResult, Inbound, Part, Parts, PollInbound, PollInboundResult,
  Reply, ReplyResult, Send, SendResult, SlotValue, Start, StartResult, Stop, StopResult, TransportComponent,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// One started instance, in whichever role `start` named (spec §5.1). The role is fixed at
/// `start` and never re-negotiated, which is what lets every later operation answer
/// `operation-unsupported` from the instance alone.
enum Instance {
  Serve(ServeInstance),
  Drive(DriveInstance),
}

struct ServeInstance {
  /// Shared so that `poll-inbound` can wait on it without holding the lock over every instance
  /// (phase-9 finding 27): a waiter clones this and lets go, and `stop` wakes it with `unblock`.
  server: Arc<tiny_http::Server>,
  pending: HashMap<String, tiny_http::Request>,
  next_event: u64,
}

/// The drive role's whole state: where to send. No connection pool is kept deliberately — a
/// `send` carries its own `timeout-ms`, so the agent that honours it is built per call (spec §5.2),
/// and a verification run's cost is dominated by the provider, not by a TCP handshake.
struct DriveInstance {
  base_url: String,
}

/// One live instance per `start` (spec §3.4: the engine names instances, one component answers all
/// of them — never one component per instance).
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

/// How a `{name: [values...]}` slot treats names. HTTP header names are case-insensitive, so
/// they are folded to lower case — the one spelling a shape can name them by (shape spec §3.6).
/// Query parameter names are case-sensitive, in HTTP and in every v1–v4 pact, so they keep theirs.
#[derive(Clone, Copy)]
enum Names {
  FoldCase,
  AsSent,
}

/// Build a `{name: [values...]}` slot (headers, query — spec's own worked example shape, and
/// `legacy.rs`'s existing `query`/`headers` slot vocabulary) from possibly-repeated name/value
/// pairs, grouping names as it goes.
fn multi_map_slot<'a>(entries: impl Iterator<Item = (&'a str, &'a str)>, names: Names) -> SlotValue {
  let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
  for (name, value) in entries {
    let name = match names {
      Names::FoldCase => name.to_ascii_lowercase(),
      Names::AsSent => name.to_string(),
    };
    grouped.entry(name).or_default().push(value.to_string());
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
  let headers_slot = multi_map_slot(
    headers.iter().map(|(n, v)| (n.as_str(), v.as_str())),
    Names::FoldCase,
  );

  // Decoded, as a pact records them and as `query_string` encodes them on the way out.
  let query_pairs: Vec<(String, String)> = form_urlencoded::parse(query.as_bytes())
    .map(|(name, value)| (name.into_owned(), value.into_owned()))
    .collect();
  let query_slot = multi_map_slot(
    query_pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())),
    Names::AsSent,
  );

  // A client offering a protocol upgrade (`Connection: Upgrade` — curl's `--http2`, the JDK's
  // default HttpClient) gets tiny_http's raw socket as its body reader, so reading "to the end"
  // would wait for the client to hang up while the client waits for the response. The mock never
  // accepts an upgrade — answering HTTP/1.1 and ignoring the offer is what RFC 9110 §7.8 allows —
  // so only the body the request declared is read.
  let upgrade_offered = headers.iter().any(|(name, value)| {
    name.eq_ignore_ascii_case("connection") && value.to_ascii_lowercase().contains("upgrade")
  });
  let mut body = Vec::new();
  if upgrade_offered {
    let declared = request.body_length().unwrap_or(0) as u64;
    let _ = request.as_reader().take(declared).read_to_end(&mut body);
  } else {
    let _ = request.as_reader().read_to_end(&mut body);
  }
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

/// The serve-role instance behind `instance`, or the reason it is not one. Two different
/// failures, deliberately distinct: an unknown instance is a host bookkeeping error, while a
/// driven instance asked to serve is a role confusion, and `operation-unsupported` naming the
/// operation is how spec §5.1's "a transport declares which roles it supports" reads at run time.
fn serve_instance<'a>(
  instances: &'a mut HashMap<String, Instance>,
  instance: &str,
  op: &str,
) -> Result<&'a mut ServeInstance, ComponentError> {
  match instances.get_mut(instance) {
    Some(Instance::Serve(serve)) => Ok(serve),
    Some(Instance::Drive(_)) => Err(ComponentError::operation_unsupported(&format!(
      "{op} (drive role)"
    ))),
    None => Err(ComponentError::transport_failed(format!(
      "no such instance '{instance}'"
    ))),
  }
}

/// Turn a `request` part into an outbound request builder: method, `base-url` + path, the query
/// slot re-encoded, and every header value. The slot vocabulary is the same one `request_parts`
/// produces on the serve side — this function and that one are each other's inverse, which is what
/// makes a mock's recording replayable at a provider.
fn build_request(base_url: &str, part: &Part) -> Result<ureq::http::request::Builder, ComponentError> {
  let method = part
    .get("method")
    .and_then(|slot| slot.content.as_str())
    .unwrap_or("GET")
    .to_ascii_uppercase();
  let path = part
    .get("path")
    .and_then(|slot| slot.content.as_str())
    .unwrap_or("/");
  let query = part.get("query").map(query_string).unwrap_or_default();

  let mut url = format!("{base_url}{path}");
  if !query.is_empty() {
    url.push('?');
    url.push_str(&query);
  }

  let mut builder = ureq::http::Request::builder().method(method.as_str()).uri(&url);
  if let Some(headers_slot) = part.get("headers")
    && let Value::Object(headers) = &headers_slot.content
  {
    for (name, values) in headers {
      for value in values.as_array().into_iter().flatten().filter_map(Value::as_str) {
        builder = builder.header(name.as_str(), value);
      }
    }
  }
  // The body's own content type travels with the body slot (spec §6's bytes/document boundary),
  // not in the headers slot, so it is applied here rather than lost.
  if let Some(content_type) = part.get("body").and_then(|slot| slot.content_type.as_deref())
    && !builder
      .headers_ref()
      .is_some_and(|headers| headers.contains_key("content-type"))
  {
    builder = builder.header("content-type", content_type);
  }
  Ok(builder)
}

/// `{name: [values...]}` back to `name=value&name=value`, in the slot's own (sorted) order so the
/// same recorded request replays byte-identically every time.
fn query_string(slot: &SlotValue) -> String {
  let Value::Object(query) = &slot.content else {
    return slot.content.as_str().unwrap_or_default().to_string();
  };
  let mut pairs = form_urlencoded::Serializer::new(String::new());
  for (name, values) in query {
    for value in values.as_array().into_iter().flatten().filter_map(Value::as_str) {
      pairs.append_pair(name, value);
    }
  }
  pairs.finish()
}

/// The provider's response as a `response` part — the same slot vocabulary the mock side records,
/// body base64-tagged because a transport carries bytes and never decides what they mean
/// (spec §4, §5.4).
fn reply_parts(mut response: ureq::http::Response<ureq::Body>) -> Result<Parts, ComponentError> {
  let status = response.status().as_u16();
  let headers: Vec<(String, String)> = response
    .headers()
    .iter()
    .map(|(name, value)| {
      (
        name.as_str().to_string(),
        String::from_utf8_lossy(value.as_bytes()).to_string(),
      )
    })
    .collect();
  let content_type = headers
    .iter()
    .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
    .map(|(_, value)| value.clone());
  let body = response
    .body_mut()
    .read_to_vec()
    .map_err(|err| ComponentError::transport_failed(format!("could not read the response body: {err}")))?;

  let mut response_part: Part = Part::new();
  response_part.insert(
    "status".to_string(),
    SlotValue {
      content: Value::from(status),
      encoded: None,
      content_type: None,
    },
  );
  response_part.insert(
    "headers".to_string(),
    multi_map_slot(
      headers.iter().map(|(n, v)| (n.as_str(), v.as_str())),
      Names::FoldCase,
    ),
  );
  response_part.insert(
    "body".to_string(),
    SlotValue {
      content: Value::String(BASE64.encode(&body)),
      encoded: Some("base64".to_string()),
      content_type,
    },
  );

  let mut parts: Parts = Parts::new();
  parts.insert("response".to_string(), response_part);
  Ok(parts)
}

impl TransportComponent for HttpTransport {
  /// Bodies are content; method, path, query, status and the headers map are plain values
  /// (component-interfaces spec §5.5).
  fn content_slots(&self) -> ContentSlots {
    ContentSlots::from([
      ("request".to_string(), vec!["body".to_string()]),
      ("response".to_string(), vec!["body".to_string()]),
    ])
  }

  fn start(&self, req: Start) -> Result<StartResult, ComponentError> {
    match req.role.as_str() {
      "serve" => self.start_serve(req),
      "drive" => self.start_drive(req),
      other => Err(ComponentError::operation_unsupported(&format!(
        "transport/start (role: {other})"
      ))),
    }
  }

  fn stop(&self, req: Stop) -> Result<StopResult, ComponentError> {
    let removed = self
      .instances
      .lock()
      .expect("instance lock poisoned")
      .remove(&req.instance);
    // A `poll-inbound` may be waiting on this server right now; wake it rather than let `stop`
    // (and the `finalise` behind it) sit out the rest of its timeout. The server closes when the
    // waiter drops its clone.
    if let Some(Instance::Serve(instance)) = removed {
      instance.server.unblock();
    }
    Ok(StopResult {})
  }

  /// `send` (spec §5.2) — the drive role's whole job: one request at the provider, its reply back
  /// as parts. `await-reply: false` still performs the request (a provider is not a broker; there
  /// is no way to speak HTTP without a response arriving) and reports no reply, which is the
  /// honest reading of "fire and forget" for this kind rather than an invented empty reply.
  fn send(&self, req: Send) -> Result<SendResult, ComponentError> {
    let base_url = {
      let instances = self.instances.lock().expect("instance lock poisoned");
      match instances.get(&req.instance) {
        Some(Instance::Drive(drive)) => drive.base_url.clone(),
        Some(Instance::Serve(_)) => {
          return Err(ComponentError::operation_unsupported(
            "transport/send (serve role)",
          ));
        }
        None => {
          return Err(ComponentError::transport_failed(format!(
            "no such instance '{}'",
            req.instance
          )));
        }
      }
    };

    let request_part = req.parts.get("request").ok_or_else(|| {
      ComponentError::transport_failed("send parts must carry a 'request' part for an HTTP request")
    })?;
    let outbound = build_request(&base_url, request_part)?;
    let body = match request_part.get("body") {
      Some(slot) => slot_bytes(slot)?,
      None => Vec::new(),
    };

    // A fresh agent per call: it is what carries this `send`'s own `timeout-ms`, and
    // `http_status_as_error(false)` is not optional here — a 404 or a 500 is the observation a
    // verification run exists to make, never a transport failure.
    let config = ureq::Agent::config_builder()
      .http_status_as_error(false)
      .timeout_global(req.timeout_ms.map(Duration::from_millis))
      .build();
    let agent: ureq::Agent = config.into();

    let response = agent
      .run(outbound.body(body.as_slice()).map_err(|err| {
        ComponentError::transport_failed(format!("could not build the outbound request: {err}"))
      })?)
      .map_err(|err| ComponentError::transport_failed(err.to_string()))?;

    if !req.await_reply {
      return Ok(SendResult { reply: None });
    }
    Ok(SendResult {
      reply: Some(reply_parts(response)?),
    })
  }

  fn poll_inbound(&self, req: PollInbound) -> Result<PollInboundResult, ComponentError> {
    // Wait with the lock released: every other instance's `reply`, and this one's `stop`, need it.
    let server = {
      let mut instances = self.instances.lock().expect("instance lock poisoned");
      serve_instance(&mut instances, &req.instance, "transport/poll-inbound")?
        .server
        .clone()
    };
    let received = server
      .recv_timeout(Duration::from_millis(req.timeout_ms))
      .map_err(|err| ComponentError::transport_failed(err.to_string()))?;

    let Some(mut request) = received else {
      return Ok(PollInboundResult { inbound: None });
    };

    let parts = request_parts(&mut request);
    let mut instances = self.instances.lock().expect("instance lock poisoned");
    let Ok(instance) = serve_instance(&mut instances, &req.instance, "transport/poll-inbound") else {
      // Stopped while this request was arriving: nothing will ever reply to it, so answer it now
      // rather than leave the client's connection hanging.
      let _ = request.respond(tiny_http::Response::from_data(Vec::new()).with_status_code(503));
      return Ok(PollInboundResult { inbound: None });
    };
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
    let instance = serve_instance(&mut instances, &req.instance, "transport/reply")?;
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
    let body_slot = response_part.get("body");
    let body = match body_slot {
      Some(slot) => slot_bytes(slot)?,
      None => Vec::new(),
    };
    let mut response = tiny_http::Response::from_data(body).with_status_code(status);
    let mut declared_content_type = false;

    if let Some(headers_slot) = response_part.get("headers")
      && let Value::Object(headers) = &headers_slot.content
    {
      for (name, values) in headers {
        declared_content_type |= name.eq_ignore_ascii_case("content-type");
        for value in values.as_array().into_iter().flatten().filter_map(Value::as_str) {
          if let Ok(header) = tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response = response.with_header(header);
          }
        }
      }
    }
    // The body's own media type labels it unless the interaction declared a Content-Type itself —
    // the same rule the drive side applies to a request body.
    if !declared_content_type
      && let Some(content_type) = body_slot.and_then(|slot| slot.content_type.as_deref())
      && let Ok(header) = tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes())
    {
      response = response.with_header(header);
    }

    request
      .respond(response)
      .map_err(|err| ComponentError::transport_failed(err.to_string()))?;
    Ok(ReplyResult {})
  }

  fn dispose(&self, req: Dispose) -> Result<DisposeResult, ComponentError> {
    let mut instances = self.instances.lock().expect("instance lock poisoned");
    if let Some(Instance::Serve(instance)) = instances.get_mut(&req.instance) {
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

impl HttpTransport {
  /// The serve role (plan task 4.2): bind a real mock server, answer with the address it actually
  /// got — `port: 0` is the normal case, and the endpoint descriptor is how the host learns which
  /// port that was.
  fn start_serve(&self, req: Start) -> Result<StartResult, ComponentError> {
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
      Instance::Serve(ServeInstance {
        server: Arc::new(server),
        pending: HashMap::new(),
        next_event: 1,
      }),
    );
    Ok(StartResult { endpoint })
  }

  /// The drive role (plan task 5.1): nothing is bound and nothing is connected — `start` only
  /// records where this instance sends. The endpoint descriptor echoes the target back, so a host
  /// that let the component normalise the URL can see what it settled on (spec §5.1: an endpoint
  /// descriptor is never assumed to be a URL, but for HTTP it is one).
  fn start_drive(&self, req: Start) -> Result<StartResult, ComponentError> {
    let options = req.options.unwrap_or_default();
    let base_url = options
      .get("base-url")
      .and_then(Value::as_str)
      .ok_or_else(|| ComponentError::transport_failed("a drive-role HTTP instance needs options.base-url"))?
      .trim_end_matches('/')
      .to_string();

    let endpoint = serde_json::json!({ "kind": req.kind, "base-url": base_url });
    self
      .instances
      .lock()
      .expect("instance lock poisoned")
      .insert(req.instance, Instance::Drive(DriveInstance { base_url }));
    Ok(StartResult { endpoint })
  }
}
