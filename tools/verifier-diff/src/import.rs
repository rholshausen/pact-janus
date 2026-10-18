//! Importing the pact-compatibility-suite's HTTP provider scenarios as fixtures.
//!
//! The suite (Apache-2.0) describes each scenario in Gherkin: a table of interactions, a provider
//! that "returns the response from interaction N, with the following changes", the pacts to verify,
//! and whether the verification succeeds. Its own Rust harness, in pact-reference, builds those pacts
//! with `pact_models` and serves the provider from pact's mock server. This importer builds them the
//! same way — the same table rules, the same `pact_models` calls, each feature at its own spec
//! version — and writes each scenario as a fixture whose scripted provider answers what the mock
//! server would.
//!
//! A scenario this harness cannot express yet is skipped, with the plan item that will let it in,
//! and `import.json` lists every one. The suite's other assertions (a callback's calls, an error's
//! wording) are about pact-reference's internals; the fixture keeps only the verdict.

use crate::fixture::{
  BrokerSpec, BrokerVerify, Fixture, ProviderSpec, Published, Route, RunVerdict, StateCallback,
};
use bytes::Bytes;
use pact_models::bodies::OptionalBody;
use pact_models::content_types::{ContentType, JSON, TEXT, XML};
use pact_models::http_parts::HttpPart;
use pact_models::interaction::Interaction;
use pact_models::pact::Pact;
use pact_models::provider_states::ProviderState;
use pact_models::sync_interaction::RequestResponseInteraction;
use pact_models::sync_pact::RequestResponsePact;
use pact_models::{Consumer, PactSpecification, Provider};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

pub const SUITE_URL: &str = "https://github.com/pact-foundation/pact-compatibility-suite";

/// The features imported, and the spec version each is written at.
const FEATURES: &[(&str, &str, PactSpecification)] = &[
  ("V1/http_provider.feature", "v1", PactSpecification::V1),
  ("V2/http_provider.feature", "v2", PactSpecification::V2),
  ("V3/http_provider.feature", "v3", PactSpecification::V3),
  ("V4/http_provider.feature", "v4", PactSpecification::V4),
];

pub fn import(suite: &Path, out: &Path) -> Result<String, String> {
  if out.exists() {
    std::fs::remove_dir_all(out).map_err(|err| format!("{}: {err}", out.display()))?;
  }
  let mut imported = 0;
  let mut skipped = Vec::new();
  for (feature, dir, spec) in FEATURES {
    let path = suite.join("features").join(feature);
    let text = std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
    let parsed = parse_feature(&text);
    let interactions = parsed
      .background
      .iter()
      .find(|step| step.text == "the following HTTP interactions have been defined:")
      .map(|step| interactions_from(&step.table, suite))
      .transpose()?
      .unwrap_or_default();
    let mut mutated = BTreeSet::new();
    for (index, scenario) in parsed.scenarios.iter().enumerate() {
      let name = format!("{:02}-{}", index + 1, slug(&scenario.name));
      let source = json!({
        "suite": SUITE_URL,
        "feature": format!("features/{feature}"),
        "scenario": scenario.name,
      });
      match fixture_from(
        scenario,
        &interactions,
        *spec,
        &mut mutated,
        &suite.join("fixtures"),
      ) {
        Ok((mut fixture, pacts)) => {
          fixture.source = source;
          let dir = out.join(dir).join(&name);
          std::fs::create_dir_all(&dir).map_err(|err| format!("{}: {err}", dir.display()))?;
          for (file, pact) in pacts {
            write_json(&dir.join(file), &pact)?;
          }
          write_json(
            &dir.join(crate::fixture::FIXTURE_FILE),
            &serde_json::to_value(&fixture).unwrap_or_default(),
          )?;
          imported += 1;
        }
        Err(Skip(reason)) => skipped.push(json!({
          "feature": format!("features/{feature}"),
          "scenario": scenario.name,
          "reason": reason,
        })),
      }
    }
  }
  let record = json!({
    "$comment": "Written by `verifier-diff import-compatibility-suite`; regenerate rather than edit. Each skipped scenario names what will let it in.",
    "suite": SUITE_URL,
    "commit": commit_of(suite),
    "licence": "Apache-2.0",
    "imported": imported,
    "skipped": skipped,
  });
  write_json(&out.join("import.json"), &record)?;
  Ok(format!(
    "imported {imported} scenario(s), skipped {}",
    record["skipped"].as_array().map_or(0, Vec::len)
  ))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
  let mut text = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
  text.push('\n');
  std::fs::write(path, text).map_err(|err| format!("{}: {err}", path.display()))
}

