//! The top-level dispatcher: one `Engine` per pipe (engine-protocol spec §3), owning handshake
//! state and the session store. `dispatch` is pipe 3.3's entry point (native/in-process — "no
//! additional rules" beyond the call pipe itself) — JSON bytes in, JSON bytes out — deliberately
//! shaped so the subprocess and WASM pipes (§3.1–3.2) can wrap it later without changing it.

use super::consumer_session::{AddInteraction, Create, Finalise, ServeVariant, Variants};
use super::frame::{EngineError, RequestFrame, ResponseFrame};
use super::hello::{self, Hello};
use super::session::{ServeVariantError, SessionStore, VariantsError};
use crate::variant::VariantError;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::panic::{self, AssertUnwindSafe};

pub struct Engine {
  hello_done: bool,
  sessions: SessionStore,
}

impl Default for Engine {
  fn default() -> Self {
    Self::new()
  }
}

impl Engine {
  pub fn new() -> Self {
    Engine {
      hello_done: false,
      sessions: SessionStore::default(),
    }
  }

  /// One frame in, one frame out (spec §4). Never panics: the dispatch boundary catches panics
  /// and converts them to `internal` (§10.1) — a panic reaching a caller would itself be a bug.
  pub fn dispatch(&mut self, bytes: &[u8]) -> Vec<u8> {
    let response = self.dispatch_bytes(bytes);
    serde_json::to_vec(&response).expect("ResponseFrame is always representable as JSON")
  }

  fn dispatch_bytes(&mut self, bytes: &[u8]) -> ResponseFrame {
    let raw: Value = match serde_json::from_slice(bytes) {
      Ok(v) => v,
      Err(err) => {
        return ResponseFrame::err(
          "",
          EngineError::malformed_frame(format!("frame body is not valid JSON ({err})")),
        );
      }
    };
    // Recovered defensively so a schema-violating frame still gets a correlated error where
    // possible (spec §4.4: an empty id means pipe-level, uncorrelatable).
    let id = raw
      .get("id")
      .and_then(Value::as_str)
      .unwrap_or_default()
      .to_string();
    if raw.get("type").and_then(Value::as_str) != Some("request") {
      return ResponseFrame::err(
        id,
        EngineError::malformed_frame("frame 'type' must be 'request' on this pipe"),
      );
    }

    let mut de = serde_json::Deserializer::from_slice(bytes);
    let request: RequestFrame = match serde_path_to_error::deserialize(&mut de) {
      Ok(request) => request,
      Err(err) => return ResponseFrame::err(id, EngineError::malformed_frame(err.to_string())),
    };

    match panic::catch_unwind(AssertUnwindSafe(|| self.dispatch_frame(request))) {
      Ok(response) => response,
      Err(_) => ResponseFrame::err(
        id,
        EngineError::internal("the engine encountered an internal error handling this request"),
      ),
    }
  }

  fn dispatch_frame(&mut self, request: RequestFrame) -> ResponseFrame {
    let RequestFrame { id, op, body, .. } = request;

    if op != "engine/hello" && !self.hello_done {
      return ResponseFrame::err(id, EngineError::handshake_required());
    }

    match op.as_str() {
      "engine/hello" => self.handle_hello(id, body),
      "consumer-session/create" => self.handle_create(id, body),
      "consumer-session/add-interaction" => self.handle_add_interaction(id, body),
      "consumer-session/variants" => self.handle_variants(id, body),
      "consumer-session/serve-variant" => self.handle_serve_variant(id, body),
      "consumer-session/finalise" => self.handle_finalise(id, body),
      other => ResponseFrame::err(id, EngineError::operation_unsupported(other)),
    }
  }

  fn handle_hello(&mut self, id: String, body: Value) -> ResponseFrame {
    let hello: Hello = match parse_body(body) {
      Ok(hello) => hello,
      Err(err) => return ResponseFrame::err(id, err),
    };
    match hello::negotiate(&hello) {
      Some(_version) => {
        self.hello_done = true;
        ResponseFrame::ok(id, hello::result())
      }
      None => ResponseFrame::err(
        id,
        EngineError::protocol_version_unsupported(&[crate::PROTOCOL_VERSION]),
      ),
    }
  }

  fn handle_create(&mut self, id: String, body: Value) -> ResponseFrame {
    let create: Create = match parse_body(body) {
      Ok(create) => create,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let session = self.sessions.create(
      create.config.consumer,
      create.config.provider,
      create.config.policy,
    );
    ResponseFrame::ok(id, json!({ "session": session }))
  }

  fn handle_add_interaction(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: AddInteraction = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let Some(session) = self.sessions.get_mut(&req.session) else {
      return ResponseFrame::err(id, EngineError::session_not_found(&req.session));
    };
    match session.add_interaction(&req.interaction) {
      Ok(handle) => ResponseFrame::ok(id, json!({ "handle": handle })),
      Err(err) => ResponseFrame::err(id, EngineError::interaction_invalid(&err.problems)),
    }
  }

  fn handle_variants(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: Variants = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let Some(session) = self.sessions.get_mut(&req.session) else {
      return ResponseFrame::err(id, EngineError::session_not_found(&req.session));
    };
    match session.variants(&req.handle, req.policy.as_ref()) {
      Ok(result) => ResponseFrame::ok(id, result),
      Err(VariantsError::HandleNotFound) => {
        ResponseFrame::err(id, EngineError::handle_not_found(&req.handle))
      }
      Err(VariantsError::Variant(VariantError::InvalidPolicy(problems))) => {
        ResponseFrame::err(id, EngineError::invalid_policy(&problems))
      }
      Err(VariantsError::Variant(VariantError::BudgetExceeded {
        space,
        selected,
        budget,
        dimensions,
      })) => ResponseFrame::err(
        id,
        EngineError::variant_budget_exceeded(space, selected, budget, &dimensions),
      ),
    }
  }

  fn handle_serve_variant(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: ServeVariant = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let Some(session) = self.sessions.get_mut(&req.session) else {
      return ResponseFrame::err(id, EngineError::session_not_found(&req.session));
    };
    match session.serve_variant(&req.handle, &req.variant) {
      Ok(()) => ResponseFrame::ok(id, json!({})),
      Err(ServeVariantError::HandleNotFound) => {
        ResponseFrame::err(id, EngineError::handle_not_found(&req.handle))
      }
      Err(ServeVariantError::VariantNotFound { selection }) => {
        ResponseFrame::err(id, EngineError::variant_not_found(&req.variant, &selection))
      }
    }
  }

  fn handle_finalise(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: Finalise = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let Some(session) = self.sessions.end(&req.session) else {
      return ResponseFrame::err(id, EngineError::session_not_found(&req.session));
    };
    ResponseFrame::ok(id, json!({ "results": session.results() }))
  }
}

/// An operation's request body, governed by the operation's schema (spec §4.1): a body that
/// doesn't match it is `malformed-frame`, same as an envelope violation (§4.4).
fn parse_body<T: DeserializeOwned>(body: Value) -> Result<T, EngineError> {
  serde_path_to_error::deserialize(body).map_err(|err| EngineError::malformed_frame(err.to_string()))
}
