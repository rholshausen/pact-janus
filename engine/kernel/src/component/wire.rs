//! The content and matcher interfaces as the component pipe carries them (component-interfaces spec
//! §3, §6, §7): the native binding's in-memory documents (spec §9.1) written as the frames' JSON and
//! read back, at both ends of the pipe.
//!
//! - [`PipeClient`] is the engine's end. It implements [`ContentComponent`] and [`MatcherComponent`]
//!   over any [`Pipe`], writing each request as its operation's document and reading the result. The
//!   WASM binding is a `PipeClient` over a wasmtime instance (`engine/component-host`).
//! - [`serve`] and [`serve_frame`] are the component's end, for a native component: read a request
//!   document, call the component, write its result.
//! - [`InProcess`] joins the two through frame bytes: a native component reached exactly as a WASM
//!   guest is, which is the serialising in-process binding spec §9.4 runs the conformance corpus
//!   through (staged plan S5.10, ADR 0105). Its client half is the WASM binding's own, not a copy, so
//!   a corpus that passes through it holds the codec every out-of-tree component is reached by.

use super::{
  Apply, ApplyResult, Compare, CompareResult, Compile, CompileResult, ComponentError, ContentComponent,
  Decode, DecodeResult, Detect, DetectResult, Embedded, EmbeddedDecode, EmbeddedEncode, Encode, EncodeResult,
  Label, LabelResult, MatcherComponent, Prefix, PrefixResult, SlotValue, media_type_matches,
};
use crate::plan::RuntimeValue;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// One end of a component pipe, as the engine sees it: an operation and its request document in, the
/// result's `ok` document or the component's error out (spec §3.1). How the frame travels, and which
/// handshake precedes it, is the pipe's business.
pub trait Pipe: Send + Sync {
  /// Who answers, for the engine's logs.
  fn name(&self) -> &str;
  fn call(&self, op: &str, body: Value) -> Result<Value, ComponentError>;
}

impl<P: Pipe + ?Sized> Pipe for Arc<P> {
  fn name(&self) -> &str {
    (**self).name()
  }

  fn call(&self, op: &str, body: Value) -> Result<Value, ComponentError> {
    (**self).call(op, body)
  }
}

/// The content and matcher interfaces over a [`Pipe`] (spec §6, §7): the kernel's in-memory documents
/// turned into the frames' JSON and back. The document model's bytes travel as `encoded: base64`
/// (ADR 0006).
pub struct PipeClient<P> {
  pipe: P,
  /// The `media-type` of every `content-types` entry the handshake declared: what
  /// [`ContentComponent::handles`] answers from.
  media_types: Vec<String>,
}

impl<P: Pipe> PipeClient<P> {
  pub fn new(pipe: P, media_types: Vec<String>) -> Self {
    PipeClient { pipe, media_types }
  }

  pub fn pipe(&self) -> &P {
    &self.pipe
  }
}

/// The `media-type` of every `content-types` entry a handshake declares (spec §3.3).
pub fn declared_media_types(hello: &Value) -> Vec<String> {
  hello
    .pointer("/contributes/content-types")
    .and_then(Value::as_array)
    .into_iter()
    .flatten()
    .filter_map(|entry| {
      entry
        .get("media-type")
        .and_then(Value::as_str)
        .map(str::to_string)
    })
    .collect()
}

impl<P: Pipe> ContentComponent for PipeClient<P> {
  fn handles(&self, content_type: &str) -> bool {
    self
      .media_types
      .iter()
      .any(|pattern| media_type_matches(pattern, content_type))
  }

  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    let result = self.pipe.call("content/decode", decode_json(req)?)?;
    let result = read_decode_result(&result)?;
    if !result.degradations.is_empty() {
      tracing::debug!(component = %self.pipe.name(), degradations = ?result.degradations, "decode degradations");
    }
    Ok(result)
  }

  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    let result = self.pipe.call("content/encode", encode_json(req)?)?;
    read_encode_result(&result)
  }

  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError> {
    let path = req.path.clone();
    let result = self.pipe.call(
      "content/compile",
      json!({ "content-type": req.content_type, "shape": req.shape, "path": req.path }),
    )?;
    // Success is the whole answer (spec §6.3). A fragment, which a component built before ADR 0100
    // may still return, is not read: say so where someone debugging that component will look.
    if result.get("fragment").is_some() {
      tracing::warn!(component = %self.pipe.name(), %path, "the component returned a plan fragment, which this engine does not read (ADR 0100); declare substitutions instead");
    }
    Ok(CompileResult::default())
  }

  fn detect(&self, req: Detect) -> Result<DetectResult, ComponentError> {
    let mut body = json!({ "value": slot_json(&req.value)? });
    if let Some(hint) = req.hint {
      body["hint"] = json!(hint);
    }
    let result = self.pipe.call("content/detect", body)?;
    Ok(DetectResult {
      media_type: optional_string(&result, "media-type"),
      confidence: result.get("confidence").and_then(Value::as_f64),
    })
  }

  fn label(&self, req: Label) -> Result<LabelResult, ComponentError> {
    let result = self.pipe.call(
      "content/label",
      json!({ "content-type": req.content_type, "value": slot_json(&req.value)?, "at": req.at }),
    )?;
    Ok(LabelResult {
      media_type: optional_string(&result, "media-type"),
    })
  }
}

