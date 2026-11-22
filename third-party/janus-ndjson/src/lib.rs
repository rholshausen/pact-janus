//! A Pact Janus content component for newline-delimited JSON, `application/x-ndjson`.
//!
//! Written from `Documentation/component-author-guide.md` and the specification it links, and
//! nothing else: it speaks component protocol v1 over the frozen `pact:janus-component@1.0.0` pipe.
//!
//! # The document
//!
//! An NDJSON body decodes to a JSON **array** holding one element per record, in the order the
//! records appear. Objects keep their members in the order the octets have them. A consumer writes
//! its shape against that array, typically `each-like` over an `object`.
//!
//! # Decisions about the octets
//!
//! - **Lines** end at `\n`. A `\r` immediately before the `\n` (a `\r\n` line ending) is dropped, so
//!   `\n` and `\r\n` bodies decode alike, and so may one body mixing the two.
//! - **A trailing newline is optional.** `{"a":1}\n` and `{"a":1}` are both one record.
//! - **Blank lines are ignored**, wherever they are: a line that is empty, or holds only spaces, tabs
//!   and carriage returns, is not a record. The NDJSON specification lets a parser ignore empty lines
//!   provided it says so; this is where it says so. Blank lines still count when a line is numbered.
//! - **Empty octets are zero records**, the empty array, because that is what an NDJSON stream with no
//!   records is, and because it is what `encode` writes for the empty array.
//! - **Every other line is exactly one JSON value.** A line that is not JSON, holds two values, or is
//!   not UTF-8 fails the decode as `decode-failed`, and the message names the line, counted from 1
//!   over every line of the body, blank ones included (and the column, where JSON parsing gives one).
//!   A UTF-8 byte order mark is not JSON, so a body beginning with one fails at line 1.
//!
//! # Encoding
//!
//! `encode` is the inverse: each element of the array is written as compact JSON, members in the
//! order the document has them, followed by `\n`. The empty array is written as no octets. Anything
//! but an array (an object, a scalar, a string or byte string document) is refused as
//! `encode-failed`. So `decode(encode(d)) == d` for every document `decode` produces: `encode` never
//! writes a blank line, and a JSON text never holds a raw newline, since JSON escapes one in a string.
//!
//! # Degradations
//!
//! None are declared. Every record is JSON, so every distinction the shape language makes on a JSON
//! value — `null`, booleans, strings, and integer against decimal by how a number is written — is
//! kept in the document exactly as the engine's own JSON component keeps it. What the format cannot
//! carry is a body that is not a sequence of records at all (an object, or a byte string, at the top
//! level); that is not a lost distinction but a document this type cannot hold, so `content/compile`
//! refuses a shape that admits no array, and `encode` refuses such a document.
//!
//! # Operations
//!
//! `component/hello`, `content/decode`, `content/encode`, `content/compile` and `content/detect`
//! (true only for octets of at least two records that are not, as a whole, one JSON document).
//! `content/label` and every other operation are `operation-unsupported`. `component/shutdown` is
//! answered `{}`.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Map, Value, json};

/// The component's name: its namespace, and the name a project declares it under.
pub const NAME: &str = "ndjson";
/// The component's version, from the crate's.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The one media type this component handles.
pub const MEDIA_TYPE: &str = "application/x-ndjson";
/// The component protocol versions this component speaks.
const PROTOCOL_VERSIONS: [u64; 1] = [1];

/// One live copy of the component, as the engine sees it.
///
/// The only thing an instance remembers is whether it has been greeted, because the protocol
/// requires anything before `component/hello` to be answered `handshake-required`. No answer to any
/// other operation depends on an earlier call.
#[derive(Debug, Default)]
pub struct Instance {
  greeted: bool,
}

impl Instance {
  /// A new instance, not yet greeted.
  pub fn new() -> Self {
    Self::default()
  }

  /// One request frame in, one response frame out.
  pub fn call(&mut self, request: &[u8]) -> Vec<u8> {
    let response = self.respond(request);
    // Serialising a `Value` cannot fail; an empty answer would be reported as malformed anyway.
    serde_json::to_vec(&response).unwrap_or_default()
  }

