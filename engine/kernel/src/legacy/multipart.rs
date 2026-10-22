//! A v1–v4 multipart body as parts (ADR 0040 decision 5): split as today's verifier splits one, with
//! `multer` at the boundary the body's content type names, read by `mime`. Each part is then read by its
//! own type, as a body is ([`super::pact`]), and compared by the legacy compiler as today's
//! `match_mime_multipart` compares it.
//!
//! The built-in multipart content component, `engine/component-multipart`, decodes a reply into the
//! document [`document`] builds from these parts: an object of part names, each an array of parts, with
//! the unnamed ones under `""`, and a part an object of `headers`, `filename` and `body`. The pact's side
//! and the reply's side must agree on that model for a verdict to mean anything, so
//! `tests/multipart_bodies.rs` holds this reader to the component's.

use super::compile::LegacyPart;
use crate::plan::RuntimeValue;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use indexmap::IndexMap;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// A part's headers.
pub const HEADERS: &str = "headers";

/// The file name a part's `Content-Disposition` gives.
pub const FILENAME: &str = "filename";

/// A part's content.
pub const BODY: &str = "body";

/// One part as `multer` reads it: its name (empty when it has none), the file name its
/// `Content-Disposition` gives, its headers (lower-cased, a repeated one with its last value, which is
/// all `multer` keeps), and its octets with a `base64` transfer encoding undone.
pub struct RawPart {
  pub name: String,
  pub filename: Option<String>,
  pub headers: IndexMap<String, String>,
  pub data: Vec<u8>,
}

impl RawPart {
  /// Whether today's verifier counts the part a field: it has a `Content-Disposition` and no file name.
  /// Any other part is a file.
  pub fn is_field(&self) -> bool {
    self.headers.contains_key("content-disposition") && self.filename.is_none()
  }
}

/// The boundary `content_type` names, read as today's verifier reads it.
pub fn boundary(content_type: &str) -> Result<String, String> {
  let mime: mime::Mime = content_type
    .parse()
    .map_err(|err| format!("'{content_type}' is not a media type: {err}"))?;
  mime
    .get_param(mime::BOUNDARY)
    .map(|boundary| boundary.as_str().to_string())
    .ok_or_else(|| format!("'{content_type}' names no boundary"))
}

/// `body`'s parts, split at `boundary` as today's verifier splits them. A `base64`
/// `Content-Transfer-Encoding` is undone, and any other is ignored; base64 that does not decode is kept
/// as it is, as today.
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

/// `future`'s output, when it needs no waiting: `multer` reads a body handed to it whole, so each of
/// its futures is ready when first polled, and no executor, thread or runtime is needed.
fn ready<F: Future>(future: F) -> Result<F::Output, String> {
  let mut future = pin!(future);
  match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
    Poll::Ready(output) => Ok(output),
    Poll::Pending => Err("the multipart parser waited for more of a body it was given whole".to_string()),
  }
}

/// The parts as the multipart component's document (module docs).
pub fn document(parts: &[LegacyPart]) -> RuntimeValue {
  let mut names: IndexMap<String, Vec<RuntimeValue>> = IndexMap::new();
  for part in parts {
    let mut members: IndexMap<String, RuntimeValue> = IndexMap::new();
    members.insert(
      HEADERS.to_string(),
      RuntimeValue::Object(
        part
          .headers
          .iter()
          .map(|(name, value)| (name.clone(), RuntimeValue::String(value.clone())))
          .collect(),
      ),
    );
    if let Some(filename) = &part.filename {
      members.insert(FILENAME.to_string(), RuntimeValue::String(filename.clone()));
    }
    members.insert(BODY.to_string(), part.body.document());
    names
      .entry(part.name.clone())
      .or_default()
      .push(RuntimeValue::Object(members));
  }
  RuntimeValue::Object(
    names
      .into_iter()
      .map(|(name, parts)| (name, RuntimeValue::Array(parts)))
      .collect(),
  )
}