fn commit_of(suite: &Path) -> Value {
  std::process::Command::new("git")
    .arg("-C")
    .arg(suite)
    .args(["rev-parse", "HEAD"])
    .output()
    .ok()
    .filter(|output| output.status.success())
    .map(|output| Value::String(String::from_utf8_lossy(&output.stdout).trim().to_string()))
    .unwrap_or(Value::Null)
}

/// A scenario this importer cannot express, and the plan item that will let it in.
struct Skip(String);

fn fixture_from(
  scenario: &Scenario,
  interactions: &[RequestResponseInteraction],
  spec: PactSpecification,
  mutated: &mut BTreeSet<usize>,
  fixtures: &Path,
) -> Result<(Fixture, Vec<(String, Value)>), Skip> {
  let interaction = |number: usize| {
    interactions.get(number.wrapping_sub(1)).cloned().ok_or_else(|| {
      Skip(format!(
        "the scenario names interaction {number}, which the table does not define"
      ))
    })
  };
  let mut routes = Vec::new();
  let mut pacts = Vec::new();
  let mut callback = StateCallback::None;
  let mut expected = None;
  let mut changed = false;
  let mut verified = Vec::new();
  // Pacts "verified from a Pact broker", by their position in `pacts`; and the scenario's other
  // broker steps.
  let mut from_broker = Vec::new();
  let mut publish = false;
  let mut published = Vec::new();
  // Every step from the first `Then` on is an assertion, whatever its keyword.
  let mut asserting = false;

  for step in &scenario.steps {
    let text = step.text.as_str();
    asserting |= step.keyword == "Then";
    if let Some(rest) = text.strip_prefix("a provider is started that returns the response from interaction ")
    {
      let (number, changes) = match rest.split_once(", with the following changes:") {
        Some((number, _)) => (number, true),
        None => (rest, false),
      };
      let mut served = interaction(parse_number(number)?)?;
      if changes {
        changed = true;
        apply_changes(&mut served, &step.table, fixtures).map_err(|err| Skip(format!("S1.1: {err}")))?;
      }
      routes.push(route_of(&served));
    } else if let Some(rest) =
      text.strip_prefix("a provider is started that returns the responses from interactions ")
    {
      for number in rest.trim_matches('"').split(',') {
        routes.push(route_of(&interaction(parse_number(number)?)?));
      }
    } else if let Some(rest) = text.strip_prefix("a Pact file for interaction ") {
      let (number, how) = rest
        .split_once(" is to be verified")
        .ok_or_else(|| unsupported(text))?;
      let number = parse_number(number)?;
      let mut pact_interaction = interaction(number)?;
      let mut pending = false;
      let mut comments = HashMap::new();
      match how {
        "" => {}
        " from a Pact broker" => from_broker.push(pacts.len()),
        ", but is marked pending" => pending = true,
        " with the following provider states defined:" => {
          pact_interaction.provider_states = states_from(&step.table)?;
        }
        " with the following comments:" => comments = comments_from(&step.table),
        other => match other
          .strip_prefix(" with a provider state \"")
          .and_then(|rest| rest.strip_suffix("\" defined"))
        {
          Some(state) => pact_interaction.provider_states.push(ProviderState {
            name: state.to_string(),
            params: HashMap::new(),
          }),
          None => return Err(unsupported(text)),
        },
      }
      verified.push(number);
      pacts.push((
        format!("pact-{}.json", pacts.len() + 1),
        pact_json(number, pact_interaction, spec, pending, comments)?,
      ));
    } else if text == "a provider state callback is configured" {
      callback = StateCallback::Ok;
    } else if text == "a provider state callback is configured, but will return a failure" {
      callback = StateCallback::Fail;
    } else if text == "publishing of verification results is enabled" {
      publish = true;
    } else if text == "a verification result will NOT be published back" {
      published.push((None, Published::Absent));
    } else if let Some((outcome, number)) = text
      .strip_prefix("a successful verification result will be published back for interaction ")
      .map(|number| (Published::Pass, number))
      .or_else(|| {
        text
          .strip_prefix("a failed verification result will be published back for the interaction ")
          .map(|number| (Published::Fail, number))
      })
    {
      // The suite writes the interaction as `{N}`.
      published.push((
        Some(parse_number(number.trim_matches(|c| c == '{' || c == '}'))?),
        outcome,
      ));
    } else if text.starts_with("a request filter is configured") {
      return Err(Skip("S1.9: request filters as hooks".to_string()));
    } else if text == "the verification is run" {
    } else if text == "the verification will be successful" {
      expected = Some(RunVerdict::Pass);
    } else if text == "the verification will NOT be successful" {
      expected = Some(RunVerdict::Fail);
    } else if asserting {
      // An assertion about pact-reference's own behaviour — a callback's calls, an error's wording, a
      // warning printed — which the differential comparison does not read.
    } else {
      return Err(unsupported(text));
    }
  }
  if routes.is_empty() || pacts.is_empty() {
    return Err(Skip(
      "the scenario starts no provider or verifies no pact".to_string(),
    ));
  }
  let broker = if from_broker.is_empty() {
    if publish || !published.is_empty() {
      return Err(Skip(
        "S1.3: publishing results for pacts read from files".to_string(),
      ));
    }
    None
  } else {
    if from_broker.len() != pacts.len() {
      return Err(Skip(
        "S1.3: pacts from a broker and from files in one run".to_string(),
      ));
    }
    let mut expected_published = std::collections::BTreeMap::new();
    for (number, outcome) in published {
      match number {
        // "will NOT be published back": nothing, for any pact.
        None => {
          for (file, _) in &pacts {
            expected_published.insert(file.clone(), outcome);
          }
        }
        Some(number) => {
          let position = verified
            .iter()
            .position(|verified| *verified == number)
            .ok_or_else(|| {
              Skip(format!(
                "S1.1: the scenario expects a result for interaction {number}, which it does not verify"
              ))
            })?;
          expected_published.insert(pacts[position].0.clone(), outcome);
        }
      }
    }
    Some(BrokerSpec {
      verify: BrokerVerify {
        publish,
        ..BrokerVerify::default()
      },
      expected_published,
      ..BrokerSpec::default()
    })
  };
  // Mutate an interaction's response once per feature: the first scenario that serves it unchanged,
  // with no states, and expects success. Every other scenario serving it would repeat the same cases.
  let mutations = expected == Some(RunVerdict::Pass)
    && !changed
    && callback == StateCallback::None
    && verified.len() == 1
    && broker.is_none()
    && mutated.insert(verified[0]);
  Ok((
    Fixture {
      description: scenario.name.clone(),
      source: Value::Null,
      pacts: pacts.iter().map(|(file, _)| file.clone()).collect(),
      provider: ProviderSpec::Scripted {
        state_callback: callback,
        routes,
      },
      expected,
      mutations,
      broker,
    },
    pacts,
  ))
}

