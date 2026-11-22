//! The built-in multipart content component (component-interfaces spec §6, ADR 0040): `multipart/*`
//! octets to a document and back. In-tree, but bound through the same interface a third-party
//! component uses (ADR 0012), as the JSON, text, form and XML ones are.
//!
//! A multipart body's document is keyed by part name (ADR 0040 decision 2): one member per name, in the
//! order the names first appear, each an array of the parts with that name in body order, and the parts
//! with no name under `""`. A part is an object of `headers` (each lower-cased name with its value),
//! `filename` (when its `Content-Disposition` gives one) and `body`.
//!
//! A part's body is decoded by the part's own `Content-Type` through the session's content registry,
//! declared components included (ADR 0103, superseding ADR 0040 decision 3): the component asks for each
//! part's body as embedded content (component-interfaces spec §6.6), and the engine answers before it
//! is asked again. A part with no type is text, or bytes when it is not UTF-8; a part that is itself
//! multipart is bytes; a part of a type nothing claims is bytes, which is the registry's answer for it.
//! The body is split with `multer`, as today's verifier splits one.

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use indexmap::IndexMap;
use pact_janus_kernel::component::{
  Compile, CompileResult, ComponentError, ContentComponent, Decode, DecodeResult, Detect, DetectResult,
  Embedded, EmbeddedDecode, EmbeddedEncode, Encode, EncodeResult, Label, LabelResult, SlotValue,
  media_type_matches,
};
use pact_janus_kernel::plan::RuntimeValue;
use serde_json::Value;

/// The media type this component declares.
pub const MEDIA_TYPE: &str = "multipart/*";

/// A part's headers: each lower-cased name with its value.
pub const HEADERS: &str = "headers";

/// The file name a part's `Content-Disposition` gives, when it gives one.
pub const FILENAME: &str = "filename";

/// A part's content, decoded by its type.
pub const BODY: &str = "body";

/// The boundary `encode` writes when the content type names none, made longer until no part holds it.
const BOUNDARY: &str = "janus-multipart-boundary";

/// Multipart bodies, `multipart/*`.
pub struct MultipartContent;

impl MultipartContent {
  pub fn new() -> Self {
    MultipartContent
  }
}

/// What becomes of the parts' bodies while a body is read into its document.
enum Bodies<'a> {
  /// Ask the engine for each typed part's body (spec §6.6): the first request.
  Ask,
  /// The engine's answers, by part index: the repeated request.
  Answered(&'a Embedded<DecodeResult>),
  /// Leave them out: what `label` reads needs only the parts' headers.
  Skip,
}

/// A body read into its document, or the embedded content reading it needs first.
enum Read {
  Document(RuntimeValue, Vec<Value>),
  Asking(Vec<EmbeddedDecode>),
}

/// A document written as a body, or the embedded documents writing it needs first.
enum Written {
  Body(Vec<u8>, String),
  Asking(Vec<EmbeddedEncode>),
}

/// The id a part's body is asked for by: its index in the body, or in the document's parts.
fn part_id(index: usize) -> String {
  index.to_string()
}

/// Whether a part's body is asked of the registry: it has a type, and the type is not multipart, which
/// stays bytes (ADR 0040 decision 3's model, unchanged).
fn embedded_type(content_type: Option<&str>) -> Option<&str> {
  content_type.filter(|content_type| !is_multipart(content_type))
}

/// A part's body as a document, from the registry's answer (ADR 0103): what it decoded, or, when the
/// part's type could not decode it, bytes for a text type (as the legacy layer reads text that is not
/// UTF-8, ADR 0036 decision 3) and no body otherwise.
fn answered_body(
  answer: &Result<DecodeResult, ComponentError>,
  content_type: &str,
  data: &[u8],
) -> Option<RuntimeValue> {
  match answer {
    Ok(result) => Some(result.document.clone()),
    Err(_) if media_type_matches("text/*", content_type) => Some(RuntimeValue::Bytes(data.to_vec())),
    Err(_) => None,
  }
}

/// A plan path's segment for a part name: `.name`, or `['name']` when it is not a plain identifier.
fn path_member(name: &str) -> String {
  if !name.is_empty()
    && name
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
  {
    format!(".{name}")
  } else {
    format!("['{}']", name.replace('\\', "\\\\").replace('\'', "\\'"))
  }
}