  fn respond(&mut self, request: &[u8]) -> Value {
    let frame = match parse_frame(request) {
      Ok(frame) => frame,
      Err(message) => {
        return error_frame("", error("malformed-frame", "protocol", message, None));
      }
    };
    let Frame { id, op, body } = frame;
    let result = if op == "component/hello" {
      let result = hello(&body);
      if result.is_ok() {
        self.greeted = true;
      }
      result
    } else if !self.greeted {
      Err(error(
        "handshake-required",
        "protocol",
        format!(
          "'{op}' was sent before component/hello; the first request on an instance must be component/hello"
        ),
        None,
      ))
    } else {
      dispatch(&op, &body)
    };
    match result {
      Ok(ok) => json!({ "type": "response", "id": id, "ok": ok }),
      Err(err) => error_frame(&id, err),
    }
  }
}

/// One request frame in, one response frame out, on a fresh instance: the frame is answered as the
/// first one an instance receives. Native tests that want an instance across calls use [`Instance`].
pub fn call(request: &[u8]) -> Vec<u8> {
  Instance::new().call(request)
}

struct Frame {
  id: String,
  op: String,
  body: Map<String, Value>,
}

fn parse_frame(request: &[u8]) -> Result<Frame, String> {
  let value: Value =
    serde_json::from_slice(request).map_err(|e| format!("the request is not a JSON frame: {e}"))?;
  let Value::Object(mut frame) = value else {
    return Err("the request is not a JSON object".into());
  };
  match frame.get("type") {
    Some(Value::String(t)) if t == "request" => {}
    _ => return Err("the frame's 'type' is not 'request'".into()),
  }
  let id = match frame.get("id") {
    Some(Value::String(id)) if !id.is_empty() => id.clone(),
    _ => return Err("the request frame has no 'id'".into()),
  };
  let op = match frame.get("op") {
    Some(Value::String(op)) => op.clone(),
    _ => return Err("the request frame has no 'op'".into()),
  };
  let body = match frame.remove("body") {
    Some(Value::Object(body)) => body,
    _ => return Err(format!("the '{op}' request frame has no object 'body'")),
  };
  Ok(Frame { id, op, body })
}

fn dispatch(op: &str, body: &Map<String, Value>) -> Result<Value, Value> {
  match op {
    "component/shutdown" => Ok(json!({})),
    "content/decode" => decode(body),
    "content/encode" => encode(body),
    "content/compile" => compile(body),
    "content/detect" => detect(body),
    _ => Err(error(
      "operation-unsupported",
      "protocol",
      format!("the ndjson component does not implement '{op}'"),
      Some(json!({ "op": op })),
    )),
  }
}

// ---------------------------------------------------------------------------------------------
// component/hello

fn hello(body: &Map<String, Value>) -> Result<Value, Value> {
  let offered: Vec<u64> = body
    .get("component-protocol-versions")
    .and_then(Value::as_array)
    .map(|versions| versions.iter().filter_map(Value::as_u64).collect())
    .unwrap_or_default();
  let Some(version) = offered.iter().find(|v| PROTOCOL_VERSIONS.contains(v)) else {
    return Err(error(
      "protocol-version-unsupported",
      "protocol",
      format!("the ndjson component speaks component protocol 1, and the engine offered {offered:?}"),
      Some(json!({ "supported": PROTOCOL_VERSIONS })),
    ));
  };
  Ok(json!({
    "component-protocol-version": version,
    "component": { "name": NAME, "version": VERSION },
    "interfaces": ["content"],
    "contributes": {
      "content-types": [ { "media-type": MEDIA_TYPE } ]
    },
    "capabilities": {}
  }))
}

// ---------------------------------------------------------------------------------------------
// content/decode