/// The matcher interface over the pipe: `apply` (spec §7.1), and the two operations the subsumption
/// walk asks (ADR 0102), `compare` (§7.5) and `prefix` (§7.6). An operator's comparability is not asked
/// here: the engine reads it from the handshake, whichever binding loaded the component.
impl<P: Pipe> MatcherComponent for PipeClient<P> {
  fn apply(&self, req: Apply) -> Result<ApplyResult, ComponentError> {
    let mut body = json!({ "action": req.action, "values": req.values });
    if let Some(config) = req.config {
      body["config"] = config;
    }
    let result = self.pipe.call("matcher/apply", body)?;
    let results = result
      .get("results")
      .and_then(Value::as_array)
      .cloned()
      .ok_or_else(|| ComponentError::synthesised("component-malformed", "apply returned no results"))?;
    Ok(ApplyResult { results })
  }

  fn compare(&self, req: Compare) -> Result<CompareResult, ComponentError> {
    let result = self.pipe.call(
      "matcher/compare",
      json!({ "operator": req.operator, "provider": req.provider, "consumer": req.consumer }),
    )?;
    let verdict = result
      .get("verdict")
      .and_then(Value::as_str)
      .ok_or_else(|| ComponentError::synthesised("component-malformed", "compare returned no verdict"))?;
    Ok(CompareResult {
      verdict: verdict.to_string(),
      reason: optional_string(&result, "reason"),
    })
  }

  fn prefix(&self, req: Prefix) -> Result<PrefixResult, ComponentError> {
    let result = self.pipe.call(
      "matcher/prefix",
      json!({ "operator": req.operator, "node": req.node }),
    )?;
    Ok(PrefixResult {
      prefix: optional_string(&result, "prefix"),
    })
  }
}

/// The transport interface over a [`Pipe`] (spec §5, `schemas/v1/transport.schema.json`): the five
/// primitives and a disposition, written as their documents and read back, with what the kind declares
/// read from the handshake's `contributes.transports` (spec §3.3). The subprocess binding's transport
/// (ADR 0108 decision 7); the WASM binding has none (spec §13).
///
/// The kernel asks a transport for its declarations, not a kind, so a component contributing several
/// kinds is answered with the first kind's `exchange-parts` and the union of the rest.
pub struct TransportClient<P> {
  pipe: P,
  content_slots: super::ContentSlots,
  exchange_parts: super::ExchangeParts,
  credential_members: crate::redaction::CredentialMembers,
  outcome_slots: super::OutcomeSlots,
}

impl<P: Pipe> TransportClient<P> {
  /// A client for the transports `hello` contributes, or why it cannot be one: a transport the engine
  /// serves or drives declares which part starts an exchange (spec §5.7).
  pub fn new(pipe: P, hello: &Value) -> Result<Self, String> {
    let kinds: Vec<&Value> = hello
      .pointer("/contributes/transports")
      .and_then(Value::as_array)
      .map(|kinds| kinds.iter().collect())
      .unwrap_or_default();
    let Some(first) = kinds.first() else {
      return Err("it implements 'transport' and contributes no transport kind".to_string());
    };
    let exchange_parts = match first.get("exchange-parts") {
      Some(parts) => super::ExchangeParts {
        initiating: parts
          .get("initiating")
          .and_then(Value::as_str)
          .filter(|name| !name.is_empty())
          .ok_or("its transport's 'exchange-parts' names no 'initiating' part")?
          .to_string(),
        answering: optional_string(parts, "answering"),
      },
      None => {
        return Err(format!(
          "its transport kind '{}' declares no 'exchange-parts' (component-interfaces spec §5.7)",
          first.get("kind").and_then(Value::as_str).unwrap_or("?")
        ));
      }
    };
    let mut content_slots = super::ContentSlots::new();
    let mut outcome_slots = super::OutcomeSlots::new();
    let mut credential_members = crate::redaction::CredentialMembers::new();
    for kind in &kinds {
      union_slots(&mut content_slots, kind.get("content-slots"));
      union_slots(&mut outcome_slots, kind.get("outcome-slots"));
      for (part, slots) in kind
        .get("credential-members")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
      {
        let part = credential_members.entry(part.clone()).or_default();
        union_slots(part, Some(slots));
      }
    }
    Ok(TransportClient {
      pipe,
      content_slots,
      exchange_parts,
      credential_members,
      outcome_slots,
    })
  }

