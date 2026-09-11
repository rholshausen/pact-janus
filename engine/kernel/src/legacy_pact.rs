//! v1–v4 pact reading (plan task 3.1), via `pact_models` — the Phase 0.4 reuse decision
//! (`Documentation/reuse-inventory.md`).
//!
//! `pact_models::pact::load_pact_from_json` takes an already-parsed [`serde_json::Value`], not a
//! file path: no filesystem access, which is what the kernel needs on the WASM path (spike 1.2
//! finding: the engine is a guest with no ambient file system). Converting a v1–v4 pact's
//! matching rules into shapes is task 5.5's job, not this module's — this only proves the
//! document can be read, and (below) hands its HTTP interactions to design 3.5's compiler in the
//! form that compiler already speaks, which is a much smaller step than an upgrade.

use crate::contract::{ContractError, Problem};
use crate::plan::{LegacyRequest, LegacyResponse};
use pact_models::bodies::OptionalBody;
use pact_models::pact::Pact;
use pact_models::v4::http_parts::{HttpRequest, HttpResponse};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::panic::RefUnwindSafe;

/// Parse a v1–v4 pact document already read into a [`Value`].
///
/// `source` is a diagnostic label (a file name, a URL, `"inline"`) that `pact_models` echoes back
/// into its own error messages; it is never used for I/O here.
pub fn read(
  source: &str,
  json: &Value,
) -> Result<Box<dyn Pact + Send + Sync + RefUnwindSafe>, ContractError> {
  pact_models::pact::load_pact_from_json(source, json).map_err(|err| ContractError::Invalid {
    problems: vec![Problem {
      pointer: String::new(),
      message: err.to_string(),
    }],
  })
}

/// Every HTTP interaction in `pact`, as `(description, request, response)`. `Interaction::
/// as_v4_http` is `pact_models`' one downcast that is uniform across every pact spec version
/// (v1–v3's `RequestResponseInteraction` converts itself on the call; a genuine v4
/// `SynchronousHttp` is a passthrough) and cleanly excludes message interactions (`None`) without
/// this module having to branch on which pact version it read — so this is the only extraction a
/// caller needs regardless of what `read` handed back.
pub fn http_interactions(pact: &dyn Pact) -> Vec<(String, HttpRequest, HttpResponse)> {
  pact
    .interactions()
    .into_iter()
    .filter_map(|interaction| {
      interaction
        .as_v4_http()
        .map(|http| (http.description, http.request, http.response))
    })
    .collect()
}

/// [`HttpRequest`] to design 3.5's own input type. The only fallible part is the body: JSON only
/// (plan task 3.5's own scope), and `Err` names what content type defeated it rather than
/// guessing.
pub fn legacy_request(request: &HttpRequest) -> Result<LegacyRequest, String> {
  Ok(LegacyRequest {
    method: request.method.clone(),
    path: request.path.clone(),
    query: convert_query(request.query.as_ref()),
    headers: convert_headers(request.headers.as_ref()),
    body: convert_body(&request.body)?,
    matching_rules: request.matching_rules.clone(),
  })
}

/// [`HttpResponse`] to design 3.5's own input type. See [`legacy_request`] for the body note.
pub fn legacy_response(response: &HttpResponse) -> Result<LegacyResponse, String> {
  Ok(LegacyResponse {
    status: response.status as u64,
    headers: convert_headers(response.headers.as_ref()),
    body: convert_body(&response.body)?,
    matching_rules: response.matching_rules.clone(),
  })
}

/// A query value is `Vec<Option<String>>` in `pact_models` — `None` is a bare `?flag` with no
/// `=value` — mapped to an empty string: [`LegacyRequest::query`] has no term for "present but
/// valueless" either, and an empty value is what such a flag would resolve to on the wire anyway.
fn convert_query(query: Option<&HashMap<String, Vec<Option<String>>>>) -> BTreeMap<String, Vec<String>> {
  query
    .map(|map| {
      map
        .iter()
        .map(|(k, values)| {
          (
            k.clone(),
            values.iter().map(|v| v.clone().unwrap_or_default()).collect(),
          )
        })
        .collect()
    })
    .unwrap_or_default()
}

/// `pact_models` headers are `Vec<String>` (repeated headers); [`LegacyRequest::headers`] takes
/// the single-valued form design 3.5 already documented as its scope, so repeats join with `", "`
/// — the same textual form the wire itself would carry for a repeated header.
fn convert_headers(headers: Option<&HashMap<String, Vec<String>>>) -> BTreeMap<String, String> {
  headers
    .map(|map| {
      map
        .iter()
        .map(|(k, values)| (k.clone(), values.join(", ")))
        .collect()
    })
    .unwrap_or_default()
}

/// [`OptionalBody`]'s three "nothing here" variants collapse to design 3.5's two-state model
/// (`LegacyRequest::body`'s own doc comment): `Missing` (the interaction never mentioned a body)
/// is `None`; `Null` and `Empty` (a body was declared — explicitly null, or present with zero
/// bytes) both become `Some(Value::Null)`, since either way there is no JSON content to check and
/// design 3.5's compiler already treats an explicit `null` as "assert the actual body is absent
/// too". A present, non-empty body must be JSON — anything else is `Err`, matching design 3.5's
/// own JSON-only scope.
fn convert_body(body: &OptionalBody) -> Result<Option<Value>, String> {
  match body {
    OptionalBody::Missing => Ok(None),
    OptionalBody::Null | OptionalBody::Empty => Ok(Some(Value::Null)),
    OptionalBody::Present(bytes, content_type, _) => {
      if let Some(content_type) = content_type
        && !content_type.is_json()
      {
        return Err(format!(
          "legacy_pact::legacy_request/response: unsupported body content type '{content_type}' (design 3.5 is JSON-only)"
        ));
      }
      serde_json::from_slice(bytes)
        .map(Some)
        .map_err(|err| format!("legacy_pact: body is not valid JSON: {err}"))
    }
  }
}
