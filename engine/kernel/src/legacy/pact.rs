//! v1–v4 pact reading (plan task 3.1), via `pact_models` — the Phase 0.4 reuse decision
//! (`Documentation/prototype/reuse-inventory.md`).
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
//! reply's headers in the single-valued form design 3.5's plans resolve against. A request with
//! generators to apply is kept as a [`RequestTemplate`], so the verification path never holds a
//! `pact_models` type (the module docs of [`crate::legacy`]).

use super::compile::{LegacyBody, LegacyRequest, LegacyResponse};
use crate::common::ContentTypes;
use crate::component::{
  ComponentError, Generate, GeneratorComponent, PactGenerators, Part, Parts, SlotValue,
};
use crate::contract::{ContractError, Problem};
use crate::plan::{CapturedValues, RuntimeValue};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_models::bodies::OptionalBody;
use pact_models::content_types::ContentType;
use pact_models::generators::form_urlencoded::FormUrlEncodedHandler;
use pact_models::generators::xml::XmlHandler;
use pact_models::generators::{
  ContentTypeHandler, GenerateValue, Generator, GeneratorCategory, GeneratorTestMode, JsonHandler,
  NoopVariantMatcher, VariantMatcher,
};
use pact_models::http_parts::HttpPart;
use pact_models::pact::Pact;
use pact_models::path_exp::DocPath;
use pact_models::provider_states::ProviderState;
use pact_models::v4::http_parts::{HttpRequest, HttpResponse};
use serde_json::{Map, Value, json};
use std::cell::RefCell;
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
  /// The interaction's id (`_id`): a Pact Broker adds one to every interaction it serves, and a
  /// published result is keyed by it (engine-protocol spec §8.3).
  pub id: Option<String>,
  /// A v4 interaction marked pending: its failure is reported and does not fail the run, as a
  /// pending pact's does.
  pub pending: bool,
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
        id: http.id,
        pending: http.pending,
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
/// to be JSON, or a form, and was not is a problem with the *document*.
#[derive(Debug, Clone)]
pub enum Unsupported {
  /// A body of a kind the legacy compiler does not compile yet: multipart (S1.23).
  ContentType(String),
  /// A body that did not parse as the type it claimed: what went wrong, naming the type.
  Malformed(String),
}

impl std::fmt::Display for Unsupported {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Unsupported::ContentType(content_type) => write!(
        f,
        "legacy_pact: a '{content_type}' body is not compiled yet (multipart bodies are staged \
         plan S1.23)"
      ),
      Unsupported::Malformed(message) => write!(f, "legacy_pact: {message}"),
    }
  }
}

/// [`HttpRequest`] to design 3.5's own input type. The only fallible part is the body, read by its
/// content type ([`legacy_body`]); `Err` names what content type defeated it rather than guessing.
pub fn legacy_request(request: &HttpRequest) -> Result<LegacyRequest, Unsupported> {
  Ok(LegacyRequest {
    method: request.method.clone(),
    path: request.path.clone(),
    query: convert_query(request.query.as_ref()),
    headers: convert_headers(request.headers.as_ref()),
    body: legacy_body(request)?,
    content_type: body_type(request),
    matching_rules: request.matching_rules.clone(),
  })
}

/// [`HttpResponse`] to design 3.5's own input type. See [`legacy_request`] for the body note.
pub fn legacy_response(response: &HttpResponse) -> Result<LegacyResponse, Unsupported> {
  Ok(LegacyResponse {
    status: response.status as u64,
    headers: convert_headers(response.headers.as_ref()),
    body: legacy_body(response)?,
    content_type: body_type(response),
    matching_rules: response.matching_rules.clone(),
  })
}

/// How today's verifier compares a body of a content type (`pact_matching::compare_bodies`'
/// `BODY_MATCHERS`, in its order), which is how the legacy layer reads one (ADR 0036 decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyKind {
  Json,
  /// `application/x-www-form-urlencoded`: compared as a query string (ADR 0038).
  Form,
  /// An XML type: compared element by element (ADR 0039).
  Xml,
  /// Multipart: a matcher of its own today, and S1.23 here.
  NotYet,
  /// A binary type (`image`, `audio`, `video`, `font`, `application/octet-stream`, and anything
  /// `pact_models` does not count as text): compared as octets.
  Bytes,
  /// Every other type: compared as text.
  Text,
}