fn decode(body: &Map<String, Value>) -> Result<Value, Value> {
  let content_type = content_type(body)?;
  let value = body
    .get("value")
    .and_then(Value::as_object)
    .ok_or_else(|| invalid_request("content/decode", "has no slot 'value'"))?;
  let octets = match slot_octets(value, &content_type)? {
    Slot::Octets(octets) => octets,
    // Already a document: a contract recorded it decoded.
    Slot::Document(document) => return Ok(json!({ "document": document })),
  };
  let records = parse_ndjson(&octets).map_err(|failure| {
    let mut details = json!({ "content-type": content_type, "line": failure.line });
    if let Some(column) = failure.column {
      details["column"] = json!(column);
    }
    if let Some(index) = failure.record {
      details["path"] = json!(format!("$[{index}]"));
    }
    error("decode-failed", "document", failure.message, Some(details))
  })?;
  Ok(json!({ "document": Value::Array(records) }))
}

/// Why a body is not NDJSON.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodeFailure {
  /// The line, counted from 1 over every line of the body.
  pub line: usize,
  /// The column within the line, counted from 1, when JSON parsing gave one.
  pub column: Option<usize>,
  /// The index the record would have had in the document.
  pub record: Option<usize>,
  /// A sentence a person can act on, naming the line.
  pub message: String,
}

/// Parses NDJSON octets into the records they hold, under the rules in the crate documentation.
pub fn parse_ndjson(octets: &[u8]) -> Result<Vec<Value>, DecodeFailure> {
  let mut records = Vec::new();
  for (index, raw) in octets.split(|b| *b == b'\n').enumerate() {
    let line_number = index + 1;
    let line = raw.strip_suffix(b"\r").unwrap_or(raw);
    if line.iter().all(|b| matches!(b, b' ' | b'\t' | b'\r')) {
      continue;
    }
    let text = std::str::from_utf8(line).map_err(|e| DecodeFailure {
      line: line_number,
      column: Some(e.valid_up_to() + 1),
      record: Some(records.len()),
      message: format!(
        "line {line_number} is not UTF-8 (an invalid byte at column {}), so it cannot be JSON",
        e.valid_up_to() + 1
      ),
    })?;
    let record: Value = serde_json::from_str(text).map_err(|e| {
      let full = e.to_string();
      let reason = full.rsplit_once(" at line ").map_or(full.as_str(), |(reason, _)| reason);
      DecodeFailure {
        line: line_number,
        column: Some(e.column()),
        record: Some(records.len()),
        message: format!(
          "line {line_number} is not a JSON value: {reason} at column {} (each line of an NDJSON body must hold exactly one JSON value)",
          e.column()
        ),
      }
    })?;
    records.push(record);
  }
  Ok(records)
}

// ---------------------------------------------------------------------------------------------
// content/encode

fn encode(body: &Map<String, Value>) -> Result<Value, Value> {
  let content_type = content_type(body)?;
  let document = body
    .get("document")
    .ok_or_else(|| invalid_request("content/encode", "has no 'document'"))?;
  match body.get("encoded").and_then(Value::as_str).unwrap_or("json") {
    "json" => {}
    "base64" => {
      return Err(encode_failed(
        &content_type,
        "the document is a byte string, and an NDJSON body is an array of records, one JSON value per line",
      ));
    }
    "text" => {
      return Err(encode_failed(
        &content_type,
        "the document is a text string, and an NDJSON body is an array of records, one JSON value per line",
      ));
    }
    other => {
      return Err(encode_failed(
        &content_type,
        &format!("the document is tagged '{other}', which the ndjson component cannot read"),
      ));
    }
  }
  let octets = write_ndjson(document).map_err(|message| encode_failed(&content_type, &message))?;
  Ok(json!({
    "value": { "content": BASE64.encode(octets), "encoded": "base64", "content-type": content_type }
  }))
}

/// Writes a document as NDJSON octets: each element compact, followed by `\n`.
pub fn write_ndjson(document: &Value) -> Result<Vec<u8>, String> {
  let Value::Array(records) = document else {
    return Err(format!(
      "an NDJSON body is an array of records, one JSON value per line, and the document is {}",
      kind_of(document)
    ));
  };
  let mut octets = Vec::new();
  for record in records {
    // Writing a `Value` to a Vec cannot fail.
    serde_json::to_writer(&mut octets, record).map_err(|e| e.to_string())?;
    octets.push(b'\n');
  }
  Ok(octets)
}

