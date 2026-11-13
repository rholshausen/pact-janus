//! `janus derive` (staged plan S4.2, ADR 0088), checked with the commands that read what it writes.
//!
//! The exit: the sample provider's OpenAPI document, spike 7.3's hand-authored one, derives a shape
//! that `janus check` matches to the sample provider's pact by selector and that finds everything
//! the shape its own tests recorded finds; and the generator-emitted document's lossy mappings are
//! each reported. What the command prints is pinned by the trycmd cases `tests/cmd/derive*.toml`.

use pretty_assertions::assert_eq;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn janus(args: &[&str]) -> (i32, Value) {
  let output = Command::new(env!("CARGO_BIN_EXE_janus"))
    .args(args)
    .current_dir(root())
    .env("NO_COLOR", "1")
    .env("RUST_LOG", "off")
    .output()
    .expect("janus runs");
  let stdout = String::from_utf8_lossy(&output.stdout);
  let document = serde_json::from_str(&stdout).unwrap_or_else(|err| {
    panic!(
      "janus {args:?} printed no JSON ({err}):\n{stdout}\n{}",
      String::from_utf8_lossy(&output.stderr)
    )
  });
  (output.status.code().unwrap_or(-1), document)
}

/// The shape and the losses `janus derive --output json` answers for the document at `path`.
fn derive(path: &str, extra: &[&str]) -> (Value, Vec<Value>) {
  let mut args = vec!["derive", path, "--output", "json"];
  args.extend_from_slice(extra);
  let (code, answer) = janus(&args);
  assert_eq!(code, 0, "{answer:#}");
  let losses = answer["losses"].as_array().cloned().unwrap_or_default();
  (answer["provider-shape"].clone(), losses)
}

fn codes(losses: &[Value]) -> BTreeSet<String> {
  losses
    .iter()
    .filter_map(|loss| loss["code"].as_str().map(str::to_string))
    .collect()
}

fn scratch(name: &str) -> PathBuf {
  let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("derive");
  std::fs::create_dir_all(&dir).unwrap();
  dir.join(name)
}

/// `janus check --output json` of the sample provider's pact against a provider shape.
fn check_the_pact(shape: &Value, name: &str) -> Value {
  let path = scratch(name);
  std::fs::write(&path, serde_json::to_vec_pretty(shape).unwrap()).unwrap();
  let (_, report) = janus(&[
    "check",
    "samples/order-service/pacts/web-app-order-service.json",
    "--provider-shape",
    path.to_str().unwrap(),
    "--output",
    "json",
  ]);
  report["pairs"][0].clone()
}

/// Each finding's interaction, path and kind: what two shapes agree on when they find the same thing.
fn findings(pair: &Value) -> BTreeSet<(String, String, String)> {
  pair["findings"]
    .as_array()
    .into_iter()
    .flatten()
    .map(|found| {
      (
        found["interaction"]["description"]
          .as_str()
          .unwrap_or_default()
          .to_string(),
        found["finding"]["path"].as_str().unwrap_or_default().to_string(),
        found["finding"]["kind"].as_str().unwrap_or_default().to_string(),
      )
    })
    .collect()
}