impl BodyKind {
  fn of(content_type: &ContentType) -> BodyKind {
    if content_type.is_json() {
      BodyKind::Json
    } else if content_type.base_type() == FORM {
      BodyKind::Form
    } else if content_type.is_xml() {
      BodyKind::Xml
    } else if content_type.main_type == "multipart" {
      BodyKind::NotYet
    } else if content_type.is_binary() || content_type.base_type() == "application/octet-stream" {
      BodyKind::Bytes
    } else {
      BodyKind::Text
    }
  }
}

/// The form media type, which today's verifier compares as a query string.
const FORM: &str = "application/x-www-form-urlencoded";

/// A form body's names and values, as the built-in form component decodes one (ADR 0038 decision 2):
/// each name once, in the order it first appears, with its values in order. Parsed with
/// `serde_urlencoded`, as today's verifier parses it.
fn form_document(body: &[u8]) -> Result<RuntimeValue, serde_urlencoded::de::Error> {
  let pairs: Vec<(String, String)> = serde_urlencoded::from_bytes(body)?;
  let mut names: indexmap::IndexMap<String, Vec<RuntimeValue>> = indexmap::IndexMap::new();
  for (name, value) in pairs {
    names.entry(name).or_default().push(RuntimeValue::String(value));
  }
  Ok(RuntimeValue::Object(
    names
      .into_iter()
      .map(|(name, values)| (name, RuntimeValue::Array(values)))
      .collect(),
  ))
}

/// The part's content type as today's verifier takes it (`HttpPart::content_type`): the body's own,
/// else the `Content-Type` header, else one detected from the body; unknown (`*/*`) when none is.
fn part_content_type(part: &dyn HttpPart) -> ContentType {
  part.content_type().unwrap_or_default()
}

/// [`OptionalBody`]'s three "nothing here" variants collapse to design 3.5's two-state model
/// (`LegacyRequest::body`'s own doc comment): `Missing` (the interaction never mentioned a body)
/// is `None`; `Null` and `Empty` (a body was declared — explicitly null, or present with zero
/// bytes) both become [`LegacyBody::Empty`], which design 3.5's compiler treats as "assert the
/// actual body is absent too". A present body is read by its content type ([`BodyKind`]) into a
/// document: JSON is parsed, a form is its names and values, XML is its elements (ADR 0039), text is
/// a string, a binary body is its octets.
fn legacy_body(part: &dyn HttpPart) -> Result<Option<LegacyBody>, Unsupported> {
  let bytes = match part.body() {
    OptionalBody::Missing => return Ok(None),
    OptionalBody::Null | OptionalBody::Empty => return Ok(Some(LegacyBody::Empty)),
    OptionalBody::Present(bytes, _, _) => bytes,
  };
  let content_type = part_content_type(part);
  match BodyKind::of(&content_type) {
    BodyKind::Json => serde_json::from_slice::<Value>(bytes)
      .map(|document| Some(LegacyBody::Document(RuntimeValue::from_json(&document))))
      .map_err(|err| Unsupported::Malformed(format!("body is not valid JSON: {err}"))),
    BodyKind::Form => form_document(bytes)
      .map(|form| Some(LegacyBody::Form(form)))
      .map_err(|err| Unsupported::Malformed(format!("body is not a valid form: {err}"))),
    BodyKind::Xml => super::xml::document(bytes)
      .map(|document| Some(LegacyBody::Xml(document)))
      .map_err(|err| Unsupported::Malformed(format!("body is not valid XML: {err}"))),
    BodyKind::NotYet => Err(Unsupported::ContentType(content_type.to_string())),
    BodyKind::Bytes => Ok(Some(LegacyBody::Document(RuntimeValue::Bytes(bytes.to_vec())))),
    BodyKind::Text => Ok(Some(LegacyBody::Document(
      match String::from_utf8(bytes.to_vec()) {
        Ok(text) => RuntimeValue::String(text),
        Err(err) => RuntimeValue::Bytes(err.into_bytes()),
      },
    ))),
  }
}

/// The content type of a part's body, when it has one with content: the type a received body's
/// must agree with, and the one it is decoded as.
fn body_type(part: &dyn HttpPart) -> Option<String> {
  part
    .body()
    .is_present()
    .then(|| part_content_type(part).to_string())
}