fn unsupported(text: &str) -> Skip {
  Skip(format!("S1.1: a step the importer does not read yet: \"{text}\""))
}

fn parse_number(text: &str) -> Result<usize, Skip> {
  text
    .trim()
    .parse()
    .map_err(|_| Skip(format!("S1.1: '{text}' is not an interaction number")))
}

fn route_of(interaction: &RequestResponseInteraction) -> Route {
  let http = interaction
    .as_v4_http()
    .expect("a request/response interaction is HTTP");
  let mut route = Route::from_interaction(&http);
  // The suite's provider is a mock server holding one interaction with no states: it answers
  // whatever state was set up.
  route.states.clear();
  route
}

/// The pact the suite writes for "a Pact file for interaction N": one interaction, consumer `c_N`,
/// provider `p`, at the feature's spec version.
fn pact_json(
  number: usize,
  interaction: RequestResponseInteraction,
  spec: PactSpecification,
  pending: bool,
  comments: HashMap<String, Value>,
) -> Result<Value, Skip> {
  let pact = RequestResponsePact {
    consumer: Consumer {
      name: format!("c_{number}"),
    },
    provider: Provider {
      name: "p".to_string(),
    },
    interactions: vec![interaction],
    specification_version: spec,
    ..RequestResponsePact::default()
  };
  let mut document = if spec == PactSpecification::V4 {
    let mut v4 = pact.as_v4_pact().map_err(|err| Skip(format!("S1.1: {err}")))?;
    for interaction in v4.interactions.iter_mut() {
      if let Some(http) = interaction.as_v4_http_mut() {
        http.pending = pending;
        http.comments = comments.clone();
      }
    }
    v4.to_json(spec)
  } else {
    pact.to_json(spec)
  }
  .map_err(|err| Skip(format!("S1.1: {err}")))?;
  // The models' own version, which would change every fixture whenever pact_models moved.
  if let Some(metadata) = document.get_mut("metadata").and_then(Value::as_object_mut) {
    metadata.remove("pactRust");
  }
  Ok(document)
}