/// A JSON pointer's segment (RFC 6901).
fn pointer_segment(segment: &str) -> String {
  segment.replace('~', "~0").replace('/', "~1")
}

impl Default for MultipartContent {
  fn default() -> Self {
    Self::new()
  }
}

/// Whether `content_type` is a multipart type, as today's verifier counts one: its main type is
/// `multipart`.
pub fn is_multipart(content_type: &str) -> bool {
  media_type_matches(MEDIA_TYPE, content_type)
}

/// One part, as `multer` reads it: its name (empty when it has none), the file name its
/// `Content-Disposition` gives, its headers (a repeated one with its last value, which is all `multer`
/// keeps), and its octets with a `base64` transfer encoding undone.
#[derive(Debug, Clone, PartialEq)]
pub struct RawPart {
  pub name: String,
  pub filename: Option<String>,
  pub headers: IndexMap<String, String>,
  pub data: Vec<u8>,
}

impl RawPart {
  /// The part's own `Content-Type`, when it has one.
  pub fn content_type(&self) -> Option<&str> {
    self.headers.get("content-type").map(String::as_str)
  }
}

/// The boundary `content_type` names, read as today's verifier reads it: the type parsed by `mime`,
/// and its `boundary` parameter. (`multer`'s own `parse_boundary` reads only `multipart/form-data`.)
pub fn boundary(content_type: &str) -> Result<String, String> {
  let mime: mime::Mime = content_type
    .parse()
    .map_err(|err| format!("'{content_type}' is not a media type: {err}"))?;
  mime
    .get_param(mime::BOUNDARY)
    .map(|boundary| boundary.as_str().to_string())
    .ok_or_else(|| format!("'{content_type}' names no boundary"))
}

/// `body`'s parts, split at `boundary` with `multer` as today's verifier splits them. A `base64`
/// `Content-Transfer-Encoding` is undone, and any other is ignored, as today; base64 that does not
/// decode is kept as it is.
pub fn split(body: &[u8], boundary: &str) -> Result<Vec<RawPart>, String> {
  let chunk: Result<Vec<u8>, std::convert::Infallible> = Ok(body.to_vec());
  let mut multipart = multer::Multipart::new(futures_util::stream::iter([chunk]), boundary);
  let mut parts = Vec::new();
  while let Some(field) = ready(multipart.next_field())?.map_err(|err| err.to_string())? {
    let name = field.name().unwrap_or_default().to_string();
    let filename = field.file_name().map(str::to_string);
    let headers: IndexMap<String, String> = field
      .headers()
      .iter()
      .map(|(header, value)| {
        (
          header.as_str().to_string(),
          String::from_utf8_lossy(value.as_bytes()).to_string(),
        )
      })
      .collect();
    let data = ready(field.bytes())?.map_err(|err| err.to_string())?.to_vec();
    let base64 = headers
      .get("content-transfer-encoding")
      .is_some_and(|encoding| encoding.eq_ignore_ascii_case("base64"));
    let data = if base64 {
      BASE64.decode(&data).unwrap_or(data)
    } else {
      data
    };
    parts.push(RawPart {
      name,
      filename,
      headers,
      data,
    });
  }
  Ok(parts)
}

/// `future`'s output, when it needs no waiting. `multer` reads a body handed to it whole, so every
/// future it returns is ready when first polled; no executor, and so no thread or runtime, is needed.
fn ready<F: Future>(future: F) -> Result<F::Output, String> {
  let mut future = pin!(future);
  match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
    Poll::Ready(output) => Ok(output),
    Poll::Pending => Err("the multipart parser waited for more of a body it was given whole".to_string()),
  }
}