/// The content types a provider's reply is decoded under (ADR 0036 decision 4): the pact's own for
/// its response body, which a verification declares as a contract declares a slot's
/// (contract-file spec §5.5), whatever the reply labels it. Whether the label agrees is the plan's
/// to check (`legacy:body-type`). A text body that is not UTF-8 was read as octets, so its reply is
/// declared as octets too, which no component claims. Empty when the response has no body.
pub fn response_types(response: &LegacyResponse) -> ContentTypes {
  let mut types = ContentTypes::new();
  if let Some(content_type) = &response.content_type {
    let is_text =
      ContentType::parse(content_type.as_str()).is_ok_and(|parsed| BodyKind::of(&parsed) == BodyKind::Text);
    let declared = match &response.body {
      Some(LegacyBody::Document(RuntimeValue::Bytes(_))) if is_text => "application/octet-stream".to_string(),
      _ => content_type.clone(),
    };
    types.insert(
      "response".to_string(),
      BTreeMap::from([("body".to_string(), declared)]),
    );
  }
  types
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

/// The request categories a provider-side run generates into, in the order today's verifier applies
/// them.
const REQUEST_CATEGORIES: [GeneratorCategory; 4] = [
  GeneratorCategory::PATH,
  GeneratorCategory::HEADER,
  GeneratorCategory::QUERY,
  GeneratorCategory::BODY,
];

/// Whether `request` has a generator a provider-side run applies (ADR 0034): any but the consumer
/// side's own (`MockServerURL`).
fn has_request_generators(request: &HttpRequest) -> bool {
  REQUEST_CATEGORIES.iter().any(|category| {
    request
      .build_generators(category)
      .values()
      .any(|generator| generator.corresponds_to_mode(&GeneratorTestMode::Provider))
  })
}

/// A pact's recorded request, kept because it has generators to apply when it is sent (ADR 0034):
/// a `ProviderState` generator reads what the exchange's states returned, so the request is
/// generated after they are set up. Opaque outside the v1–v4 boundary.
#[derive(Debug, Clone)]
pub struct RequestTemplate(HttpRequest);

impl RequestTemplate {
  /// The request, when it has a generator the provider side runs; `None` replays it as recorded.
  pub fn of(request: &HttpRequest) -> Option<RequestTemplate> {
    has_request_generators(request).then(|| RequestTemplate(request.clone()))
  }

  /// The request with its generators applied, as the parts a transport sends, and every generator
  /// that produced nothing.
  pub fn generate(
    &self,
    provider_state: &Map<String, Value>,
    generators: &dyn GeneratorComponent,
  ) -> (Parts, Vec<GeneratorFailure>) {
    let (generated, failures) = generate_request(&self.0, provider_state, generators);
    (request_parts(&generated), failures)
  }
}

/// One generator that produced no value, so the recorded one was sent (ADR 0034 decision 5).
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorFailure {
  /// Where the value would have gone, as a plan path: `$.request.path`,
  /// `$.request.headers.x-trace`, `$.request.body.items[0].id`.
  pub target: String,
  /// The generator, qualified: `pact:ProviderState`.
  pub generator: String,
  pub message: String,
}

/// `request` with its generators applied, as today's verifier applies them before sending
/// (`pact_matching::generate_request` in provider mode; ADR 0034 decision 2): the path, each
/// header and query parameter, and every JSON body value a generator's path resolves to. Values
/// come from `generators`, asked once per target with the recorded value as `example` and
/// `provider_state` as the exchange's provider-state values. A target whose generator fails keeps
/// its recorded value and is reported, not fatal: today's verifier keeps it silently.
fn generate_request(
  request: &HttpRequest,
  provider_state: &Map<String, Value>,
  generators: &dyn GeneratorComponent,
) -> (HttpRequest, Vec<GeneratorFailure>) {
  let mut request = request.clone();
  let mut failures = Vec::new();
  let state = Value::Object(provider_state.clone());
  let ask = |generator: &Generator, example: Value| generate_one(generators, generator, example, &state);

  for (_, generator) in provider_side(&request, &GeneratorCategory::PATH) {
    match ask(&generator, Value::String(request.path.clone())) {
      Ok(value) => request.path = text_of(&value),
      Err(err) => failures.push(failure("$.request.path", &generator, err)),
    }
  }

  for (key, generator) in provider_side(&request, &GeneratorCategory::HEADER) {
    let Some(name) = key.first_field() else { continue };
    let target = format!("$.request.headers.{}", name.to_ascii_lowercase());
    let headers = request.headers.get_or_insert_with(HashMap::new);
    let existing = headers.keys().find(|k| k.eq_ignore_ascii_case(name)).cloned();
    // A header generates one value from its first, as `GenerateValue<Vec<String>>` does.
    let example = existing
      .as_ref()
      .and_then(|k| headers.get(k))
      .and_then(|values| values.first().cloned())
      .unwrap_or_default();
    match ask(&generator, Value::String(example)) {
      Ok(value) => {
        headers.insert(
          existing.unwrap_or_else(|| name.to_string()),
          vec![text_of(&value)],
        );
      }
      Err(err) => failures.push(failure(&target, &generator, err)),
    }
  }

  for (key, generator) in provider_side(&request, &GeneratorCategory::QUERY) {
    let Some(name) = key.first_field() else { continue };
    let target = format!("$.request.query.{name}");
    let query = request.query.get_or_insert_with(HashMap::new);
    let values = query
      .get(name)
      .cloned()
      .unwrap_or_else(|| vec![Some(String::new())]);
    let mut generated = Vec::with_capacity(values.len());
    for value in values {
      match ask(&generator, Value::String(value.clone().unwrap_or_default())) {
        Ok(new) => generated.push(Some(text_of(&new))),
        Err(err) => {
          failures.push(failure(&target, &generator, err));
          generated.push(value);
        }
      }
    }
    query.insert(name.to_string(), generated);
  }

  let body_generators = provider_side(&request, &GeneratorCategory::BODY);
  if !body_generators.is_empty()
    && let OptionalBody::Present(bytes, content_type, encoding) = &request.body
  {
    let kind = request.content_type().map(|ct| BodyKind::of(&ct));
    let matcher: Box<dyn VariantMatcher + Send + Sync> = NoopVariantMatcher.boxed();
    // Each generator is asked through `pact_models`' own handler for the body's type, which resolves
    // its path as today's verifier does, and the failures it would drop are kept.
    let mut apply = |apply_key: &mut dyn FnMut(&DocPath, &Asking<'_, _>)| {
      for (key, generator) in &body_generators {
        let asking = Asking {
          ask: &ask,
          generator,
          failures: RefCell::new(Vec::new()),
        };
        apply_key(key, &asking);
        let target = body_target(key);
        failures.extend(
          asking
            .failures
            .into_inner()
            .into_iter()
            .map(|err| failure(&target, generator, err)),
        );
      }
    };
    let generated: Option<Vec<u8>> = match kind {
      Some(BodyKind::Json) => serde_json::from_slice::<Value>(bytes).ok().map(|document| {
        let mut handler = JsonHandler { value: document };
        apply(&mut |key, asking| handler.apply_key(key, asking, &HashMap::new(), &matcher));
        handler.value.to_string().into_bytes()
      }),
      // A form body is its pairs, each generator applied to one occurrence of its name, and written
      // back as today's verifier writes it (ADR 0038 decision 5).
      Some(BodyKind::Form) => serde_urlencoded::from_bytes::<Vec<(String, String)>>(bytes)
        .ok()
        .and_then(|params| {
          let mut handler = FormUrlEncodedHandler { params };
          apply(&mut |key, asking| handler.apply_key(key, asking, &HashMap::new(), &matcher));
          serde_urlencoded::to_string(&handler.params).ok()
        })
        .map(String::into_bytes),
      // An XML body's generators go to its attributes and text, by today's handler, and the document
      // is written back as today's verifier writes it (ADR 0039 decision 6).
      Some(BodyKind::Xml) => pact_models::xml_utils::parse_bytes(bytes)
        .ok()
        .and_then(|package| {
          let mut handler = XmlHandler {
            value: package.as_document(),
          };
          apply(&mut |key, asking| handler.apply_key(key, asking, &HashMap::new(), &matcher));
          let mut written = Vec::new();
          sxd_document::writer::format_document(&handler.value, &mut written)
            .ok()
            .map(|()| written)
        }),
      _ => None,
    };
    match generated {
      Some(body) => request.body = OptionalBody::Present(body.into(), content_type.clone(), *encoding),
      None => {
        // A multipart body, or one that is not what it says it is, is sent as recorded and every
        // generator on it says so.
        let media_type = request
          .content_type()
          .map(|ct| ct.to_string())
          .unwrap_or_else(|| "an unknown content type".to_string());
        for (key, generator) in &body_generators {
          failures.push(GeneratorFailure {
            target: body_target(key),
            generator: PactGenerators::qualified(&generator.name()),
            message: format!(
              "a body generator applies to a JSON, form or XML body, and this body is {media_type}; the recorded value was sent"
            ),
          });
        }
      }
    }
  }

  (request, failures)
}

/// A category's generators that run on the provider side, in a stable order so the same pact
/// generates in the same order every run.
fn provider_side(request: &HttpRequest, category: &GeneratorCategory) -> Vec<(DocPath, Generator)> {
  let mut generators: Vec<(DocPath, Generator)> = request
    .build_generators(category)
    .into_iter()
    .filter(|(_, generator)| generator.corresponds_to_mode(&GeneratorTestMode::Provider))
    .collect();
  generators.sort_by_key(|(path, _)| path.to_string());
  generators
}

/// Ask the generator component for one value (component-interfaces spec §7.3).
fn generate_one(
  generators: &dyn GeneratorComponent,
  generator: &Generator,
  example: Value,
  provider_state: &Value,
) -> Result<Value, ComponentError> {
  let mut config = generator
    .to_json()
    .and_then(|json| json.as_object().cloned())
    .unwrap_or_default();
  config.remove("type");
  generators
    .generate(Generate {
      generator: PactGenerators::qualified(&generator.name()),
      config: Some(Value::Object(config)),
      context: Some(json!({ "example": example, "provider-state": provider_state })),
    })
    .map(|result| result.value)
}

/// The generator component, in the form `pact_models`' body handler asks for a value: it resolves
/// the generator's path to every value it names, and this answers for each of them, keeping what
/// went wrong, which the handler would otherwise drop.
struct Asking<'a, F: Fn(&Generator, Value) -> Result<Value, ComponentError>> {
  ask: &'a F,
  generator: &'a Generator,
  failures: RefCell<Vec<ComponentError>>,
}