#[test]
fn the_sample_providers_openapi_document_finds_what_its_recorded_shape_finds() {
  let (derived, losses) = derive("samples/order-service/openapi.yaml", &[]);
  assert_eq!(derived["provenance"], "derived");
  assert_eq!(derived["provider"]["name"], "order-service");
  assert_eq!(
    codes(&losses),
    BTreeSet::from(["cardinality-unstated".to_string()]),
    "the one thing the hand-authored document leaves unsaid is whether `items` may be empty"
  );

  let recorded: Value = serde_json::from_slice(
    &std::fs::read(root().join("samples/order-service/shapes/order-service.provider-shape.json")).unwrap(),
  )
  .unwrap();
  let by_recorded = check_the_pact(&recorded, "recorded.json");
  let by_derived = check_the_pact(&derived, "derived.json");

  // Both of the pact's interactions are matched by operation: the order by the 200 entry, the
  // missing order by the 404 one.
  assert_eq!(by_derived["subsumption"]["matched"], 2, "{by_derived:#}");
  assert_eq!(by_derived["subsumption"]["not-published"], 0, "{by_derived:#}");

  let recorded_findings = findings(&by_recorded);
  let derived_findings = findings(&by_derived);
  assert!(!recorded_findings.is_empty());
  let missed: Vec<_> = recorded_findings.difference(&derived_findings).collect();
  assert!(
    missed.is_empty(),
    "the derived shape misses what the recorded one finds: {missed:?}"
  );
  // What it finds beyond them is what a document says and a test run did not see: any `id`, where
  // the recorder only ever saw order 66. And a review: the document's `date-time` against the pact's
  // regex, two conservative operators the walk cannot compare (subsumption-check spec §4.3), where
  // the recorder saw one timestamp, which the regex's own matcher decides.
  let beyond: BTreeSet<_> = derived_findings
    .difference(&recorded_findings)
    .map(|(_, path, kind)| (path.as_str(), kind.as_str()))
    .collect();
  assert_eq!(
    beyond,
    BTreeSet::from([
      ("response.body.id", "wider-values"),
      ("response.body.shippedAt", "unreviewable"),
    ])
  );
}

#[test]
fn assuming_lists_are_never_empty_says_so_and_finds_no_empty_list() {
  let (derived, losses) = derive("samples/order-service/openapi.yaml", &["--assume-non-empty"]);
  assert_eq!(
    codes(&losses),
    BTreeSet::from(["cardinality-assumed".to_string()])
  );
  let found = findings(&check_the_pact(&derived, "assumed.json"));
  assert!(
    !found.iter().any(|(_, path, _)| path == "response.body.items"),
    "{found:?}"
  );
}

#[test]
fn a_generated_documents_lossy_mappings_are_each_reported() {
  let (_, losses) = derive(
    "cli/tests/cmd/fixtures/openapi/order-service.orm-generated.openapi.json",
    &[],
  );
  let reported: BTreeSet<(String, String)> = losses
    .iter()
    .map(|loss| {
      (
        loss["code"].as_str().unwrap().to_string(),
        loss["pointer"].as_str().unwrap().to_string(),
      )
    })
    .collect();
  assert_eq!(
    reported,
    BTreeSet::from([
      (
        "cardinality-unstated".to_string(),
        "/components/schemas/Order/properties/items".to_string()
      ),
      (
        "unconstrained-object".to_string(),
        "/components/schemas/Order/properties/metadata".to_string()
      ),
    ])
  );
}

#[test]
fn every_derived_shape_is_one_the_engine_reads() {
  for document in [
    "samples/order-service/openapi.yaml",
    "cli/tests/cmd/fixtures/openapi/order-service.orm-generated.openapi.json",
    "cli/tests/cmd/fixtures/openapi/mapping.openapi.yaml",
  ] {
    let (shape, _) = derive(document, &[]);
    let bytes = serde_json::to_vec(&shape).unwrap();
    pact_janus_kernel::subsumption::read_provider_shape(&bytes)
      .unwrap_or_else(|err| panic!("{document}: {err:?}"));
    for (index, entry) in shape["interactions"].as_array().unwrap().iter().enumerate() {
      for member in ["selector", "parts"] {
        for (part, slots) in entry[member].as_object().unwrap() {
          for (slot, node) in slots.as_object().unwrap() {
            let at = format!("{document}: interactions[{index}].{member}.{part}.{slot}");
            pact_janus_kernel::shape::parse(node, &at)
              .unwrap_or_else(|problems| panic!("{at}: {problems:?}"));
          }
        }
      }
    }
  }
}