// ---------------------------------------------------------------------------------------------
// content/compile

fn compile(body: &Map<String, Value>) -> Result<Value, Value> {
  let content_type = content_type(body)?;
  let shape = body.get("shape").cloned().unwrap_or(Value::Null);
  let path = body.get("path").and_then(Value::as_str).unwrap_or("the slot");
  match admits_an_array(&shape) {
    Admits::No(what) => Err(error(
      "encode-failed",
      "document",
      format!(
        "the shape at {path} admits only {what}, and an NDJSON body is an array of records, one JSON value per line, so no '{MEDIA_TYPE}' body could ever match it"
      ),
      Some(json!({ "content-type": content_type, "path": path })),
    )),
    Admits::MaybeArray => Ok(json!({})),
  }
}

enum Admits {
  /// The shape admits no array; the string says what it admits instead.
  No(String),
  /// The shape may admit an array, or this component cannot tell.
  MaybeArray,
}

/// Whether a shape could admit an array. Conservative: anything it does not recognise (a component's
/// operator, an operator a newer shape language adds) may.
fn admits_an_array(shape: &Value) -> Admits {
  let Some(node) = shape.as_object() else {
    return Admits::MaybeArray;
  };
  let Some(operator) = node.get("shape").and_then(Value::as_str) else {
    return Admits::MaybeArray;
  };
  let example_kind = || match node.get("encoded").and_then(Value::as_str) {
    Some("base64") => "a byte string".to_string(),
    _ => node.get("example").map_or_else(|| "a value".to_string(), kind_of),
  };
  match operator {
    "object" | "each-entry" | "one-of" => Admits::No("objects".into()),
    "string" | "regex" | "datetime" | "date" | "time" | "include" | "semver" => Admits::No("strings".into()),
    "number" | "integer" | "decimal" => Admits::No("numbers".into()),
    "boolean" => Admits::No("booleans".into()),
    "null" => Admits::No("null".into()),
    "equality" | "type" => {
      let is_array =
        node.get("encoded").is_none_or(|e| e == "json") && node.get("example").is_some_and(Value::is_array);
      if is_array {
        Admits::MaybeArray
      } else {
        Admits::No(example_kind())
      }
    }
    "any-of" => {
      let any_array = node
        .get("options")
        .and_then(Value::as_array)
        .is_some_and(|options| options.iter().any(Value::is_array));
      if any_array {
        Admits::MaybeArray
      } else {
        Admits::No("literals that are not arrays".into())
      }
    }
    "optional" | "nullable" => match node.get("of") {
      Some(of) => admits_an_array(of),
      None => Admits::MaybeArray,
    },
    _ => Admits::MaybeArray,
  }
}

// ---------------------------------------------------------------------------------------------
// content/detect

fn detect(body: &Map<String, Value>) -> Result<Value, Value> {
  let Some(value) = body.get("value").and_then(Value::as_object) else {
    return Err(invalid_request("content/detect", "has no slot 'value'"));
  };
  let octets = match slot_octets(value, MEDIA_TYPE) {
    Ok(Slot::Octets(octets)) => octets,
    // A decoded document, or a value this component cannot read as octets, says nothing.
    _ => return Ok(json!({})),
  };
  Ok(match looks_like_ndjson(&octets) {
    true => json!({ "media-type": MEDIA_TYPE, "confidence": 0.6 }),
    false => json!({}),
  })
}

/// At least two records, every line JSON, and the whole not one JSON document: a single JSON value
/// on one line is NDJSON too, but it is far more likely to be plain JSON, so it is not claimed.
pub fn looks_like_ndjson(octets: &[u8]) -> bool {
  match parse_ndjson(octets) {
    Ok(records) if records.len() >= 2 => serde_json::from_slice::<Value>(octets).is_err(),
    _ => false,
  }
}