/// A form value is text, so it is asked for as a string and the answer taken as text.
impl<F: Fn(&Generator, Value) -> Result<Value, ComponentError>> GenerateValue<String> for Asking<'_, F> {
  fn generate_value(
    &self,
    value: &String,
    _context: &HashMap<&str, Value>,
    _matcher: &Box<dyn VariantMatcher + Send + Sync>,
  ) -> anyhow::Result<String> {
    (self.ask)(self.generator, Value::String(value.clone()))
      .map(|value| text_of(&value))
      .map_err(|err| {
        let message = err.message.clone();
        self.failures.borrow_mut().push(err);
        anyhow::anyhow!(message)
      })
  }
}

impl<F: Fn(&Generator, Value) -> Result<Value, ComponentError>> GenerateValue<Value> for Asking<'_, F> {
  fn generate_value(
    &self,
    value: &Value,
    _context: &HashMap<&str, Value>,
    _matcher: &Box<dyn VariantMatcher + Send + Sync>,
  ) -> anyhow::Result<Value> {
    (self.ask)(self.generator, value.clone()).map_err(|err| {
      let message = err.message.clone();
      self.failures.borrow_mut().push(err);
      anyhow::anyhow!(message)
    })
  }
}

fn failure(target: &str, generator: &Generator, err: ComponentError) -> GeneratorFailure {
  GeneratorFailure {
    target: target.to_string(),
    generator: PactGenerators::qualified(&generator.name()),
    message: err.message,
  }
}

/// A body generator's path as a plan path: `$.items[0].id` is `$.request.body.items[0].id`.
fn body_target(key: &DocPath) -> String {
  let path = key.to_string();
  format!("$.request.body{}", path.strip_prefix('$').unwrap_or(&path))
}

/// A generated value as the text a path, header or query parameter carries.
fn text_of(value: &Value) -> String {
  match value {
    Value::String(text) => text.clone(),
    other => other.to_string(),
  }
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