  pub fn pipe(&self) -> &P {
    &self.pipe
  }
}

/// `name -> [names]`, merged into `into` without repeating a name.
fn union_slots(into: &mut std::collections::BTreeMap<String, Vec<String>>, from: Option<&Value>) {
  for (part, names) in from.and_then(Value::as_object).into_iter().flatten() {
    let entry = into.entry(part.clone()).or_default();
    for name in names.as_array().into_iter().flatten().filter_map(Value::as_str) {
      if !entry.iter().any(|known| known == name) {
        entry.push(name.to_string());
      }
    }
  }
}

fn parts_of(value: Option<&Value>, what: &str) -> Result<super::Parts, ComponentError> {
  serde_json::from_value(value.cloned().unwrap_or(Value::Null)).map_err(|err| {
    ComponentError::synthesised(
      "component-malformed",
      format!("the component answered with {what} that are not parts: {err}"),
    )
  })
}

fn parts_json(parts: &super::Parts) -> Result<Value, ComponentError> {
  serde_json::to_value(parts).map_err(|err| ComponentError::internal(err.to_string()))
}

impl<P: Pipe> super::TransportComponent for TransportClient<P> {
  fn content_slots(&self) -> super::ContentSlots {
    self.content_slots.clone()
  }

  fn exchange_parts(&self) -> super::ExchangeParts {
    self.exchange_parts.clone()
  }

  fn credential_members(&self) -> crate::redaction::CredentialMembers {
    self.credential_members.clone()
  }

  fn outcome_slots(&self) -> super::OutcomeSlots {
    self.outcome_slots.clone()
  }

  fn start(&self, req: super::Start) -> Result<super::StartResult, ComponentError> {
    let mut body = json!({ "instance": req.instance, "kind": req.kind, "role": req.role });
    if let Some(options) = req.options {
      body["options"] = options;
    }
    let result = self.pipe.call("transport/start", body)?;
    let endpoint = result
      .get("endpoint")
      .cloned()
      .ok_or_else(|| ComponentError::synthesised("component-malformed", "start returned no endpoint"))?;
    Ok(super::StartResult { endpoint })
  }

  fn stop(&self, req: super::Stop) -> Result<super::StopResult, ComponentError> {
    self
      .pipe
      .call("transport/stop", json!({ "instance": req.instance }))?;
    Ok(super::StopResult {})
  }

  fn send(&self, req: super::Send) -> Result<super::SendResult, ComponentError> {
    let mut body =
      json!({ "instance": req.instance, "parts": parts_json(&req.parts)?, "await-reply": req.await_reply });
    if let Some(timeout) = req.timeout_ms {
      body["timeout-ms"] = json!(timeout);
    }
    let result = self.pipe.call("transport/send", body)?;
    let reply = match result.get("reply") {
      None | Some(Value::Null) => None,
      Some(reply) => Some(parts_of(Some(reply), "a reply's parts")?),
    };
    Ok(super::SendResult { reply })
  }

  fn poll_inbound(&self, req: super::PollInbound) -> Result<super::PollInboundResult, ComponentError> {
    let result = self.pipe.call(
      "transport/poll-inbound",
      json!({ "instance": req.instance, "timeout-ms": req.timeout_ms }),
    )?;
    let inbound = match result.get("inbound") {
      None | Some(Value::Null) => None,
      Some(inbound) => Some(super::Inbound {
        event: inbound
          .get("event")
          .and_then(Value::as_str)
          .ok_or_else(|| ComponentError::synthesised("component-malformed", "an inbound with no event"))?
          .to_string(),
        parts: parts_of(inbound.get("parts"), "an inbound's parts")?,
        expects_reply: inbound
          .get("expects-reply")
          .and_then(Value::as_bool)
          .unwrap_or(false),
      }),
    };
    Ok(super::PollInboundResult { inbound })
  }

  fn reply(&self, req: super::Reply) -> Result<super::ReplyResult, ComponentError> {
    self.pipe.call(
      "transport/reply",
      json!({ "instance": req.instance, "event": req.event, "parts": parts_json(&req.parts)? }),
    )?;
    Ok(super::ReplyResult {})
  }

  fn dispose(&self, req: super::Dispose) -> Result<super::DisposeResult, ComponentError> {
    let mut body = json!({ "instance": req.instance, "event": req.event, "disposition": req.disposition });
    if let Some(details) = req.details {
      body["details"] = details;
    }
    self.pipe.call("transport/dispose", body)?;
    Ok(super::DisposeResult {})
  }
}

