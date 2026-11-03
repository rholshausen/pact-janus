//! `consumer-session/add-pact-interaction` (ADR 0066): the calls a consumer library makes to `pact_ffi` to
//! build one interaction, applied as the FFI applies them, then converted as `janus upgrade` converts
//! a pact's interaction.
//!
//! A library that builds its interactions through the FFI (pact-js, pact-python, pact-go, pact-net,
//! pact-php) is a list of these calls, so a compatibility facade over one records the calls rather
//! than making them, and the engine makes them here. Each step is `{ call, … }`, named after the FFI
//! function without its `pactffi_` prefix, with its arguments by name. Where the FFI takes integration
//! JSON in a string, a step may carry the JSON value itself: a string is the text as given and any
//! other value its JSON text, which is what the library would have passed. A file the FFI would read
//! the host has read: the step carries its name and its octets, base64-encoded.
//!
//! Ported from pact-reference's `pact_ffi` at `1f1c2e61` (`src/mock_server/handles.rs`:
//! `pactffi_upon_receiving`, `pactffi_given`, `pactffi_given_with_params`, `pactffi_with_request`,
//! `pactffi_with_query_parameter`, `pactffi_with_header_v2`, `pactffi_response_status_v2`,
//! `process_body`, `pactffi_with_body`, `pactffi_with_binary_file`, `pactffi_with_matching_rules` and
//! `pactffi_with_multipart_file_v2`; `src/mock_server/bodies.rs`: `request_multipart`,
//! `response_multipart` and `add_part_to_multipart`). Two things differ, as ADR 0066 decision 7
//! says: a multipart boundary is derived from the part, not drawn at random, and a part's
//! `Content-Type` is the one the call names, not one guessed from the file's extension.

pub mod form;
pub mod integration;
pub mod xml;

use super::upgrade::{self, Finding, Options};
use crate::contract::ValuePart;
use crate::error::Problem;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use either::Either;
use integration::{Problems, from_integration_json, from_integration_json_v2, process_json};
use pact_models::PactSpecification;
use pact_models::bodies::OptionalBody;
use pact_models::content_types::{ContentType, TEXT, detect_content_type_from_string};
use pact_models::generators::Generators;
use pact_models::headers::parse_header;
use pact_models::http_parts::HttpPart;
use pact_models::matchingrules::{MatchingRule, MatchingRules, RuleLogic, matchers_from_json};
use pact_models::path_exp::DocPath;
use pact_models::provider_states::ProviderState;
use pact_models::v4::http_parts::{HttpRequest, HttpResponse};
use pact_models::v4::interaction::V4Interaction;
use pact_models::v4::synch_http::SynchronousHttp;
use regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

const CONTENT_TYPE: &str = "Content-Type";

/// What `add-pact-interaction` hands on: the converted interaction as an interaction specification,
/// the example a mock serves for it, and what the conversion lost or decided, its pointers into the
/// built interaction.
pub struct Built {
  pub spec: Value,
  /// Part -> slot -> value: the interaction's own example, as `upgrade` records it for the base
  /// variant, which a mock serves as today's mock server does. A shape cannot always give it back:
  /// an `eachKey` rule converts to an `each-entry` whose values are open and whose least size is 0,
  /// from which no entry is generated.
  pub example: BTreeMap<String, ValuePart>,
  pub findings: Vec<Finding>,
}

/// Build the interaction `pact`'s steps describe and convert it. Every problem with a step is
/// reported, each pointing at it under `/pact/steps`.
pub fn interaction(pact: &Value) -> Result<Built, Vec<Problem>> {
  convert(build(pact)?)
}

