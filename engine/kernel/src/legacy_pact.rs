//! v1–v4 pact reading (plan task 3.1), via `pact_models` — the Phase 0.4 reuse decision
//! (`Documentation/reuse-inventory.md`).
//!
//! `pact_models::pact::load_pact_from_json` takes an already-parsed [`serde_json::Value`], not a
//! file path: no filesystem access, which is what the kernel needs on the WASM path (spike 1.2
//! finding: the engine is a guest with no ambient file system). Converting a v1–v4 pact's
//! matching rules into shapes is task 5.5's job, not this module's — this only proves the
//! document can be read, and (below) hands its HTTP interactions to design 3.5's compiler in the
//! form that compiler already speaks, which is a much smaller step than an upgrade.
//!
//! **Plan task 5.4 is why that distinction matters.** A v1–v4 pact verifies *without* being
//! upgraded: the verification run (`protocol::verification`) reads a pact here, compiles each
//! interaction with design 3.5's compiler, and replays the pact's own recorded request at the
//! provider. Nothing on that path invents a shape, so nothing on it can lose one — which is what
//! "providers upgrade first at no cost" has to mean to be worth claiming. The two extra pieces
//! that path needs live here too, because both are v1–v4-shaped and this module is the kernel's
//! v1–v4 adapter (kernel-boundary-review.md findings 3 and 6): [`request_parts`], the recorded
//! request in the wire vocabulary a transport's `send` consumes, and [`header_captures`], the
//! reply's headers in the single-valued form design 3.5's plans resolve against.

use crate::component::{Part, Parts, SlotValue};
use crate::contract::{ContractError, Problem};
use crate::plan::{CapturedValues, LegacyRequest, LegacyResponse, RuntimeValue};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_models::bodies::OptionalBody;
use pact_models::pact::Pact;
use pact_models::provider_states::ProviderState;
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

/// One HTTP interaction of a v1–v4 pact, in the terms every caller here needs: what it is called,
/// which provider states it declares, and the request/response pair itself.
///
/// `provider_states` is the v3 list even when the pact is v1/v2 — `as_v4_http` below normalises a
/// single `providerState` string into a one-element list with no parameters, so nothing
/// downstream branches on the pact's version to find out what state an interaction needs.
pub struct LegacyInteraction {
  pub description: String,
  pub provider_states: Vec<ProviderState>,
  pub request: HttpRequest,
  pub response: HttpResponse,
}

/// Every HTTP interaction in `pact`. `Interaction::as_v4_http` is `pact_models`' one downcast that
/// is uniform across every pact spec version (v1–v3's `RequestResponseInteraction` converts itself
/// on the call; a genuine v4 `SynchronousHttp` is a passthrough) and cleanly excludes message
/// interactions (`None`) without this module having to branch on which pact version it read — so
/// this is the only extraction a caller needs regardless of what `read` handed back.
pub fn http_interactions(pact: &dyn Pact) -> Vec<LegacyInteraction> {
  pact
    .interactions()
    .into_iter()
    .filter_map(|interaction| {
      interaction.as_v4_http().map(|http| LegacyInteraction {
        description: http.description,
        provider_states: http.provider_states,
        request: http.request,
        response: http.response,
      })
    })
    .collect()
}

/// Why a v1–v4 part could not be handed to design 3.5's compiler. Two genuinely different things,
/// kept apart because their remedies are: a content type this engine has no compiler for is a
/// missing *component* (the pact is fine, the engine is short a piece), while a body that claimed
/// to be JSON and was not is a problem with the *document*.
#[derive(Debug, Clone)]
pub enum Unsupported {
  /// A body whose declared content type design 3.5 does not compile (it is JSON-only).
  ContentType(String),
  /// A body that claimed JSON and did not parse as it.
  Malformed(String),
}

impl std::fmt::Display for Unsupported {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Unsupported::ContentType(content_type) => write!(
        f,
        "legacy_pact: unsupported body content type '{content_type}' (design 3.5 is JSON-only)"
      ),
      Unsupported::Malformed(message) => write!(f, "legacy_pact: body is not valid JSON: {message}"),
    }
  }
}

