//! The serve role's server (ADR 0062 decision 5): hyper 1 through hyper-util's `auto` builder,
//! serving HTTP/1.1 and HTTP/2 with prior knowledge, as today's mock server does.
//!
//! Each served instance has a thread of its own, running a current-thread tokio runtime that is
//! nobody else's. The component starts it itself: it is native only and outside the kernel. A
//! request is collected whole and put on a channel, where `poll-inbound` waits for it within its
//! timeout. Its answer goes back through a one-shot channel kept per event, which `reply` and
//! `dispose` complete. `stop` ends the runtime, which closes the channel and so wakes a waiting
//! `poll-inbound`.

use crate::{Names, multi_map_slot, text_slot};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use pact_janus_kernel::component::{ComponentError, Part, Parts, SlotValue};
use serde_json::Value;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tokio::sync::oneshot;

/// The answer a served request is waiting for.
pub(crate) type Answer = http::Response<Full<Bytes>>;

/// One request the server collected, with where its answer goes.
pub(crate) struct Arrival {
  pub parts: Parts,
  pub answer: oneshot::Sender<Answer>,
}

/// A bound, running server: its address, the arrivals it collects, and how to stop it.
pub(crate) struct Server {
  pub address: SocketAddr,
  /// Shared so that `poll-inbound` can wait on it with the component's lock released.
  pub arrivals: Arc<Mutex<mpsc::Receiver<Arrival>>>,
  shutdown: Option<oneshot::Sender<()>>,
  thread: Option<JoinHandle<()>>,
}

impl Server {
  /// Bind `host:port` and start serving on a thread named for `instance`. Binding happens here, on
  /// the caller's thread, so a port in use fails `start` and `port: 0` is answered with the port it
  /// got.
  pub fn start(instance: &str, host: &str, port: u16) -> Result<Server, ComponentError> {
    let listener = std::net::TcpListener::bind((host, port))
      .map_err(|err| ComponentError::transport_failed(format!("could not bind {host}:{port}: {err}")))?;
    let address = listener
      .local_addr()
      .map_err(|err| ComponentError::transport_failed(format!("could not read the bound address: {err}")))?;
    listener
      .set_nonblocking(true)
      .map_err(|err| ComponentError::transport_failed(format!("could not serve {host}:{port}: {err}")))?;

    let (arrivals_tx, arrivals_rx) = mpsc::channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_io()
      .enable_time()
      .build()
      .map_err(|err| {
        ComponentError::transport_failed(format!("could not start the server's runtime: {err}"))
      })?;
    let thread = thread::Builder::new()
      .name(format!("janus http serve {instance}"))
      .spawn(move || runtime.block_on(serve(listener, arrivals_tx, shutdown_rx)))
      .map_err(|err| {
        ComponentError::transport_failed(format!("could not start the server's thread: {err}"))
      })?;

    Ok(Server {
      address,
      arrivals: Arc::new(Mutex::new(arrivals_rx)),
      shutdown: Some(shutdown_tx),
      thread: Some(thread),
    })
  }

  /// Stop accepting and end every connection, then wait for the thread, so the port is free when
  /// this returns.
  pub fn stop(&mut self) {
    if let Some(shutdown) = self.shutdown.take() {
      let _ = shutdown.send(());
    }
    if let Some(thread) = self.thread.take()
      && thread.join().is_err()
    {
      tracing::error!(address = %self.address, "the HTTP server's thread panicked");
    }
  }
}

impl Drop for Server {
  fn drop(&mut self) {
    self.stop();
  }
}

/// Wait up to `timeout` for the next arrival. `Ok(None)` is a timeout; `Err(Stopped)` means the
/// server has ended, which is what `stop` looks like to a waiter.
pub(crate) fn next_arrival(
  arrivals: &Mutex<mpsc::Receiver<Arrival>>,
  timeout: Duration,
) -> Result<Option<Arrival>, Stopped> {
  let arrivals = arrivals.lock().expect("arrivals lock poisoned");
  match arrivals.recv_timeout(timeout) {
    Ok(arrival) => Ok(Some(arrival)),
    Err(RecvTimeoutError::Timeout) => Ok(None),
    Err(RecvTimeoutError::Disconnected) => Err(Stopped),
  }
}