/// The v4 interaction `pact`'s steps build, as `pact_ffi` would have built it.
fn build(pact: &Value) -> Result<SynchronousHttp, Vec<Problem>> {
  let Some(form) = pact.as_object() else {
    return Err(vec![problem(
      "/pact",
      "'pact' is not an object of 'specification' and 'steps'",
    )]);
  };
  let specification = match form.get("specification") {
    None => PactSpecification::V4,
    Some(Value::String(version)) => match version.as_str() {
      "V2" => PactSpecification::V2,
      "V3" => PactSpecification::V3,
      "V4" => PactSpecification::V4,
      _ => {
        return Err(vec![problem(
          "/pact/specification",
          &format!("'{version}' is not V2, V3 or V4"),
        )]);
      }
    },
    Some(other) => {
      return Err(vec![problem(
        "/pact/specification",
        &format!("{other} is not V2, V3 or V4"),
      )]);
    }
  };
  let Some(Value::Array(steps)) = form.get("steps") else {
    return Err(vec![problem("/pact/steps", "'steps' is missing or not a list")]);
  };

  let mut built = SynchronousHttp::default();
  let mut problems = Vec::new();
  for (index, step) in steps.iter().enumerate() {
    let at = format!("/pact/steps/{index}");
    let mut found = Problems::new();
    match Step::read(step, &at) {
      Ok(step) => step.apply(&mut built, specification, &mut found),
      Err(problem) => problems.push(problem),
    }
    problems.extend(found.into_iter().map(|message| problem(&at, &message)));
  }
  if !problems.is_empty() {
    return Err(problems);
  }
  Ok(built)
}

/// Upgrade the built interaction as a one-interaction v4 pact (ADR 0066 decision 3).
fn convert(built: SynchronousHttp) -> Result<Built, Vec<Problem>> {
  let document = json!({
    "consumer": { "name": "consumer" },
    "provider": { "name": "provider" },
    "interactions": [ built.to_json() ],
    "metadata": { "pactSpecification": { "version": "4.0" } }
  });
  let upgraded = upgrade::pact_with("the interaction its steps built", &document, &Options::default())
    .map_err(|err| match err {
      crate::contract::ContractError::Invalid { problems } => problems,
      other => vec![problem("/pact", &other.to_string())],
    })?;
  let Some(interaction) = upgraded.contract.interactions.first() else {
    // Only a pact plugin's interaction is not converted, and no step asks for one.
    let said = upgraded
      .findings
      .iter()
      .map(|f| f.message.clone())
      .collect::<Vec<_>>()
      .join("; ");
    return Err(vec![problem(
      "/pact",
      &format!("the steps built an interaction with no contract form: {said}"),
    )]);
  };
  let findings = upgraded
    .findings
    .into_iter()
    .map(|mut finding| {
      // Pointers into the built interaction and its specification, not into a one-interaction pact
      // and contract nobody wrote.
      finding.path = finding
        .path
        .strip_prefix("/interactions/0")
        .unwrap_or(&finding.path)
        .to_string();
      finding.target = finding.target.map(|target| {
        target
          .strip_prefix("/interactions/0")
          .unwrap_or(&target)
          .to_string()
      });
      finding
    })
    .collect();
  let example = interaction
    .selection
    .variants
    .first()
    .map(|base| base.parts.clone())
    .unwrap_or_default();
  Ok(Built {
    spec: interaction.spec_document(),
    example,
    findings,
  })
}

fn problem(pointer: &str, message: &str) -> Problem {
  Problem {
    pointer: pointer.to_string(),
    message: message.to_string(),
  }
}

/// The part of an HTTP interaction a step names.
#[derive(Clone, Copy)]
enum Part {
  Request,
  Response,
}

/// One FFI call, its arguments read.
enum Step {
  UponReceiving(String),
  Given(String, Option<Value>),
  WithRequest {
    method: String,
    path: String,
  },
  WithQueryParameter {
    name: String,
    index: usize,
    value: String,
  },
  WithHeader {
    part: Part,
    name: String,
    index: usize,
    value: String,
  },
  ResponseStatus(String),
  WithBody {
    part: Part,
    content_type: Option<String>,
    body: String,
  },
  WithBinaryFile {
    part: Part,
    content_type: String,
    body: Vec<u8>,
  },
  WithMatchingRules {
    part: Part,
    rules: Map<String, Value>,
  },
  WithMultipartFile(Box<MultipartFile>),
}

struct MultipartFile {
  part: Part,
  content_type: String,
  file: String,
  contents: Vec<u8>,
  part_name: String,
  boundary: Option<String>,
}

/// A step's arguments, read by name, each failure pointing at the argument.
struct Args<'a> {
  step: &'a Map<String, Value>,
  at: &'a str,
}

