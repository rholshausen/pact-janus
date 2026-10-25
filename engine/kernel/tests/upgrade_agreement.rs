//! Plan-grammar spec §4.4's two-path agreement, over the golden corpus (staged plan S1.25): every
//! v1–v4 HTTP case under `corpora/legacy/` is compiled the two ways a pact can reach a plan —
//! directly (design 3.5), and upgraded to a contract (design 2.5) whose shapes are compiled — and
//! both plans run against the case's values. They must reach the same verdict. Their mismatches may
//! say different things, so only the verdict is compared; a case's `also-compiled-from` is where
//! the messages are pinned too.
//!
//! A case may disagree only where the conversion says it loses something, which `KNOWN` lists with
//! the finding that reports it.

use pact_janus_kernel::interaction_spec;
use pact_janus_kernel::legacy::compile::compile_interaction;
use pact_janus_kernel::legacy::pact as legacy_pact;
use pact_janus_kernel::legacy::upgrade;
use pact_janus_kernel::plan::{self, CapturedValues, RuntimeValue, Status};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Cases whose two verdicts differ, each with the finding the upgrade reports for it.
const KNOWN: &[(&str, &str)] = &[
  // A request body's unnamed member: v1–v4 close it, shapes cannot (ADR 0007).
  ("v4-closed-request-body", "request-body-opened"),
  // A header with no rule became `equality`, which a parameter the reply adds fails, where v1–v4
  // compare MIME parameters as a set (`legacy:header-value`).
  ("v3-xml-body", "rule-narrowed"),
];

fn corpus() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpora/legacy")
}

/// The case's values as each path's verifier reads a reply. A legacy plan reads one string per header,
/// `$.<part>.headers.<name>`, as the case gives them; a shape reads the `{name: [value]}` slot the HTTP
/// transport produces, which it always produces, with no header in it when there is none.
fn captured(values: &Map<String, Value>, encoded: &Map<String, Value>, legacy: bool) -> CapturedValues {
  use base64::Engine as _;
  let mut headers: BTreeMap<String, Map<String, Value>> = BTreeMap::from([
    ("request".to_string(), Map::new()),
    ("response".to_string(), Map::new()),
  ]);
  let mut resolver = CapturedValues::new();
  for (path, value) in values {
    let mut parts = path.splitn(4, '.');
    if let (Some("$"), Some(part), Some("headers"), Some(name)) =
      (parts.next(), parts.next(), parts.next(), parts.next())
      && !legacy
    {
      headers
        .entry(part.to_string())
        .or_default()
        .insert(name.to_string(), json!([value]));
      continue;
    }
    let value = match encoded.get(path).and_then(Value::as_str) {
      Some("base64") => RuntimeValue::Bytes(
        base64::engine::general_purpose::STANDARD
          .decode(value.as_str().expect("a base64 value is a string"))
          .expect("base64"),
      ),
      _ => RuntimeValue::from_json(value),
    };
    resolver = resolver.capture(path.clone(), value);
  }
  if !legacy {
    for (part, members) in headers {
      if !values.contains_key(&format!("$.{part}.headers")) {
        resolver = resolver.capture(
          format!("$.{part}.headers"),
          RuntimeValue::from_json(&Value::Object(members)),
        );
      }
    }
  }
  resolver
}

fn status(plan: &plan::Plan, values: &CapturedValues) -> (Status, Vec<String>) {
  let executed = plan::execute(plan, values);
  let (status, mismatches) = plan::outcome(&executed);
  (
    status,
    mismatches
      .iter()
      .map(|m| format!("{}: {}", m.path.clone().unwrap_or_default(), m.message))
      .collect(),
  )
}