/// The server behind a waiter ended.
pub(crate) struct Stopped;

async fn serve(
  listener: std::net::TcpListener,
  arrivals: mpsc::Sender<Arrival>,
  mut shutdown: oneshot::Receiver<()>,
) {
  let listener = match tokio::net::TcpListener::from_std(listener) {
    Ok(listener) => listener,
    Err(err) => {
      tracing::error!(error = %err, "the HTTP server could not take its socket");
      return;
    }
  };
  loop {
    tokio::select! {
      _ = &mut shutdown => break,
      accepted = listener.accept() => {
        let Ok((stream, _)) = accepted else { continue };
        let arrivals = arrivals.clone();
        tokio::spawn(async move {
          let service = service_fn(move |request| answer(request, arrivals.clone()));
          if let Err(err) = auto::Builder::new(TokioExecutor::new())
            .serve_connection(TokioIo::new(stream), service)
            .await
          {
            tracing::debug!(error = %err, "a connection to the mock ended with an error");
          }
        });
      }
    }
  }
  // Returning drops the runtime's tasks: every connection closes, and with the last sender gone a
  // waiting `poll-inbound` is told the server stopped.
}

/// Hand one request to whoever polls, and wait for its answer.
async fn answer(
  request: http::Request<Incoming>,
  arrivals: mpsc::Sender<Arrival>,
) -> Result<Answer, Infallible> {
  let (head, body) = request.into_parts();
  let body = match body.collect().await {
    Ok(collected) => collected.to_bytes(),
    Err(err) => {
      tracing::debug!(error = %err, "the mock could not read a request's body");
      return Ok(status_only(400));
    }
  };
  let (answer, answered) = oneshot::channel();
  if arrivals
    .send(Arrival {
      parts: request_parts(&head, &body),
      answer,
    })
    .is_err()
  {
    return Ok(status_only(503));
  }
  // An arrival dropped without an answer (the instance stopped while it waited) is still answered.
  Ok(answered.await.unwrap_or_else(|_| status_only(503)))
}

/// An answer with a status and nothing else.
pub(crate) fn status_only(status: u16) -> Answer {
  let mut response = http::Response::new(Full::new(Bytes::new()));
  *response.status_mut() =
    http::StatusCode::from_u16(status).unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR);
  response
}

/// HTTP/2 and HTTP/3 let a client split `Cookie` into crumbs, one field line each, which are joined
/// with `"; "` before the value leaves the protocol (RFC 9113 §8.2.3, RFC 9114 §4.2.1). A request's
/// `cookie` then reads the same in every version (ADR 0082 decision 1). The joined line takes the
/// first crumb's place.
fn joined_cookie_crumbs(headers: Vec<(String, String)>) -> Vec<(String, String)> {
  let crumbs: Vec<&str> = headers
    .iter()
    .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
    .map(|(_, value)| value.as_str())
    .collect();
  if crumbs.len() < 2 {
    return headers;
  }
  let joined = crumbs.join("; ");
  let mut out = Vec::with_capacity(headers.len());
  let mut placed = false;
  for (name, value) in headers {
    if !name.eq_ignore_ascii_case("cookie") {
      out.push((name, value));
    } else if !placed {
      out.push((name, joined.clone()));
      placed = true;
    }
  }
  out
}