impl Args<'_> {
  fn value(&self, name: &str) -> Result<&Value, Problem> {
    self
      .step
      .get(name)
      .ok_or_else(|| problem(&format!("{}/{name}", self.at), &format!("'{name}' is missing")))
  }

  fn string(&self, name: &str) -> Result<String, Problem> {
    match self.value(name)? {
      Value::String(text) => Ok(text.clone()),
      other => Err(problem(
        &format!("{}/{name}", self.at),
        &format!("'{name}' is {other}, not a string"),
      )),
    }
  }

  fn optional_string(&self, name: &str) -> Result<Option<String>, Problem> {
    match self.step.get(name) {
      None | Some(Value::Null) => Ok(None),
      Some(_) => self.string(name).map(Some),
    }
  }

  /// A value the FFI takes as text: a string as given, any other value its JSON text.
  fn text(&self, name: &str) -> Result<String, Problem> {
    Ok(match self.value(name)? {
      Value::String(text) => text.clone(),
      other => other.to_string(),
    })
  }

  fn index(&self) -> Result<usize, Problem> {
    match self.step.get("index") {
      None => Ok(0),
      Some(value) => value.as_u64().map(|index| index as usize).ok_or_else(|| {
        problem(
          &format!("{}/index", self.at),
          &format!("'index' is {value}, not a whole number"),
        )
      }),
    }
  }

  fn part(&self) -> Result<Part, Problem> {
    match self.string("part")?.as_str() {
      "request" => Ok(Part::Request),
      "response" => Ok(Part::Response),
      other => Err(problem(
        &format!("{}/part", self.at),
        &format!("'{other}' is not 'request' or 'response'"),
      )),
    }
  }

  fn octets(&self, name: &str) -> Result<Vec<u8>, Problem> {
    let text = self.string(name)?;
    BASE64.decode(text.as_bytes()).map_err(|err| {
      problem(
        &format!("{}/{name}", self.at),
        &format!("'{name}' is not base64: {err}"),
      )
    })
  }
}

impl Step {
  fn read(step: &Value, at: &str) -> Result<Step, Problem> {
    let Some(step) = step.as_object() else {
      return Err(problem(at, "a step is an object naming its 'call'"));
    };
    let args = Args { step, at };
    let call = args.string("call")?;
    Ok(match call.as_str() {
      "upon_receiving" => Step::UponReceiving(args.string("description")?),
      "given" => Step::Given(args.string("description")?, None),
      "given_with_params" => Step::Given(args.string("description")?, Some(args.value("params")?.clone())),
      "with_request" => Step::WithRequest {
        method: args.string("method")?,
        path: args.text("path")?,
      },
      "with_query_parameter" => Step::WithQueryParameter {
        name: args.string("name")?,
        index: args.index()?,
        value: args.text("value")?,
      },
      "with_header_v2" => Step::WithHeader {
        part: args.part()?,
        name: args.string("name")?,
        index: args.index()?,
        value: args.text("value")?,
      },
      "response_status_v2" => Step::ResponseStatus(args.text("status")?),
      "with_body" => Step::WithBody {
        part: args.part()?,
        content_type: args.optional_string("content_type")?,
        body: args.text("body")?,
      },
      "with_binary_file" => Step::WithBinaryFile {
        part: args.part()?,
        content_type: args.string("content_type")?,
        body: args.octets("body")?,
      },
      "with_matching_rules" => match args.value("rules")? {
        Value::Object(rules) => Step::WithMatchingRules {
          part: args.part()?,
          rules: rules.clone(),
        },
        other => {
          return Err(problem(
            &format!("{at}/rules"),
            &format!("'rules' is {other}, not an object"),
          ));
        }
      },
      "with_multipart_file_v2" => Step::WithMultipartFile(Box::new(MultipartFile {
        part: args.part()?,
        content_type: args.string("content_type")?,
        file: args.string("file")?,
        contents: args.octets("contents")?,
        part_name: args
          .optional_string("part_name")?
          .unwrap_or_else(|| "file".to_string()),
        boundary: args.optional_string("boundary")?,
      })),
      other => {
        return Err(problem(
          &format!("{at}/call"),
          &format!("'{other}' is not a call this engine makes (ADR 0066 lists them)"),
        ));
      }
    })
  }