#[test]
fn every_legacy_case_reaches_the_same_verdict_upgraded() {
  let mut checked = 0;
  let mut disagreements = Vec::new();
  let mut dirs: Vec<PathBuf> = std::fs::read_dir(corpus())
    .expect("corpora/legacy")
    .flatten()
    .map(|entry| entry.path())
    .filter(|path| path.join("case.json").exists())
    .collect();
  dirs.sort();
  for dir in dirs {
    let name = dir.file_name().unwrap().to_string_lossy().to_string();
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let input = &case["input"];
    if input["kind"] != "pact-interaction" {
      continue;
    }
    let pact = &input["pact"];
    let index = input["index"].as_u64().unwrap_or(0) as usize;
    let read = legacy_pact::read("corpus case", pact).expect("a readable pact");
    let interactions = legacy_pact::interactions(read.as_ref());
    let legacy_pact::PactInteraction::Http(interaction) = &interactions[index] else {
      continue; // a message has no contract form yet
    };
    let legacy = compile_interaction(
      &interaction.description,
      &legacy_pact::legacy_request(&interaction.request).unwrap(),
      &legacy_pact::legacy_response(&interaction.response).unwrap(),
    );
    let upgraded = upgrade::pact("corpus case", pact).expect("the pact upgrades");
    let http_index = interactions[..index]
      .iter()
      .filter(|i| matches!(i, legacy_pact::PactInteraction::Http(_)))
      .count();
    let contract_interaction = &upgraded.contract.interactions[http_index];
    let document = serde_json::to_value(contract_interaction).unwrap();
    let spec = interaction_spec::parse(&document).unwrap_or_else(|err| panic!("{name}: {:?}", err.problems));
    let shaped = plan::compile(&spec, &Default::default(), None);

    let values = case["values"].as_object().unwrap();
    let no_tags = Map::new();
    let encoded = case.get("encoded").and_then(Value::as_object).unwrap_or(&no_tags);
    let (legacy_status, legacy_mismatches) = status(&legacy, &captured(values, encoded, true));
    let (shaped_status, shaped_mismatches) = status(&shaped, &captured(values, encoded, false));
    checked += 1;
    let known = KNOWN.iter().find(|(case, _)| *case == name);
    match (legacy_status == shaped_status, known) {
      (true, None) => {}
      (true, Some(_)) => disagreements.push(format!("{name}: listed in KNOWN, and the two paths agree")),
      (false, Some((_, code))) => assert!(
        upgraded.findings.iter().any(|f| f.code == *code),
        "{name}: the paths disagree, and the upgrade has no '{code}' finding"
      ),
      (false, None) => disagreements.push(format!(
        "{name}: legacy {legacy_status:?} {legacy_mismatches:?}, upgraded {shaped_status:?} {shaped_mismatches:?}"
      )),
    }
  }
  assert!(checked > 20, "only {checked} cases were checked");
  assert!(disagreements.is_empty(), "{}", disagreements.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// Each body kind, against replies that match it and replies that do not
// ---------------------------------------------------------------------------------------------

/// Both verdicts on `values` for the pact's first interaction, which must agree with `expected`.
fn check(name: &str, pact: &Value, values: Value, expected: Status) {
  let read = legacy_pact::read("test", pact).expect("a readable pact");
  let legacy_pact::PactInteraction::Http(interaction) = &legacy_pact::interactions(read.as_ref())[0] else {
    panic!("{name}: not HTTP");
  };
  let legacy = compile_interaction(
    &interaction.description,
    &legacy_pact::legacy_request(&interaction.request).unwrap(),
    &legacy_pact::legacy_response(&interaction.response).unwrap(),
  );
  let upgraded = upgrade::pact("test", pact).expect("the pact upgrades");
  let document = serde_json::to_value(&upgraded.contract.interactions[0]).unwrap();
  let spec = interaction_spec::parse(&document).unwrap_or_else(|err| panic!("{name}: {:?}", err.problems));
  let shaped = plan::compile(&spec, &Default::default(), None);
  let mut values = values.as_object().unwrap().clone();
  let mut encoded = Map::new();
  if let Some(Value::String(bytes)) = values.remove("bytes") {
    values.insert("$.response.body".to_string(), json!(bytes));
    encoded.insert("$.response.body".to_string(), json!("base64"));
  }
  let legacy_verdict = status(&legacy, &captured(&values, &encoded, true));
  let shaped_verdict = status(&shaped, &captured(&values, &encoded, false));
  assert_eq!(
    legacy_verdict.0, expected,
    "{name}: the pact's verdict {legacy_verdict:?}"
  );
  assert_eq!(
    shaped_verdict.0, expected,
    "{name}: the contract's verdict {shaped_verdict:?}"
  );
}

fn v3(request: Value, response: Value) -> Value {
  json!({
    "consumer": { "name": "c" }, "provider": { "name": "p" },
    "interactions": [{ "description": "i", "request": request, "response": response }],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  })
}

fn reply(headers: Value, body: Value) -> Value {
  let mut values = json!({
    "$.request.method": "GET", "$.request.path": "/", "$.request.query": {},
    "$.response.status": 200,
  });
  for (name, value) in headers.as_object().unwrap() {
    values[format!("$.response.headers.{name}")] = value.clone();
  }
  if !body.is_null() {
    values["$.response.body"] = body;
  }
  values
}

const MATCHED: Status = Status::Matched;
const MISMATCHED: Status = Status::Mismatched;

#[test]
fn a_text_body_reaches_the_pacts_verdict_upgraded() {
  let pact = v3(
    json!({ "method": "GET", "path": "/" }),
    json!({ "status": 200, "headers": { "Content-Type": "text/plain" }, "body": "Hello World!",
            "matchingRules": { "body": { "$": { "matchers": [{ "match": "regex", "regex": "Hello .+!" }] } } } }),
  );
  check(
    "same",
    &pact,
    reply(json!({ "content-type": "text/plain" }), json!("Hello World!")),
    MATCHED,
  );
  check(
    "by the rule",
    &pact,
    reply(json!({ "content-type": "text/plain" }), json!("Hello Ada!")),
    MATCHED,
  );
  check(
    "not by the rule",
    &pact,
    reply(json!({ "content-type": "text/plain" }), json!("Bye Ada!")),
    MISMATCHED,
  );
  check(
    "another type",
    &pact,
    reply(json!({ "content-type": "text/html" }), json!("Hello Ada!")),
    MISMATCHED,
  );
}

/// A pact that names no `Content-Type` header still checks the reply's (`legacy:body-type`), and the
/// contract checks it on the header.
#[test]
fn a_body_with_no_content_type_header_is_still_checked_for_its_type() {
  let pact = v3(
    json!({ "method": "GET", "path": "/" }),
    json!({ "status": 200, "body": "Hello" }),
  );
  for (label, expected) in [
    (Some("text/plain"), MATCHED),
    (Some("TEXT/Plain; charset=utf-8"), MATCHED),
    (Some(" text/plain ;charset=utf-8"), MATCHED),
    (Some("text/vnd.x+plain"), MATCHED),
    (None, MATCHED),
    (Some("text/html"), MISMATCHED),
    (Some("application/plain"), MISMATCHED),
    (Some("text/plainer"), MISMATCHED),
  ] {
    let headers = label
      .map(|label| json!({ "content-type": label }))
      .unwrap_or(json!({}));
    check(
      &format!("{label:?}"),
      &pact,
      reply(headers, json!("Hello")),
      expected,
    );
  }
}

#[test]
fn a_type_with_parameters_needs_the_same_ones() {
  let pact = json!({
    "consumer": { "name": "c" }, "provider": { "name": "p" },
    "interactions": [{ "type": "Synchronous/HTTP", "description": "i",
      "request": { "method": "GET", "path": "/" },
      "response": { "status": 200,
        "body": { "content": "Hello", "contentType": "text/plain; charset=UTF-8; format=flowed", "encoded": false } } }],
    "metadata": { "pactSpecification": { "version": "4.0" } }
  });
  // `pact_models` writes the pact's charset in lower case, and a parameter's value is compared as it
  // stands, as today (ADR 0043's context).
  for (label, expected) in [
    ("text/plain; charset=utf-8; format=flowed", MATCHED),
    ("text/plain;format=\"flowed\"; CHARSET=utf-8", MATCHED),
    ("text/plain; charset=utf-8", MISMATCHED),
    ("text/plain; charset=UTF-8; format=flowed", MISMATCHED),
    ("text/plain; charset=utf-8; format=flowed; delsp=yes", MISMATCHED),
  ] {
    check(
      label,
      &pact,
      reply(json!({ "content-type": label }), json!("Hello")),
      expected,
    );
  }
}

#[test]
fn a_binary_body_reaches_the_pacts_verdict_upgraded() {
  let pact = json!({
    "consumer": { "name": "c" }, "provider": { "name": "p" },
    "interactions": [{ "type": "Synchronous/HTTP", "description": "i",
      "request": { "method": "GET", "path": "/" },
      "response": { "status": 200,
        "body": { "content": "AAEC/w==", "contentType": "application/octet-stream", "encoded": "base64" } } }],
    "metadata": { "pactSpecification": { "version": "4.0" } }
  });
  let with = |label: &str, bytes: &str| {
    let mut values = reply(json!({ "content-type": label }), Value::Null);
    values["bytes"] = json!(bytes);
    values
  };
  check(
    "same",
    &pact,
    with("application/octet-stream", "AAEC/w=="),
    MATCHED,
  );
  check(
    "other octets",
    &pact,
    with("application/octet-stream", "AAEC"),
    MISMATCHED,
  );
  check("another type", &pact, with("image/png", "AAEC/w=="), MISMATCHED);
}

#[test]
fn a_form_body_reaches_the_pacts_verdict_upgraded() {
  let pact = v3(
    json!({ "method": "GET", "path": "/" }),
    json!({ "status": 200, "headers": { "Content-Type": "application/x-www-form-urlencoded" },
            "body": "n=17&ok=true&tag=a&tag=b",
            "matchingRules": { "body": {
              "$.n": { "matchers": [{ "match": "integer" }] },
              "$.ok": { "matchers": [{ "match": "boolean" }] },
              "$.tag": { "matchers": [{ "match": "regex", "regex": "[a-z]" }] } } } }),
  );
  let form = |n: Value, ok: Value, tag: Value| {
    reply(
      json!({ "content-type": "application/x-www-form-urlencoded" }),
      json!({ "n": n, "ok": ok, "tag": tag }),
    )
  };
  check(
    "same",
    &pact,
    form(json!(["17"]), json!(["true"]), json!(["a", "b"])),
    MATCHED,
  );
  check(
    "by the rules",
    &pact,
    form(json!(["+903"]), json!(["false"]), json!(["x", "y"])),
    MATCHED,
  );
  check(
    "not an integer",
    &pact,
    form(json!(["-1"]), json!(["true"]), json!(["a", "b"])),
    MISMATCHED,
  );
  check(
    "not a boolean",
    &pact,
    form(json!(["17"]), json!(["yes"]), json!(["a", "b"])),
    MISMATCHED,
  );
  check(
    "one value too few",
    &pact,
    form(json!(["17"]), json!(["true"]), json!(["a"])),
    MISMATCHED,
  );
  check(
    "one value too many",
    &pact,
    form(json!(["17", "18"]), json!(["true"]), json!(["a", "b"])),
    MISMATCHED,
  );
}

#[test]
fn a_number_rule_on_a_query_parameter_reads_the_text_upgraded() {
  let pact = v3(
    json!({ "method": "GET", "path": "/", "query": { "page": ["2"], "size": ["1.5"] },
            "matchingRules": { "query": {
              "page": { "matchers": [{ "match": "integer" }] },
              "size": { "matchers": [{ "match": "decimal" }] } } } }),
    json!({ "status": 200 }),
  );
  let query = |page: &str, size: &str| {
    json!({ "$.request.method": "GET", "$.request.path": "/",
            "$.request.query": { "page": [page], "size": [size] }, "$.response.status": 200 })
  };
  for (page, size, expected) in [
    ("3", "2", MATCHED),
    ("0012", "-1e3", MATCHED),
    ("3", "inf", MATCHED),
    ("3", ".5", MATCHED),
    ("a", "2", MISMATCHED),
    ("3.0", "2", MISMATCHED),
    ("3", "1,5", MISMATCHED),
    ("3", "e5", MISMATCHED),
  ] {
    check(&format!("{page} {size}"), &pact, query(page, size), expected);
  }
}

#[test]
fn an_xml_body_reaches_the_pacts_verdict_upgraded() {
  let pact = v3(
    json!({ "method": "GET", "path": "/" }),
    json!({ "status": 200, "headers": { "Content-Type": "application/xml" },
            "body": "<?xml version=\"1.0\"?><o:order xmlns:o=\"urn:o\" xmlns:x=\"urn:x\" id=\"7\" x:rev=\"2\"><total>9.5</total><note/><item>a</item><item>b</item></o:order>",
            "matchingRules": { "body": {
              "$.urn:o:order['@id']": { "matchers": [{ "match": "integer" }] },
              "$.urn:o:order.total['#text']": { "matchers": [{ "match": "decimal" }] },
              "$.urn:o:order.item": { "matchers": [{ "match": "type" }] } } } }),
  );
  let order = |id: &str, rev: &str, total: Value, note: Value, items: Value| {
    let mut note_element = json!({});
    if let Value::String(text) = note {
      note_element["#text"] = json!(text);
    }
    reply(
      json!({ "content-type": "application/xml" }),
      json!({ "order": {
        "#namespace": "urn:o", "@id": id, "@{urn:x}rev": rev,
        "total": [{ "#text": total }], "note": [note_element],
        "item": items,
      } }),
    )
  };
  let items = json!([{ "#text": "c" }, { "#text": "d" }, { "#text": "e" }]);
  check(
    "same",
    &pact,
    order(
      "7",
      "2",
      json!("9.5"),
      Value::Null,
      json!([{ "#text": "a" }, { "#text": "b" }]),
    ),
    MATCHED,
  );
  check(
    "by the rules",
    &pact,
    order("8", "2", json!("1e2"), Value::Null, items.clone()),
    MATCHED,
  );
  check(
    "not an integer id",
    &pact,
    order("x", "2", json!("9.5"), Value::Null, items.clone()),
    MISMATCHED,
  );
  check(
    "another namespaced attribute",
    &pact,
    order("7", "3", json!("9.5"), Value::Null, items.clone()),
    MISMATCHED,
  );
  check(
    "no total text",
    &pact,
    order("7", "2", Value::Null, Value::Null, items.clone()),
    MISMATCHED,
  );
  check(
    "text where the pact has none",
    &pact,
    order("7", "2", json!("9.5"), json!("x"), items.clone()),
    MISMATCHED,
  );
  check(
    "no items",
    &pact,
    order("7", "2", json!("9.5"), Value::Null, json!([])),
    MISMATCHED,
  );
  let mut wrong_namespace = order("7", "2", json!("9.5"), Value::Null, items);
  wrong_namespace["$.response.body"]["order"]["#namespace"] = json!("urn:other");
  check("another namespace", &pact, wrong_namespace, MISMATCHED);
}

#[test]
fn a_multipart_body_reaches_the_pacts_verdict_upgraded() {
  let case: Value =
    serde_json::from_str(&std::fs::read_to_string(corpus().join("v3-multipart-body/case.json")).unwrap())
      .unwrap();
  let pact = &case["input"]["pact"];
  let values = case["values"].clone();
  check("the corpus case", pact, values.clone(), MATCHED);
  let mutate = |name: &str, change: &dyn Fn(&mut Value)| {
    let mut values = values.clone();
    change(&mut values["$.response.body"]);
    check(name, pact, values, MISMATCHED);
  };
  mutate("a field not by its rule", &|body| {
    body["name"][0]["body"] = json!("ada")
  });
  mutate("a missing part", &|body| {
    body.as_object_mut().unwrap().remove("order");
  });
  mutate("a file where a field is", &|body| {
    body["name"][0]["filename"] = json!("n.txt")
  });
  mutate("a field where a file is", &|body| {
    body["order"][0].as_object_mut().unwrap().remove("filename");
  });
  mutate("a file's member not by its rule", &|body| {
    body["order"][0]["body"]["id"] = json!("42")
  });
  mutate("a file of another type", &|body| {
    body["order"][0]["headers"]["content-type"] = json!("text/plain")
  });
  mutate("an XML file's attribute not by its rule", &|body| {
    body["note"][0]["body"]["note"]["@lang"] = json!("eng")
  });
  let mut no_type = values.clone();
  no_type["$.response.body"]["order"][0]["headers"]
    .as_object_mut()
    .unwrap()
    .remove("content-type");
  check("a file part with no type", pact, no_type, MATCHED);
}