// --- the engine's end: requests written, results read ---

fn decode_json(req: Decode) -> Result<Value, ComponentError> {
  let mut body = json!({ "content-type": req.content_type, "value": slot_json(&req.value)? });
  if let Some(options) = req.options {
    body["options"] = options;
  }
  // What the registry made of the content it asked for (spec §6.6, ADR 0103), on the repeated request.
  if let Some(embedded) = req.embedded {
    body["embedded"] = answers_json(embedded, |result| {
      let mut answer = tagged_document(&result.document, "document");
      if !result.degradations.is_empty() {
        answer["degradations"] = Value::Array(result.degradations);
      }
      Ok(answer)
    })?;
  }
  Ok(body)
}

fn read_decode_result(result: &Value) -> Result<DecodeResult, ComponentError> {
  let embedded = match result.get("embedded") {
    None => Vec::new(),
    Some(embedded) => embedded_decodes(embedded)?,
  };
  if !embedded.is_empty() {
    return Ok(DecodeResult::asking(embedded));
  }
  Ok(DecodeResult {
    document: document_of(result, "document", "decode")?,
    degradations: degradations_of(result),
    embedded: Vec::new(),
  })
}

fn encode_json(req: Encode) -> Result<Value, ComponentError> {
  let mut body = tagged_document(&req.document, "document");
  body["content-type"] = json!(req.content_type);
  if let Some(options) = req.options {
    body["options"] = options;
  }
  // The shape it was produced from (spec §6.2, ADR 0101), when there is one.
  if let Some(shape) = req.shape {
    body["shape"] = shape;
  }
  if let Some(embedded) = req.embedded {
    body["embedded"] = answers_json(embedded, |result| {
      Ok(json!({ "value": slot_json(&result.value)? }))
    })?;
  }
  Ok(body)
}

fn read_encode_result(result: &Value) -> Result<EncodeResult, ComponentError> {
  let embedded = match result.get("embedded") {
    None => Vec::new(),
    Some(embedded) => embedded_encodes(embedded)?,
  };
  if !embedded.is_empty() {
    return Ok(EncodeResult::asking(embedded));
  }
  let value = result
    .get("value")
    .cloned()
    .and_then(|value| serde_json::from_value::<SlotValue>(value).ok())
    .ok_or_else(|| ComponentError::synthesised("component-malformed", "encode returned no slot value"))?;
  Ok(EncodeResult::of(value))
}

/// A response frame's `ok` document, or its `error` as the component's own error, passed through
/// untranslated (spec §11.2). A frame that is not one is the component's failure, marked as such.
pub fn read_response(bytes: &[u8], id: &str, op: &str) -> Result<Value, ComponentError> {
  let malformed = |why: &str| {
    ComponentError::synthesised(
      "component-malformed",
      format!("the component answered '{op}' with {why}"),
    )
  };
  let frame: Value =
    serde_json::from_slice(bytes).map_err(|err| malformed(&format!("a frame that is not JSON ({err})")))?;
  if frame.get("type").and_then(Value::as_str) != Some("response") {
    return Err(malformed("a frame that is not a response"));
  }
  if let Some(error) = frame.get("error") {
    return Err(
      serde_json::from_value(error.clone())
        .unwrap_or_else(|_| malformed("an error that is not a ComponentError")),
    );
  }
  let answered = frame.get("id").and_then(Value::as_str).unwrap_or_default();
  if answered != id {
    return Err(malformed(&format!(
      "a response to '{answered}' where '{id}' was asked"
    )));
  }
  frame
    .get("ok")
    .cloned()
    .ok_or_else(|| malformed("a response carrying neither 'ok' nor 'error'"))
}

// --- the component's end: requests read, results written ---