#[test]
fn every_entry_of_the_mapping_document_is_selected_by_a_request_to_its_operation() {
  // The base path every server agrees on is in the template; the operation whose servers disagree
  // has none, and says so.
  let (shape, losses) = derive("cli/tests/cmd/fixtures/openapi/mapping.openapi.yaml", &[]);
  let templates: BTreeSet<&str> = shape["interactions"]
    .as_array()
    .unwrap()
    .iter()
    .filter_map(|entry| {
      entry
        .pointer("/selector/request/path/template")
        .and_then(Value::as_str)
    })
    .collect();
  assert!(templates.contains("/v1/outcomes/{id}"), "{templates:?}");
  assert!(templates.contains("/no-json"), "{templates:?}");
  assert!(codes(&losses).contains("base-path-ambiguous"));

  // A range is the statuses no other response of the operation names, so the 404 entry and the
  // 4XX one never both select an interaction.
  let range = shape["interactions"]
    .as_array()
    .unwrap()
    .iter()
    .find(|entry| entry["source"]["response"] == "4XX")
    .expect("the 4XX entry");
  let range_codes: Vec<u64> = range["selector"]["response"]["status"]["codes"]
    .as_array()
    .unwrap()
    .iter()
    .filter_map(Value::as_u64)
    .collect();
  assert_eq!(range_codes.len(), 99);
  assert!(!range_codes.contains(&404));
  assert!(
    !shape["interactions"]
      .as_array()
      .unwrap()
      .iter()
      .any(|entry| entry["source"]["response"] == "default")
  );
}

/// S4.4's exit (ADR 0089): one `janus check` run in which a recorded finding and a derived one each
/// take the action their policy names. The sample provider's pact is checked against the shape its
/// tests recorded, and the same pact, under a second provider's name, against the shape its OpenAPI
/// document derives. The policy blocks on findings and warns on derived ones, and the project's
/// `derived` routing survives the run's `--on-finding block`.
#[test]
fn a_policy_blocks_on_recorded_findings_and_warns_on_derived_ones() {
  let pact: Value = serde_json::from_slice(
    &std::fs::read(root().join("samples/order-service/pacts/web-app-order-service.json")).unwrap(),
  )
  .unwrap();
  let mut renamed = pact.clone();
  renamed["provider"]["name"] = Value::from("order-service-derived");
  let renamed_pact = scratch("web-app-order-service-derived.json");
  std::fs::write(&renamed_pact, serde_json::to_vec_pretty(&renamed).unwrap()).unwrap();

  let derived_shape = scratch("order-service-derived.provider-shape.json");
  let (derived, _) = derive(
    "samples/order-service/openapi.yaml",
    &["--provider", "order-service-derived"],
  );
  std::fs::write(&derived_shape, serde_json::to_vec_pretty(&derived).unwrap()).unwrap();

  let policy = scratch("policy.yaml");
  std::fs::write(&policy, "provenance:\n  derived:\n    on-finding: warn\n").unwrap();

  let (code, report) = janus(&[
    "check",
    "samples/order-service/pacts/web-app-order-service.json",
    renamed_pact.to_str().unwrap(),
    "--provider-shape",
    "samples/order-service/shapes/order-service.provider-shape.json",
    "--provider-shape",
    derived_shape.to_str().unwrap(),
    "--policy",
    policy.to_str().unwrap(),
    "--on-finding",
    "block",
    "--output",
    "json",
  ]);
  assert_eq!(code, 1, "a block is the subject failing: {report:#}");
  let decisions: Vec<(&str, &str)> = report["pairs"]
    .as_array()
    .unwrap()
    .iter()
    .map(|pair| {
      (
        pair["provider"]["name"].as_str().unwrap(),
        pair["decision"].as_str().unwrap(),
      )
    })
    .collect();
  assert_eq!(
    decisions,
    vec![("order-service", "block"), ("order-service-derived", "warn")]
  );
  for pair in report["pairs"].as_array().unwrap() {
    let provenance = if pair["provider"]["name"] == "order-service" {
      "recorded"
    } else {
      "derived"
    };
    let reason = pair["reasons"]
      .as_array()
      .unwrap()
      .iter()
      .find(|reason| reason["code"] == "subsumption-findings")
      .unwrap_or_else(|| panic!("{pair:#}"));
    assert!(
      reason["message"]
        .as_str()
        .unwrap()
        .contains(&format!("from {provenance} shapes")),
      "{reason:#}"
    );
    assert!(
      pair["findings"]
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["provenance"] == provenance),
      "{pair:#}"
    );
  }
}