/// [`HttpRequest`] to design 3.5's own input type. The only fallible part is the body: JSON only
/// (plan task 3.5's own scope), and `Err` names what content type defeated it rather than
/// guessing.
pub fn legacy_request(request: &HttpRequest) -> Result<LegacyRequest, Unsupported> {
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
pub fn legacy_response(response: &HttpResponse) -> Result<LegacyResponse, Unsupported> {
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
fn convert_body(body: &OptionalBody) -> Result<Option<Value>, Unsupported> {
  match body {
    OptionalBody::Missing => Ok(None),
    OptionalBody::Null | OptionalBody::Empty => Ok(Some(Value::Null)),
    OptionalBody::Present(bytes, content_type, _) => {
      if let Some(content_type) = content_type
        && !content_type.is_json()
      {
        return Err(Unsupported::ContentType(content_type.to_string()));
      }
      serde_json::from_slice(bytes)
        .map(Some)
        .map_err(|err| Unsupported::Malformed(err.to_string()))
    }
  }
}

// ---------------------------------------------------------------------------------------------
// The replay side (plan task 5.4): a recorded request out, a reply's headers back in.
// ---------------------------------------------------------------------------------------------

/// The pact's own recorded request, in the wire vocabulary a transport's `send` consumes
/// (component-interfaces spec §4) — method, path, query, headers, body.
///
/// **Nothing here re-derives the request from a shape**, because a v1–v4 pact has none: the
/// provider sees the bytes the consumer recorded, which is the same rule variant-semantics spec
/// §5.2 fixes for a Janus contract's recorded variant. The body travels base64-tagged with its
/// declared media type, exactly as a transport carries every other body (spec §4's "transports
/// carry octets, not documents"), so a non-JSON body replays unchanged even though design 3.5
/// cannot compile a plan for it.
///
/// Header and query names are lower-cased and grouped the way the HTTP transport's own serve side
/// records them, so a replayed request is indistinguishable from a recorded one.
pub fn request_parts(request: &HttpRequest) -> Parts {
  let mut part: Part = Part::new();
  part.insert("method".to_string(), text_slot(&request.method));
  part.insert("path".to_string(), text_slot(&request.path));

  let query = convert_query(request.query.as_ref());
  if !query.is_empty() {
    part.insert("query".to_string(), multi_map_slot(query.into_iter()));
  }
  let headers: BTreeMap<String, Vec<String>> = request
    .headers
    .as_ref()
    .map(|map| {
      map
        .iter()
        .map(|(name, values)| (name.to_ascii_lowercase(), values.clone()))
        .collect()
    })
    .unwrap_or_default();
  if !headers.is_empty() {
    part.insert("headers".to_string(), multi_map_slot(headers.into_iter()));
  }
  if let OptionalBody::Present(bytes, content_type, _) = &request.body {
    part.insert(
      "body".to_string(),
      SlotValue {
        content: Value::String(BASE64.encode(bytes)),
        encoded: Some("base64".to_string()),
        content_type: content_type.as_ref().map(ToString::to_string),
      },
    );
  }

  let mut parts: Parts = Parts::new();
  parts.insert("request".to_string(), part);
  parts
}

/// The pact's own recorded response, in the same wire vocabulary [`request_parts`] uses — status,
/// headers, body. What a contract's recorded variant carries alongside the request (contract-file
/// spec §8.3: "its `parts` are the pact's request and response examples"), so an upgraded contract
/// holds the same evidence the pact did.
pub fn response_parts(response: &HttpResponse) -> Parts {
  let mut part: Part = Part::new();
  part.insert(
    "status".to_string(),
    SlotValue {
      content: Value::from(response.status),
      encoded: None,
      content_type: None,
    },
  );
  let headers: BTreeMap<String, Vec<String>> = response
    .headers
    .as_ref()
    .map(|map| {
      map
        .iter()
        .map(|(name, values)| (name.to_ascii_lowercase(), values.clone()))
        .collect()
    })
    .unwrap_or_default();
  if !headers.is_empty() {
    part.insert("headers".to_string(), multi_map_slot(headers.into_iter()));
  }
  if let OptionalBody::Present(bytes, content_type, _) = &response.body {
    part.insert(
      "body".to_string(),
      SlotValue {
        content: Value::String(BASE64.encode(bytes)),
        encoded: Some("base64".to_string()),
        content_type: content_type.as_ref().map(ToString::to_string),
      },
    );
  }

  let mut parts: Parts = Parts::new();
  parts.insert("response".to_string(), part);
  parts
}

/// A reply's header slots as the single-valued captures design 3.5's compiled plans resolve
/// against: `$.<part>.headers.<lower-cased name>`, one string per name.
///
/// The wire form is `{name: [value, …]}` (component-interfaces spec §4's own worked example) and
/// the *expected* side went through [`convert_headers`], which joins a repeated header with
/// `", "`. This is that same join on the actual side, so a legacy plan compares a header value
/// against a header value rather than a string against a list. Layered over the generic
/// [`parts_resolver`](crate::protocol) captures rather than replacing them: `CapturedValues`
/// resolves against the longest captured prefix, so these win for header paths and change nothing
/// else.
///
/// Permanently HTTP-shaped, like the rest of this module — v1–v4 pacts are HTTP documents by
/// specification, and the compiler that reads them says so in its own module docs.
pub fn header_captures(resolver: CapturedValues, parts: &Parts) -> CapturedValues {
  let mut resolver = resolver;
  for (part_name, slots) in parts {
    let Some(slot) = slots.get("headers") else {
      continue;
    };
    let Value::Object(headers) = &slot.content else {
      continue;
    };
    for (name, value) in headers {
      let text = match value {
        Value::Array(values) => values.iter().map(header_text).collect::<Vec<_>>().join(", "),
        other => header_text(other),
      };
      resolver = resolver.capture(
        format!("$.{part_name}.headers.{}", name.to_ascii_lowercase()),
        RuntimeValue::String(text),
      );
    }
  }
  resolver
}

/// A header value's text. A JSON string is its own contents; anything else a transport put in a
/// header slot is rendered rather than dropped, so a malformed slot produces a readable mismatch
/// instead of a silent absence.
fn header_text(value: &Value) -> String {
  match value {
    Value::String(text) => text.clone(),
    other => other.to_string(),
  }
}

fn text_slot(text: &str) -> SlotValue {
  SlotValue {
    content: Value::String(text.to_string()),
    encoded: None,
    content_type: None,
  }
}

/// `{name: [values…]}`, the shape the HTTP transport reads back for headers and query parameters.
fn multi_map_slot(entries: impl Iterator<Item = (String, Vec<String>)>) -> SlotValue {
  SlotValue {
    content: Value::Object(
      entries
        .map(|(name, values)| {
          (
            name,
            Value::Array(values.into_iter().map(Value::String).collect()),
          )
        })
        .collect(),
    ),
    encoded: None,
    content_type: None,
  }
}