/// One operation on a native component (spec §9.1), from its request document to its result document
/// (spec §6, §7): the component's end of the pipe. An operation on an interface the component does not
/// implement, or one this engine does not know, is `operation-unsupported` (spec §3.1); a request
/// missing what its operation requires is `malformed-frame`.
pub fn serve(
  content: Option<&dyn ContentComponent>,
  matcher: Option<&dyn MatcherComponent>,
  op: &str,
  body: &Value,
) -> Result<Value, ComponentError> {
  let unsupported = || ComponentError::operation_unsupported(op);
  if let Some(verb) = op.strip_prefix("content/") {
    let content = content.ok_or_else(unsupported)?;
    return match verb {
      "decode" => {
        let embedded = match body.get("embedded") {
          None => None,
          Some(answers) => Some(read_answers(answers, op, |answer| {
            Ok(DecodeResult {
              document: document_of(answer, "document", "decode")?,
              degradations: degradations_of(answer),
              embedded: Vec::new(),
            })
          })?),
        };
        let result = content.decode(Decode {
          content_type: string(body, "content-type", op)?,
          value: slot(body, "value", op)?,
          options: body.get("options").cloned(),
          embedded,
        })?;
        Ok(decode_result_json(result))
      }
      "encode" => {
        required(body, "document", op)?;
        let embedded = match body.get("embedded") {
          None => None,
          Some(answers) => Some(read_answers(answers, op, |answer| {
            Ok(EncodeResult::of(slot(answer, "value", op)?))
          })?),
        };
        let result = content.encode(Encode {
          content_type: string(body, "content-type", op)?,
          document: document_of(body, "document", "encode")?,
          options: body.get("options").cloned(),
          shape: body.get("shape").cloned(),
          embedded,
        })?;
        encode_result_json(result)
      }
      "compile" => {
        content.compile(Compile {
          content_type: string(body, "content-type", op)?,
          shape: required(body, "shape", op)?.clone(),
          path: string(body, "path", op)?,
        })?;
        Ok(json!({}))
      }
      "detect" => {
        let result = content.detect(Detect {
          value: slot(body, "value", op)?,
          hint: optional_string(body, "hint"),
        })?;
        let mut answer = json!({});
        if let Some(media_type) = result.media_type {
          answer["media-type"] = json!(media_type);
        }
        if let Some(confidence) = result.confidence {
          answer["confidence"] = json!(confidence);
        }
        Ok(answer)
      }
      "label" => {
        let result = content.label(Label {
          content_type: string(body, "content-type", op)?,
          value: slot(body, "value", op)?,
          at: string(body, "at", op)?,
        })?;
        Ok(match result.media_type {
          Some(media_type) => json!({ "media-type": media_type }),
          None => json!({}),
        })
      }
      _ => Err(unsupported()),
    };
  }
  if let Some(verb) = op.strip_prefix("matcher/") {
    let matcher = matcher.ok_or_else(unsupported)?;
    return match verb {
      "apply" => {
        let values = required(body, "values", op)?
          .as_array()
          .cloned()
          .ok_or_else(|| malformed(op, "'values' must be an array"))?;
        let result = matcher.apply(Apply {
          action: string(body, "action", op)?,
          config: body.get("config").cloned(),
          values,
        })?;
        Ok(json!({ "results": result.results }))
      }
      "compare" => {
        let result = matcher.compare(Compare {
          operator: string(body, "operator", op)?,
          provider: required(body, "provider", op)?.clone(),
          consumer: required(body, "consumer", op)?.clone(),
        })?;
        let mut answer = json!({ "verdict": result.verdict });
        if let Some(reason) = result.reason {
          answer["reason"] = json!(reason);
        }
        Ok(answer)
      }
      "prefix" => {
        let result = matcher.prefix(Prefix {
          operator: string(body, "operator", op)?,
          node: required(body, "node", op)?.clone(),
        })?;
        Ok(match result.prefix {
          Some(prefix) => json!({ "prefix": prefix }),
          None => json!({}),
        })
      }
      _ => Err(unsupported()),
    };
  }
  Err(unsupported())
}

/// [`serve`], frame to frame (spec §3.1): one request frame's bytes in, exactly one response frame's
/// out. A frame that cannot be read is answered with `malformed-frame`, under the id it carried when
/// it carried one.
pub fn serve_frame(
  content: Option<&dyn ContentComponent>,
  matcher: Option<&dyn MatcherComponent>,
  request: &[u8],
) -> Vec<u8> {
  let frame: Value = match serde_json::from_slice(request) {
    Ok(frame) => frame,
    Err(err) => {
      return response(
        "",
        Err(malformed("", &format!("a frame that is not JSON ({err})"))),
      );
    }
  };
  let id = frame.get("id").and_then(Value::as_str).unwrap_or_default();
  let result = match (
    frame.get("type").and_then(Value::as_str),
    frame.get("op").and_then(Value::as_str),
    frame.get("body").filter(|body| body.is_object()),
  ) {
    (Some("request"), Some(op), Some(body)) => serve(content, matcher, op, body),
    _ => Err(malformed(
      "",
      "a request frame needs 'type: request', a string 'op' and an object 'body'",
    )),
  };
  response(id, result)
}

fn response(id: &str, result: Result<Value, ComponentError>) -> Vec<u8> {
  let frame = match result {
    Ok(ok) => json!({ "type": "response", "id": id, "ok": ok }),
    Err(error) => json!({ "type": "response", "id": id, "error": error }),
  };
  serde_json::to_vec(&frame).unwrap_or_default()
}

