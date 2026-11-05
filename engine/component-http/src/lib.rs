//! The built-in HTTP transport component (component-interfaces spec §5): the native binding for a
//! real mock HTTP server in the **serve** role (plan task 4.2) and a real HTTP client in the
//! **drive** role (plan task 5.1), in-tree but through the same interface a third-party transport
//! would use (ADR 0012). It is a matcher too ([`status`], ADR 0057): knowing what a status class is
//! and speaking HTTP are the same knowledge (spec §2.1).
//!
//! One component, two roles, five role-neutral primitives (spec §5.1): a served instance answers
//! `poll-inbound`/`reply`/`dispose`, a driven one answers `send`, and each answers
//! `operation-unsupported` for the other's — spec §5.2's rule that "no reply expected" and "reply
//! expected but absent" are different observations applies to whole operations too, so a driven
//! instance asked to poll says so by name rather than returning "nothing arrived" forever.
//!
//! The two halves are rewritten separately (staged plan §3). The drive half ([`drive`], S1.5) sends
//! through the engine's one HTTP client, which is async behind an executor seam and starts no
//! thread (ADR 0035). The serve half ([`serve`], S2.1) is hyper on a thread and runtime of its own
//! per served instance (ADR 0062 decision 5), serving HTTP/1.1 and HTTP/2.