  fn apply(self, built: &mut SynchronousHttp, specification: PactSpecification, problems: &mut Problems) {
    match self {
      Step::UponReceiving(description) => built.description = description,
      Step::Given(name, None) => built.provider_states.push(ProviderState::default(&name)),
      Step::Given(name, Some(params)) => {
        let params = match params {
          Value::Object(map) => map.into_iter().collect(),
          other => HashMap::from([("value".to_string(), other)]),
        };
        built.provider_states.push(ProviderState { name, params });
      }
      Step::WithRequest { method, path } => {
        let request = &mut built.request;
        let path = from_integration_json_v2(
          &mut request.matching_rules,
          &mut request.generators,
          &path,
          DocPath::empty(),
          "path",
          0,
          problems,
        );
        request.method = method;
        request.path = first(path);
      }
      Step::WithQueryParameter { name, index, value } => {
        with_query_parameter(built, &name, index, &value, problems)
      }
      Step::WithHeader {
        part,
        name,
        index,
        value,
      } => match part {
        Part::Request => with_header(&mut built.request, &name, index, &value, problems),
        Part::Response => with_header(&mut built.response, &name, index, &value, problems),
      },
      Step::ResponseStatus(status) => {
        let response = &mut built.response;
        let status = from_integration_json_v2(
          &mut response.matching_rules,
          &mut response.generators,
          &status,
          DocPath::empty(),
          "status",
          0,
          problems,
        );
        response.status = first(status).parse().unwrap_or_default();
      }
      Step::WithBody {
        part,
        content_type,
        body,
      } => {
        // A content type that does not parse falls back to detection, as the FFI's does.
        let content_type = content_type.and_then(|ct| ContentType::parse(ct.as_str()).ok());
        match part {
          Part::Request => with_body(&mut built.request, &body, &content_type, problems),
          Part::Response => with_body(&mut built.response, &body, &content_type, problems),
        }
      }
      Step::WithBinaryFile {
        part,
        content_type,
        body,
      } => {
        let rule = specification >= PactSpecification::V3;
        match part {
          Part::Request => with_binary_file(&mut built.request, &content_type, body, rule),
          Part::Response => with_binary_file(&mut built.response, &content_type, body, rule),
        }
      }
      Step::WithMatchingRules { part, rules } => {
        // Wrapped in `matchingRules` when not already, as `matchers_from_json` reads them.
        let rules = if rules.contains_key("matchingRules") {
          Value::Object(rules)
        } else {
          json!({ "matchingRules": rules })
        };
        match matchers_from_json(&rules, &None) {
          Ok(rules) => match part {
            Part::Request => built.request.matching_rules_mut().merge(&rules),
            Part::Response => built.response.matching_rules_mut().merge(&rules),
          },
          Err(err) => problems.push(format!("the matching rules cannot be read: {err}")),
        }
      }
      Step::WithMultipartFile(file) => match file.part {
        Part::Request => with_multipart_file(&mut built.request, &file),
        Part::Response => with_multipart_file(&mut built.response, &file),
      },
    }
  }
}

/// The FFI keeps the first of several values where it can hold one.
fn first(value: Either<String, Vec<String>>) -> String {
  match value {
    Either::Left(value) => value,
    Either::Right(values) => values.into_iter().next().unwrap_or_default(),
  }
}

/// `pactffi_with_query_parameter`: the value at `index` of the parameter `name`.
fn with_query_parameter(
  built: &mut SynchronousHttp,
  name: &str,
  index: usize,
  value: &str,
  problems: &mut Problems,
) {
  let request = &mut built.request;
  let mut path = DocPath::root();
  path.push_field(name).push_index(index);
  let value = from_integration_json(
    &mut request.matching_rules,
    &mut request.generators,
    value,
    path,
    "query",
    problems,
  );
  let query = request.query.get_or_insert_with(HashMap::new);
  let values = query.entry(name.to_string()).or_default();
  if index >= values.len() {
    values.resize_with(index + 1, Default::default);
  }
  values[index] = Some(value);
}

/// The two parts of an HTTP interaction, as the FFI reaches into them: their rules and generators
/// together, and the inherent `content_type` and `set_header` the FFI calls.
trait Fields: HttpPart {
  fn rules_and_generators(&mut self) -> (&mut MatchingRules, &mut Generators);
  fn part_content_type(&self) -> Option<ContentType>;
  /// Replaces a header's values, keeping the spelling of a name already set in another case.
  fn replace_header(&mut self, name: &str, values: &[&str]);
}

macro_rules! fields {
  ($part:ty) => {
    impl Fields for $part {
      fn rules_and_generators(&mut self) -> (&mut MatchingRules, &mut Generators) {
        (&mut self.matching_rules, &mut self.generators)
      }

      fn part_content_type(&self) -> Option<ContentType> {
        self.content_type()
      }

      fn replace_header(&mut self, name: &str, values: &[&str]) {
        self.set_header(name, values);
      }
    }
  };
}