fn decode_result_json(result: DecodeResult) -> Value {
  if !result.embedded.is_empty() {
    let embedded: Vec<Value> = result
      .embedded
      .into_iter()
      .map(|entry| {
        json!({
          "id": entry.id,
          "content-type": entry.content_type,
          "value": serde_json::to_value(&entry.value).unwrap_or(Value::Null),
        })
      })
      .collect();
    return json!({ "embedded": embedded });
  }
  let mut answer = tagged_document(&result.document, "document");
  if !result.degradations.is_empty() {
    answer["degradations"] = Value::Array(result.degradations);
  }
  answer
}

fn encode_result_json(result: EncodeResult) -> Result<Value, ComponentError> {
  if !result.embedded.is_empty() {
    let embedded: Vec<Value> = result
      .embedded
      .into_iter()
      .map(|entry| {
        let mut answer = tagged_document(&entry.document, "document");
        answer["id"] = json!(entry.id);
        answer["content-type"] = json!(entry.content_type);
        if let Some(at) = entry.at {
          answer["at"] = json!(at);
        }
        answer
      })
      .collect();
    return Ok(json!({ "embedded": embedded }));
  }
  Ok(json!({ "value": slot_json(&result.value)? }))
}

/// The engine's answers to the embedded content a component asked for, as a repeated request carries
/// them (spec §6.6): by id, each the operation's result or `{ error }`.
fn read_answers<T>(
  answers: &Value,
  op: &str,
  read: impl Fn(&Value) -> Result<T, ComponentError>,
) -> Result<Embedded<T>, ComponentError> {
  let answers = answers
    .as_object()
    .ok_or_else(|| malformed(op, "'embedded' must be an object of answers by id"))?;
  answers
    .iter()
    .map(|(id, answer)| {
      let answer = match answer.get("error") {
        Some(error) => Err(
          serde_json::from_value::<ComponentError>(error.clone()).map_err(|err| {
            malformed(
              op,
              &format!("embedded answer '{id}' has an error that is not one: {err}"),
            )
          })?,
        ),
        None => Ok(read(answer)?),
      };
      Ok((id.clone(), answer))
    })
    .collect()
}

fn malformed(op: &str, why: &str) -> ComponentError {
  let message = if op.is_empty() {
    why.to_string()
  } else {
    format!("'{op}' request: {why}")
  };
  ComponentError {
    code: "malformed-frame".to_string(),
    category: "protocol".to_string(),
    message,
    source: None,
    details: None,
    chain: None,
  }
}

fn required<'a>(body: &'a Value, member: &str, op: &str) -> Result<&'a Value, ComponentError> {
  body
    .get(member)
    .ok_or_else(|| malformed(op, &format!("'{member}' is required")))
}

fn string(body: &Value, member: &str, op: &str) -> Result<String, ComponentError> {
  required(body, member, op)?
    .as_str()
    .map(str::to_string)
    .ok_or_else(|| malformed(op, &format!("'{member}' must be a string")))
}

fn slot(body: &Value, member: &str, op: &str) -> Result<SlotValue, ComponentError> {
  serde_json::from_value(required(body, member, op)?.clone())
    .map_err(|err| malformed(op, &format!("'{member}' is not a slot value: {err}")))
}

// --- both ends ---

fn optional_string(document: &Value, member: &str) -> Option<String> {
  document.get(member).and_then(Value::as_str).map(str::to_string)
}

fn degradations_of(document: &Value) -> Vec<Value> {
  document
    .get("degradations")
    .and_then(Value::as_array)
    .cloned()
    .unwrap_or_default()
}

fn slot_json(slot: &SlotValue) -> Result<Value, ComponentError> {
  serde_json::to_value(slot).map_err(|err| ComponentError::internal(err.to_string()))
}

/// A document under `member`, tagged as the content schema tags one: octets as `encoded: base64`,
/// anything else as its JSON value.
fn tagged_document(document: &RuntimeValue, member: &str) -> Value {
  let mut body = json!({});
  match document {
    RuntimeValue::Bytes(bytes) => {
      body[member] = json!(BASE64.encode(bytes));
      body["encoded"] = json!("base64");
    }
    document => body[member] = document.to_json(),
  }
  body
}

/// The document under `member` of a frame tagged as [`tagged_document`] writes one, `text` included.
fn document_of(frame: &Value, member: &str, op: &str) -> Result<RuntimeValue, ComponentError> {
  let text = || frame.get(member).and_then(Value::as_str).unwrap_or_default();
  Ok(match frame.get("encoded").and_then(Value::as_str) {
    Some("base64") => RuntimeValue::Bytes(BASE64.decode(text()).map_err(|err| {
      ComponentError::synthesised("component-malformed", format!("{op} returned bad base64: {err}"))
    })?),
    Some("text") => RuntimeValue::String(text().to_string()),
    _ => RuntimeValue::from_json(frame.get(member).unwrap_or(&Value::Null)),
  })
}