/// A request as the `request` part (component-interfaces spec §5.5): method, path and query as
/// sent, headers by lower-cased name, and the body's octets.
fn request_parts(head: &http::request::Parts, body: &Bytes) -> Parts {
  let path = head.uri.path().to_string();
  let query = head.uri.query().unwrap_or_default();

  let headers: Vec<(String, String)> = head
    .headers
    .iter()
    .map(|(name, value)| {
      (
        name.as_str().to_string(),
        String::from_utf8_lossy(value.as_bytes()).into_owned(),
      )
    })
    .collect();
  let headers = match head.version {
    http::Version::HTTP_2 | http::Version::HTTP_3 => joined_cookie_crumbs(headers),
    _ => headers,
  };
  let headers_slot = multi_map_slot(
    headers
      .iter()
      .map(|(name, value)| (name.as_str(), value.as_str())),
    Names::FoldCase,
  );

  // Decoded, as a pact records them and as `query_string` encodes them on the way out.
  let query_pairs: Vec<(String, String)> = form_urlencoded::parse(query.as_bytes())
    .map(|(name, value)| (name.into_owned(), value.into_owned()))
    .collect();
  let query_slot = multi_map_slot(
    query_pairs
      .iter()
      .map(|(name, value)| (name.as_str(), value.as_str())),
    Names::AsSent,
  );

  let content_type = head
    .headers
    .get(http::header::CONTENT_TYPE)
    .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned());
  let body_slot = SlotValue {
    content: Value::String(BASE64.encode(body)),
    encoded: Some("base64".to_string()),
    content_type,
  };

  let mut request_part: Part = Part::new();
  request_part.insert("method".to_string(), text_slot(head.method.as_str()));
  request_part.insert("path".to_string(), text_slot(path));
  request_part.insert("query".to_string(), query_slot);
  request_part.insert("headers".to_string(), headers_slot);
  request_part.insert("body".to_string(), body_slot);

  let mut parts: Parts = Parts::new();
  parts.insert("request".to_string(), request_part);
  parts
}

/// The `response` part as an answer: its status (200 when it has none), each header value, and its
/// body. The body's own media type labels it unless the interaction declared a `Content-Type`
/// itself, the rule the drive side applies to a request body.
pub(crate) fn response(response_part: &Part) -> Result<Answer, ComponentError> {
  let status = response_part
    .get("status")
    .and_then(|slot| slot.content.as_u64())
    .unwrap_or(200);
  let status = u16::try_from(status)
    .ok()
    .and_then(|status| http::StatusCode::from_u16(status).ok())
    .ok_or_else(|| ComponentError::transport_failed(format!("{status} is not an HTTP status")))?;
  let body_slot = response_part.get("body");
  let body = match body_slot {
    Some(slot) => crate::slot_bytes(slot)?,
    None => Vec::new(),
  };

  let mut answer = http::Response::new(Full::new(Bytes::from(body)));
  *answer.status_mut() = status;
  let headers = answer.headers_mut();
  if let Some(headers_slot) = response_part.get("headers")
    && let Value::Object(declared) = &headers_slot.content
  {
    for (name, values) in declared {
      let Ok(name) = http::HeaderName::from_bytes(name.as_bytes()) else {
        continue;
      };
      // The body framed here decides these; a recorded value for another body would break it.
      if name == http::header::CONTENT_LENGTH || name == http::header::TRANSFER_ENCODING {
        continue;
      }
      for value in values.as_array().into_iter().flatten().filter_map(Value::as_str) {
        if let Ok(value) = http::HeaderValue::from_bytes(value.as_bytes()) {
          headers.append(name.clone(), value);
        }
      }
    }
  }
  if !headers.contains_key(http::header::CONTENT_TYPE)
    && let Some(content_type) = body_slot.and_then(|slot| slot.content_type.as_deref())
    && let Ok(value) = http::HeaderValue::from_bytes(content_type.as_bytes())
  {
    headers.insert(http::header::CONTENT_TYPE, value);
  }
  Ok(answer)
}

/// The answer to an arrival disposed of without a reply. HTTP has no disposition (spec §5.3), but a
/// connection must not hang, so it gets a 500. One the engine rejected because it did not match
/// carries every mismatch as JSON, so the consumer's own client sees why (kernel-boundary review
/// finding 9).
pub(crate) fn disposed(disposition: &str, details: Option<&Value>) -> Answer {
  let mismatches = (disposition == "reject")
    .then(|| details.and_then(|details| details.get("mismatches")))
    .flatten();
  let Some(mismatches) = mismatches else {
    return status_only(500);
  };
  let mut answer = http::Response::new(Full::new(Bytes::from(
    serde_json::to_vec(mismatches).unwrap_or_default(),
  )));
  *answer.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
  answer.headers_mut().insert(
    http::header::CONTENT_TYPE,
    http::HeaderValue::from_static("application/json"),
  );
  answer
}

/// Events waiting for an answer, by the id `poll-inbound` gave them.
pub(crate) type Pending = HashMap<String, oneshot::Sender<Answer>>;