fields!(HttpRequest);
fields!(HttpResponse);

/// `pactffi_with_header_v2`: the value at `index` of the header `name`, which a value at index 0 may
/// split into several (a header the FFI parses as a list).
fn with_header<P: Fields>(http: &mut P, name: &str, index: usize, value: &str, problems: &mut Problems) {
  let mut path = DocPath::root();
  path.push_field(name);
  if index > 0 {
    path.push_index(index);
  }
  let (rules, generators) = http.rules_and_generators();
  let value = from_integration_json_v2(rules, generators, value, path, "header", index, problems);
  let values = match value {
    Either::Right(values) => {
      // Several values as a JSON list replace the header's values.
      let values: Vec<&str> = values.iter().map(String::as_str).collect();
      http.replace_header(name, &values);
      return;
    }
    Either::Left(value) => value,
  };
  let headers = http.headers_mut();
  // An existing key may be spelled in another case.
  let lookup = name.to_lowercase();
  let key = headers.keys().find(|key| key.to_lowercase() == lookup).cloned();
  if index > 0 {
    let existing = headers
      .entry(key.unwrap_or_else(|| name.to_string()))
      .or_default();
    if index >= existing.len() {
      existing.resize_with(index + 1, Default::default);
    }
    existing[index] = values;
    return;
  }
  match key {
    Some(key) => {
      let parsed = parse_header(key.as_str(), values.as_str());
      let existing = headers.get_mut(&key).expect("the key was found above");
      if existing.is_empty() {
        existing.extend_from_slice(parsed.as_slice());
      } else if parsed.is_empty() {
        existing[0] = String::default();
      } else if parsed.len() == 1 {
        existing[0] = values;
      } else {
        existing.clear();
        existing.extend_from_slice(parsed.as_slice());
      }
    }
    None => {
      headers.insert(name.to_string(), parse_header(name, values.as_str()));
    }
  }
}

/// `pactffi_with_body` for an HTTP part: the body processed by its content type, and a
/// `Content-Type` header for it unless one is set.
fn with_body<P: Fields>(
  http: &mut P,
  body: &str,
  content_type: &Option<ContentType>,
  problems: &mut Problems,
) {
  let hint = http.part_content_type();
  let (rules, generators) = http.rules_and_generators();
  let processed = process_body(body, content_type, &hint, rules, generators, problems);
  *http.body_mut() = processed;
  if let Some(content_type) = http.part_content_type() {
    http
      .headers_mut()
      .entry(CONTENT_TYPE.to_string())
      .or_insert_with(|| vec![content_type.to_string()]);
  }
}

/// The FFI's `process_body`: a body's text, processed by its content type (the call's, else the
/// part's, else detected from the text, else `text/plain`).
fn process_body(
  body: &str,
  content_type: &Option<ContentType>,
  hint: &Option<ContentType>,
  rules: &mut MatchingRules,
  generators: &mut Generators,
  problems: &mut Problems,
) -> OptionalBody {
  if body.is_empty() {
    return OptionalBody::Empty;
  }
  let detected = detect_content_type_from_string(body);
  let content_type = content_type
    .clone()
    .or_else(|| hint.clone())
    .or_else(|| detected.clone())
    .unwrap_or_else(|| TEXT.clone());
  let described_as_json = detected.as_ref().is_some_and(ContentType::is_json);
  let bytes = if content_type.is_json() {
    process_json(body, rules.add_category("body"), generators, problems).into_bytes()
  } else if content_type.is_xml() && described_as_json {
    match serde_json::from_str::<Value>(body) {
      Ok(Value::Object(document)) => {
        match xml::generate_xml_body(&document, rules.add_category("body"), generators) {
          Ok(bytes) => bytes,
          Err(message) => {
            problems.push(message);
            Vec::new()
          }
        }
      }
      _ => {
        problems.push("an XML builder document is a JSON object".to_string());
        Vec::new()
      }
    }
  } else if content_type.is_form_urlencoded() && described_as_json {
    form::process_form_urlencoded_json(body, rules.add_category("body"), generators, problems).into_bytes()
  } else {
    body.as_bytes().to_vec()
  };
  OptionalBody::Present(bytes.into(), Some(content_type), None)
}