/// The registry's answers to what a component asked for, as the repeated request carries them (spec
/// §6.6): by id, each the operation's result or `{ error }`.
fn answers_json<T>(
  answers: Embedded<T>,
  result: impl Fn(T) -> Result<Value, ComponentError>,
) -> Result<Value, ComponentError> {
  let mut json = serde_json::Map::new();
  for (id, answer) in answers {
    let answer = match answer {
      Ok(answer) => result(answer)?,
      Err(error) => json!({ "error": error }),
    };
    json.insert(id, answer);
  }
  Ok(Value::Object(json))
}

fn malformed_embedded(op: &str, why: &str) -> ComponentError {
  ComponentError::synthesised(
    "component-malformed",
    format!("the component asked for embedded content in '{op}' with {why}"),
  )
}

/// `content/decode`'s `embedded`: each `{ id, content-type, value }` the component asks the engine to
/// decode.
fn embedded_decodes(embedded: &Value) -> Result<Vec<EmbeddedDecode>, ComponentError> {
  let entries = embedded
    .as_array()
    .ok_or_else(|| malformed_embedded("content/decode", "a value that is not an array"))?;
  entries
    .iter()
    .map(|entry| {
      let id = entry.get("id").and_then(Value::as_str);
      let content_type = entry.get("content-type").and_then(Value::as_str);
      let value = entry
        .get("value")
        .cloned()
        .and_then(|value| serde_json::from_value::<SlotValue>(value).ok());
      match (id, content_type, value) {
        (Some(id), Some(content_type), Some(value)) => Ok(EmbeddedDecode {
          id: id.to_string(),
          content_type: content_type.to_string(),
          value,
        }),
        _ => Err(malformed_embedded(
          "content/decode",
          "an entry missing its id, content-type or value",
        )),
      }
    })
    .collect()
}

/// `content/encode`'s `embedded`: each `{ id, content-type, document, encoded?, at? }` the component
/// asks the engine to encode.
fn embedded_encodes(embedded: &Value) -> Result<Vec<EmbeddedEncode>, ComponentError> {
  let entries = embedded
    .as_array()
    .ok_or_else(|| malformed_embedded("content/encode", "a value that is not an array"))?;
  entries
    .iter()
    .map(|entry| {
      let id = entry.get("id").and_then(Value::as_str);
      let content_type = entry.get("content-type").and_then(Value::as_str);
      match (id, content_type, entry.get("document")) {
        (Some(id), Some(content_type), Some(_)) => Ok(EmbeddedEncode {
          id: id.to_string(),
          content_type: content_type.to_string(),
          document: document_of(entry, "document", "content/encode")?,
          at: optional_string(entry, "at"),
        }),
        _ => Err(malformed_embedded(
          "content/encode",
          "an entry missing its id, content-type or document",
        )),
      }
    })
    .collect()
}

// --- the serialising in-process binding ---

/// A native component behind real frames (spec §9.4): each call is written as a request frame's
/// bytes, read back and served by [`serve_frame`], and its response frame's bytes read as the WASM
/// binding reads a guest's. Wrapped in a [`PipeClient`], it is reached exactly as an out-of-tree
/// component is, by the same code.
pub struct InProcess {
  name: String,
  content: Option<Arc<dyn ContentComponent>>,
  matcher: Option<Arc<dyn MatcherComponent>>,
  next_id: AtomicU64,
}

impl InProcess {
  pub fn new(
    name: impl Into<String>,
    content: Option<Arc<dyn ContentComponent>>,
    matcher: Option<Arc<dyn MatcherComponent>>,
  ) -> Self {
    InProcess {
      name: name.into(),
      content,
      matcher,
      next_id: AtomicU64::new(0),
    }
  }
}

impl Pipe for InProcess {
  fn name(&self) -> &str {
    &self.name
  }