fn states_from(table: &[Vec<String>]) -> Result<Vec<ProviderState>, Skip> {
  rows(table)
    .into_iter()
    .map(|row| {
      let name = row.get("State Name").cloned().unwrap_or_default();
      let params = match row.get("Parameters") {
        Some(parameters) => serde_json::from_str::<Value>(parameters)
          .map_err(|err| Skip(format!("S1.1: provider state parameters: {err}")))?
          .as_object()
          .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
          .unwrap_or_default(),
        None => HashMap::new(),
      };
      Ok(ProviderState { name, params })
    })
    .collect()
}

fn comments_from(table: &[Vec<String>]) -> HashMap<String, Value> {
  let mut text = Vec::new();
  let mut comments = HashMap::new();
  for row in rows(table) {
    let comment = row.get("comment").cloned().unwrap_or_default();
    match row.get("type").map(String::as_str) {
      Some("testname") => {
        comments.insert("testname".to_string(), Value::String(comment));
      }
      _ => text.push(Value::String(comment)),
    }
  }
  if !text.is_empty() {
    comments.insert("text".to_string(), Value::Array(text));
  }
  comments
}

/// A table's rows after the header, as header -> cell.
fn rows(table: &[Vec<String>]) -> Vec<HashMap<String, String>> {
  let Some((header, body)) = table.split_first() else {
    return vec![];
  };
  body
    .iter()
    .map(|row| header.iter().cloned().zip(row.iter().cloned()).collect())
    .collect()
}

// --- the suite's table rules (pact-reference compatibility-suite/tests/shared_steps/mod.rs) ---------

