//! The built-in multipart content component (component-interfaces spec §6, ADR 0040): `multipart/*`
//! octets to a document and back. In-tree, but bound through the same interface a third-party
//! component uses (ADR 0012), as the JSON, text, form and XML ones are.
//!
//! A multipart body's document is keyed by part name (ADR 0040 decision 2): one member per name, in the
//! order the names first appear, each an array of the parts with that name in body order, and the parts
//! with no name under `""`. A part is an object of `headers` (each lower-cased name with its value),
//! `filename` (when its `Content-Disposition` gives one) and `body`.
//!
//! A part's body is decoded by the part's own `Content-Type` through the built-in JSON, XML, text and
//! form components, in their registry order (decision 3). A part with no type is text, or bytes when it
//! is not UTF-8, and a part of any other type is bytes. The body is split with `multer`, as today's
//! verifier splits one.

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use indexmap::IndexMap;
use pact_janus_component_form::FormContent;
use pact_janus_component_json::JsonContent;
use pact_janus_component_text::TextContent;
use pact_janus_component_xml::XmlContent;
use pact_janus_kernel::component::{
  Compile, CompileResult, ComponentError, ContentComponent, Decode, DecodeResult, Detect, DetectResult,
  Encode, EncodeResult, SlotValue, media_type_matches,
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
pub struct MultipartContent {
  /// The built-in components a part's body is decoded and encoded by, in the order every embedding
  /// registers them (ADR 0040 decision 3).
  parts: Vec<Box<dyn ContentComponent>>,
}

impl MultipartContent {
  pub fn new() -> Self {
    MultipartContent {
      parts: vec![
        Box::new(JsonContent::new()),
        Box::new(XmlContent::new()),
        Box::new(TextContent::new()),
        Box::new(FormContent::new()),
      ],
    }
  }

  /// The component that reads a part of `content_type`, if a built-in one does.
  fn part_component(&self, content_type: &str) -> Option<&dyn ContentComponent> {
    self
      .parts
      .iter()
      .find(|component| component.handles(content_type))
      .map(|component| component.as_ref())
  }

  /// A part's body as a document: by its type through a built-in component, as text when it has no
  /// type, as bytes otherwise. `None` when its type's component cannot decode it. Text that is not
  /// UTF-8 is bytes, as the legacy layer reads a text body that is not (ADR 0036 decision 3).
  fn decode_part(&self, content_type: Option<&str>, data: Vec<u8>) -> Option<RuntimeValue> {
    let Some(content_type) = content_type else {
      return Some(text_or_bytes(data));
    };
    let Some(component) = self.part_component(content_type) else {
      return Some(RuntimeValue::Bytes(data));
    };
    let decoded = component.decode(Decode {
      content_type: content_type.to_string(),
      value: octets(&data, content_type),
      options: None,
    });
    match decoded {
      Ok(result) => Some(result.document),
      Err(_) if media_type_matches("text/*", content_type) => Some(RuntimeValue::Bytes(data)),
      Err(_) => None,
    }
  }

  /// A part's body as octets: through the built-in component for its type, or as its text or bytes.
  fn encode_part(
    &self,
    name: &str,
    content_type: Option<&str>,
    body: &RuntimeValue,
  ) -> Result<Vec<u8>, String> {
    match (
      body,
      content_type.and_then(|content_type| self.part_component(content_type)),
    ) {
      (RuntimeValue::Absent, _) => Ok(Vec::new()),
      (RuntimeValue::Bytes(bytes), _) => Ok(bytes.clone()),
      (_, Some(component)) => {
        let content_type = content_type.unwrap_or_default();
        component
          .encode(Encode {
            content_type: content_type.to_string(),
            document: body.clone(),
            options: None,
          })
          .map_err(|err| format!("part '{name}': {}", err.message))
          .and_then(|result| slot_bytes(&result.value))
      }
      (RuntimeValue::String(text), None) => Ok(text.as_bytes().to_vec()),
      (_, None) => Err(format!(
        "part '{name}': a structured body needs a content type a built-in component writes (JSON, XML, \
         text or form)"
      )),
    }
  }
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

/// `body` as a multipart document, its boundary from `content_type` (ADR 0040 decision 2).
pub fn parse(body: &[u8], content_type: &str) -> Result<RuntimeValue, String> {
  MultipartContent::new().document(body, content_type)
}

impl MultipartContent {
  fn document(&self, body: &[u8], content_type: &str) -> Result<RuntimeValue, String> {
    let boundary = boundary(content_type)?;
    let mut names: IndexMap<String, Vec<RuntimeValue>> = IndexMap::new();
    for part in split(body, &boundary)? {
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
      if let Some(body) = self.decode_part(part.content_type(), part.data.clone()) {
        members.insert(BODY.to_string(), body);
      }
      names
        .entry(part.name)
        .or_default()
        .push(RuntimeValue::Object(members));
    }
    Ok(RuntimeValue::Object(
      names
        .into_iter()
        .map(|(name, parts)| (name, RuntimeValue::Array(parts)))
        .collect(),
    ))
  }

  /// A multipart document as a body, and the content type that names its boundary.
  fn write(&self, document: &RuntimeValue, content_type: &str) -> Result<(Vec<u8>, String), String> {
    let RuntimeValue::Object(names) = document else {
      return Err("a multipart body is an object of part names, each with a list of parts".to_string());
    };
    let mut written: Vec<(Vec<String>, Vec<u8>)> = Vec::new();
    for (name, parts) in names {
      let parts = match parts {
        RuntimeValue::Array(parts) => parts.as_slice(),
        part => std::slice::from_ref(part),
      };
      for part in parts {
        written.push(self.write_part(name, part)?);
      }
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
    Ok((body, content_type))
  }

  /// One part's header lines and octets. A named part with no `Content-Disposition` gets
  /// `form-data; name="…"`, with its file name; a `base64` transfer encoding is applied.
  fn write_part(&self, name: &str, part: &RuntimeValue) -> Result<(Vec<String>, Vec<u8>), String> {
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
    let data = self.encode_part(name, header("content-type"), body)?;
    let data = if header("content-transfer-encoding")
      .is_some_and(|encoding| encoding.eq_ignore_ascii_case("base64"))
    {
      BASE64.encode(&data).into_bytes()
    } else {
      data
    };
    Ok((lines, data))
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
) -> Result<RuntimeValue, ComponentError> {
  let failed = |message: String| ComponentError::decode_failed(content_type, message);
  match (&value.content, value.encoded.as_deref()) {
    (Value::String(content), Some("base64")) => {
      let bytes = BASE64
        .decode(content)
        .map_err(|err| failed(format!("invalid base64: {err}")))?;
      component.document(&bytes, content_type).map_err(failed)
    }
    (Value::String(content), None | Some("text")) => component
      .document(content.as_bytes(), content_type)
      .map_err(failed),
    (Value::Object(_), None | Some("json")) => Ok(RuntimeValue::from_json(&value.content)),
    (_, Some(other)) if !matches!(other, "base64" | "text" | "json") => {
      Err(failed(format!("unrecognised content encoding '{other}'")))
    }
    _ => Err(failed(
      "a multipart slot's content must be its octets or an object of parts".to_string(),
    )),
  }
}

impl ContentComponent for MultipartContent {
  fn handles(&self, content_type: &str) -> bool {
    is_multipart(content_type)
  }

  /// A declared type that names no boundary takes the one the slot is labelled with: the boundary
  /// frames these octets, and is not part of what the type declares (ADR 0047). A `janus upgrade`d
  /// contract declares `multipart/form-data`, and a reply names its own boundary.
  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    require_multipart(&req.content_type)?;
    let labelled =
      req.value.content_type.as_deref().filter(|label| {
        boundary(&req.content_type).is_err() && boundary(label).is_ok() && is_multipart(label)
      });
    Ok(DecodeResult {
      document: slot_document(self, labelled.unwrap_or(&req.content_type), &req.value)?,
      degradations: Vec::new(),
    })
  }

  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    require_multipart(&req.content_type)?;
    let (body, content_type) = self
      .write(&req.document, &req.content_type)
      .map_err(|message| ComponentError::encode_failed(&req.content_type, message))?;
    Ok(EncodeResult {
      value: SlotValue {
        content: Value::String(BASE64.encode(body)),
        encoded: Some("base64".to_string()),
        content_type: Some(content_type),
      },
    })
  }

  /// Nothing to contribute (spec §6.3): a multipart document is objects, arrays and its parts'
  /// documents, which the generic plan handles.
  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError> {
    require_multipart(&req.content_type)?;
    Ok(CompileResult {
      fragment: None,
      grammar_version: None,
    })
  }

  /// A multipart body is recognised by its declared type and boundary, not its octets (spec §6.5).
  fn detect(&self, _req: Detect) -> Result<DetectResult, ComponentError> {
    Err(ComponentError::operation_unsupported("content/detect"))
  }
}