impl MultipartContent {
  /// `body` as a multipart document, its boundary from `content_type` (ADR 0040 decision 2), or the
  /// parts' bodies it needs decoded first.
  fn document(&self, body: &[u8], content_type: &str, bodies: Bodies) -> Result<Read, String> {
    let boundary = boundary(content_type)?;
    let mut names: IndexMap<String, Vec<RuntimeValue>> = IndexMap::new();
    let mut asking = Vec::new();
    let mut degradations = Vec::new();
    for (index, part) in split(body, &boundary)?.into_iter().enumerate() {
      let mut members: IndexMap<String, RuntimeValue> = IndexMap::new();
      let headers = part
        .headers
        .iter()
        .map(|(name, value)| (name.clone(), RuntimeValue::String(value.clone())))
        .collect();
      members.insert(HEADERS.to_string(), RuntimeValue::Object(headers));
      if let Some(filename) = &part.filename {
        members.insert(FILENAME.to_string(), RuntimeValue::String(filename.clone()));
      }
      let within_name = names.get(&part.name).map_or(0, Vec::len);
      let body = match (part.content_type(), embedded_type(part.content_type()), &bodies) {
        (_, _, Bodies::Skip) => None,
        (None, _, _) => Some(text_or_bytes(part.data.clone())),
        (Some(_), None, _) => Some(RuntimeValue::Bytes(part.data.clone())),
        (Some(_), Some(embedded), Bodies::Ask) => {
          asking.push(EmbeddedDecode {
            id: part_id(index),
            content_type: embedded.to_string(),
            value: octets(&part.data, embedded),
          });
          None
        }
        (Some(_), Some(embedded), Bodies::Answered(answers)) => {
          let answer = answers
            .get(&part_id(index))
            .ok_or_else(|| format!("the engine gave no answer for part '{}'", part.name))?;
          if let Ok(result) = answer {
            let at = format!("${}[{within_name}].{BODY}", path_member(&part.name));
            degradations.extend(result.degradations.iter().map(|degradation| {
              let mut degradation = degradation.clone();
              let inner = degradation
                .get("path")
                .and_then(Value::as_str)
                .and_then(|path| path.strip_prefix('$'))
                .unwrap_or_default()
                .to_string();
              degradation["path"] = Value::String(format!("{at}{inner}"));
              degradation
            }));
          }
          answered_body(answer, embedded, &part.data)
        }
      };
      if let Some(body) = body {
        members.insert(BODY.to_string(), body);
      }
      names
        .entry(part.name)
        .or_default()
        .push(RuntimeValue::Object(members));
    }
    if !asking.is_empty() {
      return Ok(Read::Asking(asking));
    }
    Ok(Read::Document(
      RuntimeValue::Object(
        names
          .into_iter()
          .map(|(name, parts)| (name, RuntimeValue::Array(parts)))
          .collect(),
      ),
      degradations,
    ))
  }