fn interactions_from(table: &[Vec<String>], suite: &Path) -> Result<Vec<RequestResponseInteraction>, String> {
  let fixtures = suite.join("fixtures");
  let mut out = Vec::new();
  for (row_index, row) in rows(table).into_iter().enumerate() {
    let mut interaction = RequestResponseInteraction {
      description: format!("Interaction {row_index}"),
      id: Some(format!("ID{}", row_index + 1)),
      ..RequestResponseInteraction::default()
    };
    if let Some(method) = row.get("method") {
      interaction.request.method = method.clone();
    }
    if let Some(path) = row.get("path") {
      interaction.request.path = path.clone();
    }
    if let Some(query) = row.get("query") {
      interaction.request.query = pact_models::query_strings::parse_query_string(query);
    }
    if let Some(headers) = row.get("headers").filter(|headers| !headers.is_empty()) {
      interaction.request.headers = Some(parse_headers(headers));
    }
    if let Some(body) = row.get("body") {
      setup_body(body, &mut interaction.request, &fixtures)?;
    }
    if let Some(rules) = row.get("matching rules") {
      interaction.request.matching_rules = matching_rules(rules, &fixtures)?;
    }
    if let Some(status) = row.get("response") {
      interaction.response.status = status
        .parse()
        .map_err(|_| format!("response status '{status}'"))?;
    }
    if let Some(headers) = row.get("response headers").filter(|headers| !headers.is_empty()) {
      interaction.response.headers = Some(parse_headers(headers));
    }
    if let Some(body) = row.get("response body").filter(|body| !body.is_empty()) {
      setup_body(body, &mut interaction.response, &fixtures)?;
    }
    if let Some(rules) = row.get("response matching rules") {
      interaction.response.matching_rules = matching_rules(rules, &fixtures)?;
    }
    out.push(interaction);
  }
  Ok(out)
}

/// The changes a provider step makes to the interaction it serves.
fn apply_changes(
  interaction: &mut RequestResponseInteraction,
  table: &[Vec<String>],
  fixtures: &Path,
) -> Result<(), String> {
  for row in rows(table) {
    for (field, value) in row {
      match field.as_str() {
        "response" => {
          interaction.response.status = value.parse().map_err(|_| format!("response status '{value}'"))?;
        }
        "response headers" => {
          let headers = interaction.response.headers_mut();
          for (name, values) in parse_headers(&value) {
            headers.entry(name).or_default().extend(values);
          }
        }
        "response body" => setup_body(&value, &mut interaction.response, fixtures)?,
        _ => {}
      }
    }
  }
  Ok(())
}

fn parse_headers(text: &str) -> HashMap<String, Vec<String>> {
  text
    .split(',')
    .filter_map(|header| {
      let header = header.trim();
      let header = header.strip_prefix('\'').unwrap_or(header);
      let header = header.strip_suffix('\'').unwrap_or(header);
      let (name, value) = header.split_once(':')?;
      let name = name.trim();
      Some((
        name.to_string(),
        pact_models::headers::parse_header(name, value.trim()),
      ))
    })
    .collect()
}

fn matching_rules(rules: &str, fixtures: &Path) -> Result<pact_models::matchingrules::MatchingRules, String> {
  let json: Value = match rules.strip_prefix("JSON:") {
    Some(inline) => serde_json::from_str(inline).map_err(|err| format!("matching rules: {err}"))?,
    None => {
      let path = fixtures.join(rules.trim());
      let text = std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
      serde_json::from_str(&text).map_err(|err| format!("{}: {err}", path.display()))?
    }
  };
  pact_models::matchingrules::matchers_from_json(&json!({ "matchingRules": json }), &None)
    .map_err(|err| format!("matching rules: {err}"))
}