// ---------------------------------------------------------------------------------------------
// Slot values, errors

enum Slot {
  Octets(Vec<u8>),
  Document(Value),
}

fn slot_octets(value: &Map<String, Value>, content_type: &str) -> Result<Slot, Value> {
  let content = value.get("content");
  match value.get("encoded").and_then(Value::as_str).unwrap_or("json") {
    "base64" => {
      let text = content.and_then(Value::as_str).ok_or_else(|| {
        decode_failed(
          content_type,
          "the slot value is tagged 'base64' but its content is not a string",
        )
      })?;
      BASE64.decode(text).map(Slot::Octets).map_err(|e| {
        decode_failed(
          content_type,
          &format!("the slot value is tagged 'base64' but is not base64: {e}"),
        )
      })
    }
    "text" => {
      let text = content.and_then(Value::as_str).ok_or_else(|| {
        decode_failed(
          content_type,
          "the slot value is tagged 'text' but its content is not a string",
        )
      })?;
      Ok(Slot::Octets(text.as_bytes().to_vec()))
    }
    "json" => match content {
      // An absent content is an empty slot: no octets, so no records.
      None => Ok(Slot::Octets(Vec::new())),
      Some(document) => Ok(Slot::Document(document.clone())),
    },
    other => Err(decode_failed(
      content_type,
      &format!("the slot value is tagged '{other}', which the ndjson component cannot read"),
    )),
  }
}

/// The request's `content-type`, checked to be this component's type (parameters ignored).
fn content_type(body: &Map<String, Value>) -> Result<String, Value> {
  let content_type = body
    .get("content-type")
    .and_then(Value::as_str)
    .unwrap_or(MEDIA_TYPE);
  let essence = content_type.split(';').next().unwrap_or_default().trim();
  if essence.eq_ignore_ascii_case(MEDIA_TYPE) {
    Ok(content_type.to_string())
  } else {
    Err(error(
      "unsupported-content-type",
      "document",
      format!("the ndjson component handles '{MEDIA_TYPE}', not '{content_type}'"),
      Some(json!({ "content-type": content_type })),
    ))
  }
}

fn kind_of(value: &Value) -> String {
  match value {
    Value::Null => "null",
    Value::Bool(_) => "a boolean",
    Value::Number(_) => "a number",
    Value::String(_) => "a string",
    Value::Array(_) => "an array",
    Value::Object(_) => "an object",
  }
  .to_string()
}

fn decode_failed(content_type: &str, message: &str) -> Value {
  error(
    "decode-failed",
    "document",
    message.to_string(),
    Some(json!({ "content-type": content_type })),
  )
}

fn encode_failed(content_type: &str, message: &str) -> Value {
  error(
    "encode-failed",
    "document",
    message.to_string(),
    Some(json!({ "content-type": content_type })),
  )
}

fn invalid_request(op: &str, what: &str) -> Value {
  error(
    "malformed-frame",
    "protocol",
    format!("the {op} request {what}"),
    None,
  )
}

fn error(code: &str, category: &str, message: String, details: Option<Value>) -> Value {
  let mut error = json!({ "code": code, "category": category, "message": message });
  if let Some(details) = details {
    error["details"] = details;
  }
  error
}

fn error_frame(id: &str, error: Value) -> Value {
  json!({ "type": "response", "id": id, "error": error })
}

// ---------------------------------------------------------------------------------------------
// The WASM binding

#[cfg(target_arch = "wasm32")]
mod bindings {
  wit_bindgen::generate!({ world: "component", path: "wit" });

  use std::cell::RefCell;

  // One instance per WASM instance. A component instance is never called concurrently.
  thread_local! {
    static INSTANCE: RefCell<super::Instance> = RefCell::new(super::Instance::new());
  }

  struct Component;

  impl exports::pact::janus_component::pipe::Guest for Component {
    fn call(request: Vec<u8>) -> Vec<u8> {
      INSTANCE.with(|instance| instance.borrow_mut().call(&request))
    }
  }

  export!(Component);
}