  /// A multipart document as a body, and the content type that names its boundary, or the parts'
  /// bodies it needs encoded first.
  fn write(
    &self,
    document: &RuntimeValue,
    content_type: &str,
    answers: Option<&Embedded<EncodeResult>>,
  ) -> Result<Written, String> {
    let RuntimeValue::Object(names) = document else {
      return Err("a multipart body is an object of part names, each with a list of parts".to_string());
    };
    let mut written: Vec<(Vec<String>, Vec<u8>)> = Vec::new();
    let mut asking = Vec::new();
    for (name, parts) in names {
      let (parts, listed) = match parts {
        RuntimeValue::Array(parts) => (parts.as_slice(), true),
        part => (std::slice::from_ref(part), false),
      };
      for (within_name, part) in parts.iter().enumerate() {
        let at = if listed {
          format!("/{}/{within_name}/{BODY}", pointer_segment(name))
        } else {
          format!("/{}/{BODY}", pointer_segment(name))
        };
        let index = written.len() + asking.len();
        match self.write_part(
          name,
          part,
          PartAt {
            id: part_id(index),
            at,
          },
          answers,
        )? {
          PartWritten::Written(lines, data) => written.push((lines, data)),
          PartWritten::Asking(embedded) => asking.push(embedded),
        }
      }
    }
    if !asking.is_empty() {
      return Ok(Written::Asking(asking));
    }
    let (boundary, content_type) = match boundary(content_type) {
      Ok(boundary) => (boundary, content_type.to_string()),
      Err(_) => {
        let boundary = fresh_boundary(&written);
        let content_type = format!("{content_type}; boundary={boundary}");
        (boundary, content_type)
      }
    };
    let delimiter = format!("--{boundary}");
    if written
      .iter()
      .any(|(_, data)| contains(data, delimiter.as_bytes()))
    {
      return Err(format!("a part holds the boundary '{boundary}'"));
    }
    let mut body = Vec::new();
    for (headers, data) in written {
      body.extend_from_slice(delimiter.as_bytes());
      body.extend_from_slice(b"\r\n");
      for header in headers {
        body.extend_from_slice(header.as_bytes());
        body.extend_from_slice(b"\r\n");
      }
      body.extend_from_slice(b"\r\n");
      body.extend_from_slice(&data);
      body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(delimiter.as_bytes());
    body.extend_from_slice(b"--\r\n");
    Ok(Written::Body(body, content_type))
  }

  /// One part's header lines and octets. A named part with no `Content-Disposition` gets
  /// `form-data; name="…"`, with its file name; a `base64` transfer encoding is applied.
  fn write_part(
    &self,
    name: &str,
    part: &RuntimeValue,
    place: PartAt,
    answers: Option<&Embedded<EncodeResult>>,
  ) -> Result<PartWritten, String> {
    let RuntimeValue::Object(members) = part else {
      return Err(format!(
        "part '{name}' must be an object of headers, filename and body"
      ));
    };
    let headers: Vec<(String, String)> = match members.get(HEADERS) {
      None | Some(RuntimeValue::Absent) => Vec::new(),
      Some(RuntimeValue::Object(headers)) => headers
        .iter()
        .map(|(header, value)| match value {
          RuntimeValue::String(value) => Ok((header.to_ascii_lowercase(), value.clone())),
          _ => Err(format!("part '{name}': header '{header}' must be a string")),
        })
        .collect::<Result<_, _>>()?,
      Some(_) => return Err(format!("part '{name}': its headers must be an object")),
    };
    let filename = match members.get(FILENAME) {
      None | Some(RuntimeValue::Absent) => None,
      Some(RuntimeValue::String(filename)) => Some(filename.as_str()),
      Some(_) => return Err(format!("part '{name}': its filename must be a string")),
    };
    let header = |wanted: &str| {
      headers
        .iter()
        .find(|(header, _)| header == wanted)
        .map(|(_, value)| value.as_str())
    };
    let mut lines = Vec::new();
    if !name.is_empty() && header("content-disposition").is_none() {
      let quoted = |value: &str| {
        if value.contains(['"', '\r', '\n']) {
          Err(format!(
            "part '{name}': '{value}' cannot be written in a Content-Disposition"
          ))
        } else {
          Ok(format!("\"{value}\""))
        }
      };
      let mut disposition = format!("content-disposition: form-data; name={}", quoted(name)?);
      if let Some(filename) = filename {
        disposition.push_str(&format!("; filename={}", quoted(filename)?));
      }
      lines.push(disposition);
    }
    for (header, value) in &headers {
      if value.contains(['\r', '\n']) {
        return Err(format!(
          "part '{name}': header '{header}' cannot hold a line break"
        ));
      }
      lines.push(format!("{header}: {value}"));
    }
    let body = members.get(BODY).unwrap_or(&RuntimeValue::Absent);
    let data = match encode_part(name, header("content-type"), body, place, answers)? {
      PartBody::Octets(data) => data,
      PartBody::Asking(embedded) => return Ok(PartWritten::Asking(embedded)),
    };
    let data = if header("content-transfer-encoding")
      .is_some_and(|encoding| encoding.eq_ignore_ascii_case("base64"))
    {
      BASE64.encode(&data).into_bytes()
    } else {
      data
    };
    Ok(PartWritten::Written(lines, data))
  }
}

/// Where a part is: the id its body is asked for by, and a JSON pointer to its body in the document.
struct PartAt {
  id: String,
  at: String,
}

enum PartWritten {
  Written(Vec<String>, Vec<u8>),
  Asking(EmbeddedEncode),
}

enum PartBody {
  Octets(Vec<u8>),
  Asking(EmbeddedEncode),
}

/// A part's body as octets (ADR 0103): octets as they are, a body of a type that is not multipart
/// through the registry, which the engine is asked for first, and text with no type, or of a type no
/// component writes, as its UTF-8.
fn encode_part(
  name: &str,
  content_type: Option<&str>,
  body: &RuntimeValue,
  place: PartAt,
  answers: Option<&Embedded<EncodeResult>>,
) -> Result<PartBody, String> {
  let embedded = embedded_type(content_type);
  match (body, embedded, answers) {
    (RuntimeValue::Absent, _, _) => Ok(PartBody::Octets(Vec::new())),
    (RuntimeValue::Bytes(bytes), _, _) => Ok(PartBody::Octets(bytes.clone())),
    (_, Some(content_type), None) => Ok(PartBody::Asking(EmbeddedEncode {
      id: place.id,
      content_type: content_type.to_string(),
      document: body.clone(),
      at: Some(place.at),
    })),
    (_, Some(content_type), Some(answers)) => match answers.get(&place.id) {
      Some(Ok(result)) => slot_bytes(&result.value)
        .map(PartBody::Octets)
        .map_err(|err| format!("part '{name}': {err}")),
      Some(Err(error)) => match body {
        RuntimeValue::String(text) if error.code == "unsupported-content-type" => {
          Ok(PartBody::Octets(text.as_bytes().to_vec()))
        }
        _ => Err(format!("part '{name}' as '{content_type}': {}", error.message)),
      },
      None => Err(format!("the engine gave no answer for part '{name}'")),
    },
    (RuntimeValue::String(text), None, _) => Ok(PartBody::Octets(text.as_bytes().to_vec())),
    (_, None, _) => Err(format!(
      "part '{name}': a structured body needs a content type a content component writes"
    )),
  }
}

fn text_or_bytes(data: Vec<u8>) -> RuntimeValue {
  match String::from_utf8(data) {
    Ok(text) => RuntimeValue::String(text),
    Err(err) => RuntimeValue::Bytes(err.into_bytes()),
  }
}

/// Octets as the slot a content component decodes.
fn octets(data: &[u8], content_type: &str) -> SlotValue {
  SlotValue {
    content: Value::String(BASE64.encode(data)),
    encoded: Some("base64".to_string()),
    content_type: Some(content_type.to_string()),
  }
}

/// The octets a content component's encoded slot carries.
fn slot_bytes(value: &SlotValue) -> Result<Vec<u8>, String> {
  match (&value.content, value.encoded.as_deref()) {
    (Value::String(content), Some("base64")) => BASE64
      .decode(content)
      .map_err(|err| format!("invalid base64: {err}")),
    (Value::String(content), _) => Ok(content.as_bytes().to_vec()),
    (other, _) => Ok(other.to_string().into_bytes()),
  }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
  haystack.windows(needle.len()).any(|window| window == needle)
}

/// [`BOUNDARY`], with a number after it when a part holds it.
fn fresh_boundary(parts: &[(Vec<String>, Vec<u8>)]) -> String {
  (0..)
    .map(|n| {
      if n == 0 {
        BOUNDARY.to_string()
      } else {
        format!("{BOUNDARY}-{n}")
      }
    })
    .find(|boundary| !parts.iter().any(|(_, data)| contains(data, boundary.as_bytes())))
    .unwrap_or_else(|| BOUNDARY.to_string())
}

fn require_multipart(content_type: &str) -> Result<(), ComponentError> {
  if is_multipart(content_type) {
    Ok(())
  } else {
    Err(ComponentError::unsupported_content_type(content_type))
  }
}

/// A slot's content as a document: `base64` octets or text are split; a document already decoded,
/// as a contract records one, is taken as it is.
fn slot_document(
  component: &MultipartContent,
  content_type: &str,
  value: &SlotValue,
  bodies: Bodies,
) -> Result<Read, ComponentError> {
  let failed = |message: String| ComponentError::decode_failed(content_type, message);
  match (&value.content, value.encoded.as_deref()) {
    (Value::String(content), Some("base64")) => {
      let bytes = BASE64
        .decode(content)
        .map_err(|err| failed(format!("invalid base64: {err}")))?;
      component.document(&bytes, content_type, bodies).map_err(failed)
    }
    (Value::String(content), None | Some("text")) => component
      .document(content.as_bytes(), content_type, bodies)
      .map_err(failed),
    (Value::Object(_), None | Some("json")) => Ok(Read::Document(
      RuntimeValue::from_json(&value.content),
      Vec::new(),
    )),
    (_, Some(other)) if !matches!(other, "base64" | "text" | "json") => {
      Err(failed(format!("unrecognised content encoding '{other}'")))
    }
    _ => Err(failed(
      "a multipart slot's content must be its octets or an object of parts".to_string(),
    )),
  }
}

/// The type a slot's octets are split by: the declared one, or, when it names no boundary, the
/// multipart type the slot is labelled with that does (ADR 0047).
fn framing_type<'a>(content_type: &'a str, value: &'a SlotValue) -> &'a str {
  value
    .content_type
    .as_deref()
    .filter(|label| boundary(content_type).is_err() && boundary(label).is_ok() && is_multipart(label))
    .unwrap_or(content_type)
}

impl ContentComponent for MultipartContent {
  fn handles(&self, content_type: &str) -> bool {
    is_multipart(content_type)
  }