fn setup_body(body: &str, part: &mut dyn HttpPart, fixtures: &Path) -> Result<(), String> {
  if body.is_empty() {
    return Ok(());
  }
  let present = |bytes: Vec<u8>, content_type: ContentType| {
    OptionalBody::Present(Bytes::from(bytes), Some(content_type), None)
  };
  if let Some(json) = body.strip_prefix("JSON:") {
    part.add_header("content-type", vec!["application/json"]);
    *part.body_mut() = present(json.trim().as_bytes().to_vec(), JSON.clone());
  } else if let Some(xml) = body.strip_prefix("XML:") {
    part.add_header("content-type", vec!["application/xml"]);
    *part.body_mut() = present(xml.trim().as_bytes().to_vec(), XML.clone());
  } else if let Some(file) = body.strip_prefix("file:") {
    let file = file.trim();
    let path = fixtures.join(file);
    let bytes = std::fs::read(&path).map_err(|err| format!("{}: {err}", path.display()))?;
    if file.ends_with("-body.xml") {
      let (content_type, contents) = body_fixture(&String::from_utf8_lossy(&bytes));
      part.add_header("content-type", vec![content_type.as_str()]);
      let parsed = ContentType::parse(content_type.as_str()).unwrap_or_else(|_| TEXT.clone());
      *part.body_mut() = present(contents.into_bytes(), parsed);
    } else {
      let content_type = content_type_of(file, part);
      part.add_header("content-type", vec![content_type.to_string().as_str()]);
      *part.body_mut() = present(bytes, content_type);
    }
  } else {
    let text = if body == "EMPTY" { "" } else { body };
    let content_type = content_type_of(text, part);
    part.add_header("content-type", vec![content_type.to_string().as_str()]);
    *part.body_mut() = present(text.as_bytes().to_vec(), content_type);
  }
  Ok(())
}

fn content_type_of(name: &str, part: &dyn HttpPart) -> ContentType {
  if name.ends_with(".json") {
    JSON.clone()
  } else if name.ends_with(".xml") {
    XML.clone()
  } else if name.ends_with(".jpg") {
    ContentType::from("image/jpeg")
  } else if name.ends_with(".pdf") {
    ContentType::from("application/pdf")
  } else {
    part.content_type().unwrap_or_else(|| TEXT.clone())
  }
}

/// A `*-body.xml` fixture: `<body><contentType>…</contentType><contents eol="CRLF">…</contents></body>`,
/// as the content type and the contents, trimmed, with CRLF line endings when `eol` asks for them.
fn body_fixture(xml: &str) -> (String, String) {
  let content_type = element(xml, "contentType")
    .map(|(_, text)| text)
    .unwrap_or_else(|| "text/plain".to_string());
  let (crlf, contents) = match element(xml, "contents") {
    Some((attributes, text)) => (attributes.contains("eol=\"CRLF\""), text),
    None => (false, String::new()),
  };
  let contents = if crlf {
    contents.replace("\r\n", "\n").replace('\n', "\r\n")
  } else {
    contents
  };
  (content_type, contents)
}

/// An element's attributes and its text: CDATA unwrapped, entities decoded, trimmed.
fn element(xml: &str, name: &str) -> Option<(String, String)> {
  let open = xml.find(&format!("<{name}"))?;
  let after_name = open + name.len() + 1;
  let close_of_open = after_name + xml[after_name..].find('>')?;
  let attributes = xml[after_name..close_of_open].to_string();
  let end = close_of_open + xml[close_of_open..].find(&format!("</{name}>"))?;
  let inner = xml[close_of_open + 1..end].trim();
  let text = match inner
    .strip_prefix("<![CDATA[")
    .and_then(|rest| rest.strip_suffix("]]>"))
  {
    Some(cdata) => cdata.trim().to_string(),
    None => inner
      .replace("&lt;", "<")
      .replace("&gt;", ">")
      .replace("&quot;", "\"")
      .replace("&apos;", "'")
      .replace("&amp;", "&"),
  };
  Some((attributes, text))
}

fn slug(name: &str) -> String {
  let mut out = String::new();
  for c in name.chars() {
    if c.is_ascii_alphanumeric() {
      out.push(c.to_ascii_lowercase());
    } else if !out.ends_with('-') {
      out.push('-');
    }
  }
  out.trim_matches('-').to_string()
}

// --- Gherkin, the subset the suite uses -------------------------------------------------------------

#[derive(Debug, Default)]
struct Feature {
  background: Vec<Step>,
  scenarios: Vec<Scenario>,
}

#[derive(Debug)]
struct Scenario {
  name: String,
  steps: Vec<Step>,
}