mod drive;
mod serve;
pub mod status;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use drive::DriveInstance;
use pact_janus_kernel::component::{
  ComponentError, ContentSlots, Dispose, DisposeResult, ExchangeParts, Inbound, PollInbound,
  PollInboundResult, Reply, ReplyResult, Send, SendResult, SlotValue, Start, StartResult, Stop, StopResult,
  TransportComponent,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// One started instance, in whichever role `start` named (spec §5.1). The role is fixed at
/// `start` and never re-negotiated, which is what lets every later operation answer
/// `operation-unsupported` from the instance alone.
enum Instance {
  Serve(ServeInstance),
  /// Shared so that a `send` can wait on its provider without holding the lock over every
  /// instance.
  Drive(Arc<DriveInstance>),
}

struct ServeInstance {
  /// Its arrivals are shared so that `poll-inbound` can wait on them without holding the lock over
  /// every instance (phase-9 finding 27): a waiter clones them and lets go, and `stop` ends the
  /// server, which wakes it.
  server: serve::Server,
  pending: serve::Pending,
  next_event: u64,
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
pub(crate) fn slot_bytes(value: &SlotValue) -> Result<Vec<u8>, ComponentError> {
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
pub(crate) enum Names {
  FoldCase,
  AsSent,
}

/// Build a `{name: [values...]}` slot (headers, query — spec's own worked example shape, and
/// `legacy.rs`'s existing `query`/`headers` slot vocabulary) from possibly-repeated name/value
/// pairs, grouping names as it goes.
pub(crate) fn multi_map_slot<'a>(
  entries: impl Iterator<Item = (&'a str, &'a str)>,
  names: Names,
) -> SlotValue {
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

/// `{name: [values...]}` back to `name=value&name=value`, in the slot's own (sorted) order so the
/// same recorded request replays byte-identically every time.
pub(crate) fn query_string(slot: &SlotValue) -> String {
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

impl TransportComponent for HttpTransport {
  /// Bodies are content; method, path, query, status and the headers map are plain values
  /// (component-interfaces spec §5.5).
  fn content_slots(&self) -> ContentSlots {
    ContentSlots::from([
      ("request".to_string(), vec!["body".to_string()]),
      ("response".to_string(), vec!["body".to_string()]),
    ])
  }

  /// An exchange starts with a request and is answered with a response (component-interfaces spec
  /// §5.7): the names the engine reads instead of spelling them.
  fn exchange_parts(&self) -> ExchangeParts {
    ExchangeParts {
      initiating: "request".to_string(),
      answering: Some("response".to_string()),
    }
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
    // A `poll-inbound` may be waiting on this server right now. Stopping it closes its arrivals,
    // which wakes the waiter rather than let `stop` (and the `finalise` behind it) sit out the rest
    // of its timeout. An arrival still waiting for an answer is answered 503 as its sender drops.
    if let Some(Instance::Serve(mut instance)) = removed {
      instance.server.stop();
    }
    Ok(StopResult {})
  }

  /// `send` (spec §5.2) — the drive role's whole job: one request at the provider, its reply back
  /// as parts. `await-reply: false` still performs the request (a provider is not a broker; there
  /// is no way to speak HTTP without a response arriving) and reports no reply, which is the
  /// honest reading of "fire and forget" for this kind rather than an invented empty reply.
  fn send(&self, req: Send) -> Result<SendResult, ComponentError> {
    let drive = {
      let instances = self.instances.lock().expect("instance lock poisoned");
      match instances.get(&req.instance) {
        Some(Instance::Drive(drive)) => Arc::clone(drive),
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
    let reply = drive.send(request_part, req.timeout_ms.map(Duration::from_millis))?;
    if !req.await_reply {
      return Ok(SendResult { reply: None });
    }
    Ok(SendResult { reply: Some(reply) })
  }

  fn poll_inbound(&self, req: PollInbound) -> Result<PollInboundResult, ComponentError> {
    // Wait with the lock released: every other instance's `reply`, and this one's `stop`, need it.
    let arrivals = {
      let mut instances = self.instances.lock().expect("instance lock poisoned");
      Arc::clone(
        &serve_instance(&mut instances, &req.instance, "transport/poll-inbound")?
          .server
          .arrivals,
      )
    };
    let arrival = serve::next_arrival(&arrivals, Duration::from_millis(req.timeout_ms));
    let mut instances = self.instances.lock().expect("instance lock poisoned");
    let arrival = match arrival {
      Ok(Some(arrival)) => arrival,
      Ok(None) => return Ok(PollInboundResult { inbound: None }),
      // Woken by `stop`, this poll saw nothing arrive; the next one finds no instance.
      Err(serve::Stopped) if !instances.contains_key(&req.instance) => {
        return Ok(PollInboundResult { inbound: None });
      }
      Err(serve::Stopped) => {
        return Err(ComponentError::transport_failed("the server stopped on its own"));
      }
    };
    let Ok(instance) = serve_instance(&mut instances, &req.instance, "transport/poll-inbound") else {
      // Stopped while this request was arriving: nothing will ever reply to it, so answer it now
      // rather than leave the client's connection hanging.
      let _ = arrival.answer.send(serve::status_only(503));
      return Ok(PollInboundResult { inbound: None });
    };
    let event = format!("e-{}", instance.next_event);
    instance.next_event += 1;
    instance.pending.insert(event.clone(), arrival.answer);

    Ok(PollInboundResult {
      inbound: Some(Inbound {
        event,
        parts: arrival.parts,
        expects_reply: true,
      }),
    })
  }

  fn reply(&self, req: Reply) -> Result<ReplyResult, ComponentError> {
    let mut instances = self.instances.lock().expect("instance lock poisoned");
    let instance = serve_instance(&mut instances, &req.instance, "transport/reply")?;
    let answer = instance.pending.remove(&req.event).ok_or_else(|| {
      ComponentError::transport_failed(format!("no pending arrival for event '{}'", req.event))
    })?;
    let response_part = req.parts.get("response").ok_or_else(|| {
      ComponentError::transport_failed("reply parts must carry a 'response' part for an HTTP arrival")
    })?;
    let response = match serve::response(response_part) {
      Ok(response) => response,
      Err(err) => {
        // The arrival is still answered, so its connection does not hang on a reply that failed.
        let _ = answer.send(serve::status_only(500));
        return Err(err);
      }
    };
    answer
      .send(response)
      .map_err(|_| ComponentError::transport_failed("the client went away before its reply"))?;
    Ok(ReplyResult {})
  }

  fn dispose(&self, req: Dispose) -> Result<DisposeResult, ComponentError> {
    let mut instances = self.instances.lock().expect("instance lock poisoned");
    if let Some(Instance::Serve(instance)) = instances.get_mut(&req.instance)
      && let Some(answer) = instance.pending.remove(&req.event)
    {
      let _ = answer.send(serve::disposed(&req.disposition, req.details.as_ref()));
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

    let port = u16::try_from(port)
      .map_err(|_| ComponentError::invalid_config("http", format!("{port} is not a port")))?;
    let server = serve::Server::start(&req.instance, host, port)?;
    let bound_port = server.address.port();

    // An IPv6 address is bracketed in a URL (RFC 3986 §3.2.2).
    let authority = if host.contains(':') {
      format!("[{host}]")
    } else {
      host.to_string()
    };
    let endpoint = serde_json::json!({
      "kind": "http",
      "host": host,
      "port": bound_port,
      "base-url": format!("http://{authority}:{bound_port}"),
    });

    self.instances.lock().expect("instance lock poisoned").insert(
      req.instance,
      Instance::Serve(ServeInstance {
        server,
        pending: serve::Pending::new(),
        next_event: 1,
      }),
    );
    Ok(StartResult { endpoint })
  }

  /// The drive role (plan task 5.1): nothing is bound and nothing is connected — `start` reads the
  /// options and builds the client, so options it cannot act on fail the run before it starts. The
  /// endpoint descriptor echoes the target back, so a host that let the component normalise the URL
  /// can see what it settled on (spec §5.1: an endpoint descriptor is never assumed to be a URL, but
  /// for HTTP it is one).
  fn start_drive(&self, req: Start) -> Result<StartResult, ComponentError> {
    let none = serde_json::Map::new();
    let options = match &req.options {
      None => &none,
      Some(Value::Object(options)) => options,
      Some(_) => {
        return Err(ComponentError::invalid_config(
          "http",
          "the drive options must be an object",
        ));
      }
    };
    let drive = DriveInstance::from_options(options)?;
    let endpoint = serde_json::json!({ "kind": req.kind, "base-url": drive.base_url() });
    self
      .instances
      .lock()
      .expect("instance lock poisoned")
      .insert(req.instance, Instance::Drive(Arc::new(drive)));
    Ok(StartResult { endpoint })
  }
}