/// `pactffi_with_binary_file` for an HTTP part: the octets as the body, a `Content-Type` header
/// unless one is set, and from v3 a rule that the body is of that type.
fn with_binary_file<P: Fields>(http: &mut P, content_type: &str, body: Vec<u8>, rule: bool) {
  *http.body_mut() = if body.is_empty() {
    OptionalBody::Empty
  } else {
    OptionalBody::Present(body.into(), None, None)
  };
  if !http.has_header(CONTENT_TYPE) {
    http
      .headers_mut()
      .insert(CONTENT_TYPE.to_string(), vec![content_type.to_string()]);
  }
  if rule {
    http.matching_rules_mut().add_category("body").add_rule(
      DocPath::root(),
      MatchingRule::ContentType(content_type.into()),
      RuleLogic::And,
    );
  }
}

static MULTIPART_MARKER: LazyLock<Regex> =
  LazyLock::new(|| Regex::new("\\-\\-([a-zA-Z0-9'\\(\\)+_,-.\\/:=? ]*)\r\n").expect("a valid pattern"));

/// `pactffi_with_multipart_file_v2` for an HTTP part: the file as a part of a `multipart/form-data`
/// body, appended to one the part already has, with a rule that the part is of the type named.
fn with_multipart_file<P: Fields>(http: &mut P, file: &MultipartFile) {
  let boundary = file.boundary.clone().unwrap_or_else(|| derived_boundary(file));
  let name = std::path::Path::new(&file.file)
    .file_name()
    .map(|name| name.to_string_lossy().into_owned())
    .unwrap_or_else(|| file.file.clone());
  let mut part = format!(
    "Content-Disposition: form-data; name=\"{}\"; filename=\"{name}\"\r\nContent-Type: {}\r\n\r\n",
    file.part_name, file.content_type
  )
  .into_bytes();
  part.extend_from_slice(&file.contents);
  part.extend_from_slice(b"\r\n");

  let existing = match http.body() {
    OptionalBody::Present(bytes, ..) => MULTIPART_MARKER
      .captures(&String::from_utf8_lossy(bytes))
      .and_then(|captures| {
        captures
          .get(1)
          .map(|marker| (bytes.clone(), marker.as_str().to_string()))
      }),
    _ => None,
  };
  if let Some((bytes, marker)) = existing {
    // A body with a multipart marker already: the part is added before its end marker, under its
    // boundary, and its header and rules are left as the first file set them.
    let end = format!("--{marker}--\r\n");
    let mut body = bytes.strip_suffix(end.as_bytes()).unwrap_or(&bytes).to_vec();
    body.extend(format!("--{marker}\r\n").into_bytes());
    body.extend(part);
    body.extend(end.into_bytes());
    let content_type = http.body().content_type();
    *http.body_mut() = OptionalBody::Present(body.into(), content_type, None);
  } else {
    let mut body = format!("--{boundary}\r\n").into_bytes();
    body.extend(part);
    body.extend(format!("--{boundary}--\r\n").into_bytes());
    let header = format!("multipart/form-data; boundary={boundary}");
    http.replace_header(CONTENT_TYPE, &[header.as_str()]);
    *http.body_mut() = OptionalBody::Present(body.into(), Some("multipart/form-data".into()), None);
    http.matching_rules_mut().add_category("header").add_rule(
      DocPath::new_unwrap("Content-Type"),
      MatchingRule::Regex(r"multipart/form-data;(\s*charset=[^;]*;)?\s*boundary=.*".into()),
      RuleLogic::And,
    );
  }
  let mut path = DocPath::root();
  path.push_field(file.part_name.as_str());
  http.matching_rules_mut().add_category("body").add_rule(
    path,
    MatchingRule::ContentType(file.content_type.as_str().into()),
    RuleLogic::And,
  );
}

/// A boundary that is the same for the same part on every run (ADR 0066 decision 7): FNV-1a over
/// the part's name, the file's name and its octets, as 16 hex digits.
fn derived_boundary(file: &MultipartFile) -> String {
  let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
  for byte in file
    .part_name
    .bytes()
    .chain([0])
    .chain(file.file.bytes())
    .chain([0])
    .chain(file.contents.iter().copied())
  {
    hash ^= u64::from(byte);
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
  }
  format!("{hash:016x}")
}

#[cfg(test)]
mod tests;