#[derive(Debug)]
struct Step {
  keyword: String,
  text: String,
  table: Vec<Vec<String>>,
}

fn parse_feature(text: &str) -> Feature {
  let mut feature = Feature::default();
  let mut in_background = false;
  for line in text.lines().map(str::trim) {
    if line.is_empty() || line.starts_with('#') || line.starts_with('@') || line.starts_with("Feature:") {
      continue;
    }
    if line.starts_with("Background:") {
      in_background = true;
    } else if let Some(name) = line.strip_prefix("Scenario:") {
      in_background = false;
      feature.scenarios.push(Scenario {
        name: name.trim().to_string(),
        steps: vec![],
      });
    } else if line.starts_with('|') {
      let cells = line
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect();
      let steps = if in_background {
        &mut feature.background
      } else {
        match feature.scenarios.last_mut() {
          Some(scenario) => &mut scenario.steps,
          None => continue,
        }
      };
      if let Some(step) = steps.last_mut() {
        step.table.push(cells);
      }
    } else if let Some((keyword, rest)) = line.split_once(' ')
      && ["Given", "When", "Then", "And", "But"].contains(&keyword)
    {
      let step = Step {
        keyword: keyword.to_string(),
        text: rest.trim().to_string(),
        table: vec![],
      };
      if in_background {
        feature.background.push(step);
      } else if let Some(scenario) = feature.scenarios.last_mut() {
        scenario.steps.push(step);
      }
    }
  }
  feature
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  const FEATURE: &str = r#"
@provider
Feature: Basic HTTP provider

  Background:
    Given the following HTTP interactions have been defined:
      | No | method | path   | response | response body             |
      | 1  | GET    | /basic | 200      | JSON: { "one": "a" }      |

  Scenario: Verifying a simple HTTP request
    Given a provider is started that returns the response from interaction 1
    And a Pact file for interaction 1 is to be verified
    When the verification is run
    Then the verification will be successful

  Scenario: Verifies the response status code
    Given a provider is started that returns the response from interaction 1, with the following changes:
      | response |
      | 400      |
    And a Pact file for interaction 1 is to be verified
    When the verification is run
    Then the verification will NOT be successful
    And the verification results will contain a "Response status did not match" error

  Scenario: Via a broker
    Given a provider is started that returns the response from interaction 1
    And a Pact file for interaction 1 is to be verified from a Pact broker
"#;

  const BROKER_FEATURE: &str = r#"
Feature: Broker scenarios

  Background:
    Given the following HTTP interactions have been defined:
      | No | method | path   | response | response body        |
      | 1  | GET    | /basic | 200      | JSON: { "one": "a" } |
      | 2  | GET    | /other | 200      | JSON: { "two": "b" } |

  Scenario: Publishing what each pact got
    Given a provider is started that returns the response from interaction 1
    And a Pact file for interaction 1 is to be verified from a Pact broker
    And a Pact file for interaction 2 is to be verified from a Pact broker
    And publishing of verification results is enabled
    When the verification is run
    Then the verification will NOT be successful
    And a successful verification result will be published back for interaction {1}
    And a failed verification result will be published back for the interaction {2}

  Scenario: Nothing published
    Given a provider is started that returns the response from interaction 1
    And a Pact file for interaction 1 is to be verified from a Pact broker
    When the verification is run
    Then the verification will be successful
    And a verification result will NOT be published back

  Scenario: From a broker and from a file
    Given a provider is started that returns the response from interaction 1
    And a Pact file for interaction 1 is to be verified from a Pact broker
    And a Pact file for interaction 2 is to be verified
    When the verification is run
"#;

  #[test]
  fn a_broker_scenario_says_what_each_pact_publishes() {
    let feature = parse_feature(BROKER_FEATURE);
    let interactions = interactions_from(&feature.background[0].table, Path::new("unused")).unwrap();
    let fixture = |index: usize| {
      fixture_from(
        &feature.scenarios[index],
        &interactions,
        PactSpecification::V1,
        &mut BTreeSet::new(),
        Path::new("unused"),
      )
    };

    let (both, _) = fixture(0).ok().unwrap();
    let broker = both.broker.unwrap();
    assert!(broker.verify.publish);
    assert_eq!(
      broker.expected_published,
      std::collections::BTreeMap::from([
        ("pact-1.json".to_string(), Published::Pass),
        ("pact-2.json".to_string(), Published::Fail),
      ])
    );

    let (nothing, _) = fixture(1).ok().unwrap();
    let broker = nothing.broker.unwrap();
    assert!(!broker.verify.publish);
    assert_eq!(
      broker.expected_published.get("pact-1.json"),
      Some(&Published::Absent)
    );

    let Err(Skip(reason)) = fixture(2) else {
      panic!("a run from a broker and a file is skipped");
    };
    assert!(reason.starts_with("S1.3"), "{reason}");
  }

  #[test]
  fn a_feature_is_read_into_steps_and_tables() {
    let feature = parse_feature(FEATURE);
    assert_eq!(feature.background[0].table.len(), 2);
    assert_eq!(feature.scenarios.len(), 3);
    assert_eq!(
      feature.scenarios[1].steps[0].table,
      vec![vec!["response".to_string()], vec!["400".to_string()]]
    );
  }

  #[test]
  fn a_scenario_becomes_a_fixture_or_says_why_not() {
    let feature = parse_feature(FEATURE);
    let interactions = interactions_from(&feature.background[0].table, Path::new("unused")).unwrap();
    let mut mutated = BTreeSet::new();

    let (simple, pacts) = fixture_from(
      &feature.scenarios[0],
      &interactions,
      PactSpecification::V1,
      &mut mutated,
      Path::new("unused"),
    )
    .ok()
    .unwrap();
    assert_eq!(simple.expected, Some(RunVerdict::Pass));
    assert!(simple.mutations, "the first unchanged success is mutated");
    assert_eq!(pacts[0].1["consumer"]["name"], json!("c_1"));
    assert_eq!(
      pacts[0].1["interactions"][0]["response"]["body"],
      json!({ "one": "a" })
    );
    assert!(pacts[0].1["metadata"].get("pactRust").is_none());

    let (changed, _) = fixture_from(
      &feature.scenarios[1],
      &interactions,
      PactSpecification::V1,
      &mut mutated,
      Path::new("unused"),
    )
    .ok()
    .unwrap();
    assert_eq!(changed.expected, Some(RunVerdict::Fail));
    let ProviderSpec::Scripted { routes, .. } = &changed.provider else {
      panic!()
    };
    assert_eq!(routes[0].response.status, 400);

    let (via_broker, _) = fixture_from(
      &feature.scenarios[2],
      &interactions,
      PactSpecification::V1,
      &mut mutated,
      Path::new("unused"),
    )
    .ok()
    .unwrap();
    let broker = via_broker.broker.expect("a broker scenario is a broker fixture");
    assert!(!broker.verify.publish);
    assert!(!via_broker.mutations, "a broker fixture is not mutated");
  }

  #[test]
  fn a_body_fixture_gives_its_content_type_and_contents() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<body>
    <contentType>application/x-www-form-urlencoded</contentType>
    <contents><![CDATA[a=1&b=2]]></contents>
</body>"#;
    assert_eq!(
      body_fixture(xml),
      (
        "application/x-www-form-urlencoded".to_string(),
        "a=1&b=2".to_string()
      )
    );
    let crlf =
      "<body><contentType>multipart/form-data</contentType><contents eol=\"CRLF\">a\nb</contents></body>";
    assert_eq!(body_fixture(crlf).1, "a\r\nb");
  }

  #[test]
  fn a_scenario_name_is_a_directory_name() {
    assert_eq!(
      slug("Response with XML body (negative case)"),
      "response-with-xml-body-negative-case"
    );
  }
}