  /// A declared type that names no boundary takes the one the slot is labelled with: the boundary
  /// frames these octets, and is not part of what the type declares (ADR 0047). A `janus upgrade`d
  /// contract declares `multipart/form-data`, and a reply names its own boundary.
  ///
  /// Each typed part's body is asked of the registry first (spec §6.6, ADR 0103): the first request is
  /// answered with what it needs, and the repeated one, carrying the registry's answers, with the
  /// document.
  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    require_multipart(&req.content_type)?;
    let bodies = match &req.embedded {
      None => Bodies::Ask,
      Some(answers) => Bodies::Answered(answers),
    };
    let content_type = framing_type(&req.content_type, &req.value);
    Ok(match slot_document(self, content_type, &req.value, bodies)? {
      Read::Document(document, degradations) => DecodeResult {
        document,
        degradations,
        embedded: Vec::new(),
      },
      Read::Asking(embedded) => DecodeResult::asking(embedded),
    })
  }

  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    require_multipart(&req.content_type)?;
    let written = self
      .write(&req.document, &req.content_type, req.embedded.as_ref())
      .map_err(|message| ComponentError::encode_failed(&req.content_type, message))?;
    Ok(match written {
      Written::Body(body, content_type) => EncodeResult::of(SlotValue {
        content: Value::String(BASE64.encode(body)),
        encoded: Some("base64".to_string()),
        content_type: Some(content_type),
      }),
      Written::Asking(embedded) => EncodeResult::asking(embedded),
    })
  }

  /// Nothing to refuse (spec §6.3): a multipart document is objects, arrays and its parts'
  /// documents, all of which any shape can describe.
  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError> {
    require_multipart(&req.content_type)?;
    Ok(CompileResult {})
  }

  /// A multipart body is recognised by its declared type and boundary, not its octets (spec §6.5).
  fn detect(&self, _req: Detect) -> Result<DetectResult, ComponentError> {
    Err(ComponentError::operation_unsupported("content/detect"))
  }

  /// A part's `body` is labelled by the part's own `Content-Type` (ADR 0077), the type it was decoded
  /// by: `/<name>/<index>/body`. Nothing else in the document carries a label, and a part with no
  /// type has none.
  fn label(&self, req: Label) -> Result<LabelResult, ComponentError> {
    require_multipart(&req.content_type)?;
    let content_type = framing_type(&req.content_type, &req.value);
    let Read::Document(document, _) = slot_document(self, content_type, &req.value, Bodies::Skip)? else {
      return Ok(LabelResult { media_type: None });
    };
    let segments: Vec<String> = req
      .at
      .split('/')
      .skip(1)
      .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
      .collect();
    let media_type = match segments.as_slice() {
      [name, index, body] if body == BODY => match &document {
        RuntimeValue::Object(names) => index
          .parse::<usize>()
          .ok()
          .and_then(|index| match names.get(name) {
            Some(RuntimeValue::Array(parts)) => parts.get(index),
            _ => None,
          })
          .and_then(|part| match part {
            RuntimeValue::Object(members) => match members.get(HEADERS) {
              Some(RuntimeValue::Object(headers)) => match headers.get("content-type") {
                Some(RuntimeValue::String(content_type)) => Some(content_type.clone()),
                _ => None,
              },
              _ => None,
            },
            _ => None,
          }),
        _ => None,
      },
      _ => None,
    };
    Ok(LabelResult { media_type })
  }
}