  fn call(&self, op: &str, body: Value) -> Result<Value, ComponentError> {
    let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
    let frame = json!({ "type": "request", "id": id, "op": op, "body": body });
    let request = serde_json::to_vec(&frame).map_err(|err| ComponentError::internal(err.to_string()))?;
    let response = serve_frame(self.content.as_deref(), self.matcher.as_deref(), &request);
    read_response(&response, &id, op)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  /// Decodes any slot to its content as a string, asking for one embedded slot first when the content
  /// type is `multipart/test`; encodes a string as its text.
  struct Echo;

  impl ContentComponent for Echo {
    fn handles(&self, _content_type: &str) -> bool {
      true
    }
    fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
      if req.content_type == "multipart/test" && req.embedded.is_none() {
        return Ok(DecodeResult::asking(vec![EmbeddedDecode {
          id: "0".to_string(),
          content_type: "text/plain".to_string(),
          value: req.value,
        }]));
      }
      let document = match req.embedded {
        Some(answers) => match &answers["0"] {
          Ok(answer) => answer.document.clone(),
          Err(error) => RuntimeValue::String(error.code.clone()),
        },
        None => RuntimeValue::String(req.value.content.as_str().unwrap_or_default().to_string()),
      };
      Ok(DecodeResult::of(document))
    }
    fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
      let RuntimeValue::Bytes(bytes) = req.document else {
        return Err(ComponentError::encode_failed(&req.content_type, "only octets"));
      };
      Ok(EncodeResult::of(SlotValue {
        content: json!(String::from_utf8_lossy(&bytes)),
        encoded: Some("text".to_string()),
        content_type: Some(req.content_type),
      }))
    }
    fn compile(&self, _req: Compile) -> Result<CompileResult, ComponentError> {
      Ok(CompileResult {})
    }
    fn detect(&self, _req: Detect) -> Result<DetectResult, ComponentError> {
      Err(ComponentError::operation_unsupported("content/detect"))
    }
  }

  fn serialised() -> PipeClient<InProcess> {
    PipeClient::new(
      InProcess::new("echo", Some(Arc::new(Echo)), None),
      vec!["text/*".to_string()],
    )
  }

  fn text(content: &str) -> SlotValue {
    SlotValue {
      content: json!(content),
      encoded: Some("text".to_string()),
      content_type: None,
    }
  }

  #[test]
  fn a_native_component_behind_frames_answers_as_it_does_natively() {
    let decoded = serialised()
      .decode(Decode::of("text/plain", text("hello")))
      .unwrap();
    assert_eq!(
      decoded,
      DecodeResult::of(RuntimeValue::String("hello".to_string()))
    );
    let encoded = serialised()
      .encode(Encode::of("text/plain", RuntimeValue::Bytes(b"octets".to_vec())))
      .unwrap();
    assert_eq!(encoded.value.content, json!("octets"));
  }

  #[test]
  fn embedded_content_is_asked_for_and_answered_across_the_frames() {
    let client = serialised();
    let asked = client.decode(Decode::of("multipart/test", text("part"))).unwrap();
    assert_eq!(asked.embedded.len(), 1);
    assert_eq!(asked.embedded[0].content_type, "text/plain");
    let mut repeated = Decode::of("multipart/test", text("part"));
    repeated.embedded = Some(Embedded::from([(
      "0".to_string(),
      Ok(DecodeResult::of(RuntimeValue::Bytes(vec![0xff]))),
    )]));
    assert_eq!(
      client.decode(repeated).unwrap().document,
      RuntimeValue::Bytes(vec![0xff]),
      "octets travel tagged as base64 both ways"
    );
    let mut failed = Decode::of("multipart/test", text("part"));
    failed.embedded = Some(Embedded::from([(
      "0".to_string(),
      Err(ComponentError::decode_failed("text/plain", "no")),
    )]));
    assert_eq!(
      client.decode(failed).unwrap().document,
      RuntimeValue::String("decode-failed".to_string()),
      "an error answer reaches the component as an error"
    );
  }

  #[test]
  fn a_component_error_crosses_the_frames_unchanged() {
    let error = serialised()
      .encode(Encode::of(
        "text/plain",
        RuntimeValue::String("not octets".to_string()),
      ))
      .unwrap_err();
    assert_eq!(error.code, "encode-failed");
    assert_eq!(
      error.source, None,
      "the component's own error, not one the binding made"
    );
    assert_eq!(
      serialised()
        .detect(Detect {
          value: text("x"),
          hint: None
        })
        .unwrap_err()
        .code,
      "operation-unsupported"
    );
  }

  #[test]
  fn an_interface_the_component_lacks_and_a_request_missing_a_member_are_refused() {
    let echo: &dyn ContentComponent = &Echo;
    assert_eq!(
      serve(
        Some(echo),
        None,
        "matcher/apply",
        &json!({ "action": "x", "values": [] })
      )
      .unwrap_err()
      .code,
      "operation-unsupported"
    );
    assert_eq!(
      serve(
        Some(echo),
        None,
        "content/decode",
        &json!({ "content-type": "text/plain" })
      )
      .unwrap_err()
      .code,
      "malformed-frame"
    );
    let unreadable = serve_frame(Some(echo), None, b"not json");
    assert_eq!(
      serde_json::from_slice::<Value>(&unreadable).unwrap()["error"]["code"],
      json!("malformed-frame")
    );
  }
}
