//! Spike 7.3: derive provider shapes from OpenAPI, and measure how noisy the findings get.
//!
//! `cargo run -- report` is the experiment: one API, one consumer contract, and the same
//! subsumption checker run against three provider shapes of that API — recorded from the live
//! provider (task 7.2), derived from a hand-authored OpenAPI spec, and derived from an
//! ORM-generator-style one. The noise difference between the last two is what this spike is for.
//!
//! `cargo run -- derive <spec.json>` prints one derived provider shape, for looking at.

mod derive;

use derive::{OperationName, derive};
use pact_janus_kernel::contract::{Contract, IdentifyMode, read as read_contract};
use pact_janus_kernel::subsumption::{
  Recorder, Severity, SubsumptionReport, check, read_provider_shape,
};
use pact_janus_sample_order_service::{Config, DEFAULT_TOKEN, start};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn corpus(name: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus").join(name)
}

fn read_json(name: &str) -> Value {
  let path = corpus(name);
  let bytes = std::fs::read(&path).unwrap_or_else(|err| panic!("reading {path:?}: {err}"));
  serde_json::from_slice(&bytes).unwrap_or_else(|err| panic!("parsing {path:?}: {err}"))
}

fn operation_names() -> BTreeMap<String, OperationName> {
  read_json("operation-map.json")
    .as_object()
    .expect("an object")
    .iter()
    .filter(|(operation_id, _)| !operation_id.starts_with('$'))
    .map(|(operation_id, name)| {
      (
        operation_id.clone(),
        OperationName {
          description: name["description"].as_str().expect("a description").to_string(),
          states: name
            .get("states")
            .and_then(Value::as_array)
            .map(|states| {
              states
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
            })
            .unwrap_or_default(),
        },
      )
    })
    .collect()
}

fn consumer() -> Contract {
  let document = read_json("consumer-contract.json");
  read_contract(document.to_string().as_bytes(), IdentifyMode::Tolerant)
    .expect("a well-formed consumer contract")
}

/// Record the live sample provider, the way task 7.2's demonstration does: the baseline every
/// derived shape is measured against.
fn recorded() -> Value {
  let provider = start(Config::default()).expect("the sample provider binds");
  let base_url = provider.base_url();
  let mut recorder = Recorder::new("order-service");
  for (status, shipped, items) in [
    ("PENDING", false, 1),
    ("SHIPPED", true, 2),
    ("PENDING", false, 3),
    ("CANCELLED", false, 0),
    ("SHIPPED", true, 1),
    ("PENDING", false, 2),
  ] {
    let agent: ureq::Agent = ureq::Agent::config_builder()
      .http_status_as_error(false)
      .build()
      .into();
    agent
      .post(format!("{base_url}/_pact/provider-states"))
      .header("content-type", "application/json")
      .send(
        json!({ "action": "setup", "state": "an order exists",
                "params": { "id": "66", "status": status, "shipped": shipped, "items": items } })
          .to_string(),
      )
      .expect("the provider answers");
    let mut response = agent
      .get(format!("{base_url}/orders/66"))
      .header("authorization", &format!("Bearer {DEFAULT_TOKEN}"))
      .call()
      .expect("the provider answers");
    let body: Value = serde_json::from_str(&response.body_mut().read_to_string().expect("a body"))
      .expect("json");
    recorder.observe(
      "a request for an order",
      &["an order exists".to_string()],
      &BTreeMap::from([(
        "response".to_string(),
        BTreeMap::from([("body".to_string(), body)]),
      )]),
    );
  }
  serde_json::to_value(recorder.finish()).expect("serializes")
}

/// Counts for one provider shape: what the checker said, split the way a policy would read it.
struct Measurement {
  label: &'static str,
  gaps: usize,
  findings: usize,
  reviews: usize,
  by_kind: BTreeMap<String, usize>,
  report: SubsumptionReport,
}

fn measure(label: &'static str, provider_shape: &Value, gaps: usize) -> Measurement {
  let published = read_provider_shape(provider_shape.to_string().as_bytes())
    .unwrap_or_else(|err| panic!("{label}: not a readable provider shape: {err}"));
  let report = check(&consumer(), &published)
    .unwrap_or_else(|err| panic!("{label}: the check could not run: {err}"));

  let mut by_kind = BTreeMap::new();
  let mut findings = 0;
  let mut reviews = 0;
  for interaction in &report.interactions {
    for finding in &interaction.findings {
      match finding.severity {
        Severity::Finding => findings += 1,
        Severity::Review => reviews += 1,
        Severity::Advisory => {}
      }
      *by_kind.entry(finding.kind.clone()).or_insert(0) += 1;
    }
  }
  Measurement {
    label,
    gaps,
    findings,
    reviews,
    by_kind,
    report,
  }
}

fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  match args.first().map(String::as_str) {
    Some("derive") => {
      let spec = read_json(args.get(1).map(String::as_str).unwrap_or("order-service.hand-authored.openapi.json"));
      let derived = derive(&spec, "order-service", &operation_names());
      println!("{}", serde_json::to_string_pretty(&derived.provider_shape).unwrap());
      for gap in &derived.gaps {
        eprintln!("gap [{}] {}: {}", gap.code, gap.path, gap.detail);
      }
    }
    Some("gaps") => {
      let spec = read_json("mapping-gaps.openapi.json");
      let derived = derive(&spec, "gap-service", &operation_names());
      for gap in &derived.gaps {
        println!("{:<24} {:<44} {}", gap.code, gap.path, gap.detail);
      }
      println!("\n{}", serde_json::to_string_pretty(&derived.provider_shape).unwrap());
    }
    Some("pathologies") => pathologies(),
    Some("unmapped") => {
      // FINDINGS §2: the same derivation with no operation-map file, which is what a team that
      // just points the importer at its own spec actually gets.
      let spec = read_json("order-service.hand-authored.openapi.json");
      let derived = derive(&spec, "order-service", &BTreeMap::new());
      let published =
        read_provider_shape(derived.provider_shape.to_string().as_bytes()).expect("readable");
      println!("derived interaction description: {:?}", published.interactions[0].description);
      let report = check(&consumer(), &published).expect("the check runs");
      println!("verdict: {}", report.interactions[0].verdict);
      println!("findings: {}", report.summary.findings);
      println!("--- rendered ---\n{}", pact_janus_kernel::subsumption::render(&report));
    }
    _ => report(),
  }
}

/// Each generator behaviour, applied to the hand-authored spec one at a time: what does each one
/// cost, measured rather than asserted (FINDINGS §4).
fn pathologies() {
  let base = read_json("order-service.hand-authored.openapi.json");
  let names = operation_names();
  let cases: Vec<(&str, fn(&mut Value))> = vec![
    ("none (hand-authored)", |_| {}),
    ("no required list", |spec| {
      each_schema(spec, &|schema| {
        schema.remove("required");
      })
    }),
    ("nullable on every field", |spec| {
      each_property(spec, &|property| {
        property.insert("nullable".to_string(), Value::Bool(true));
      })
    }),
    ("enums flattened to their storage type", |spec| {
      each_schema(spec, &|schema| {
        schema.remove("enum");
      });
      each_property(spec, &|property| {
        property.remove("enum");
      })
    }),
    ("integer reported as number", |spec| {
      each_property(spec, &|property| {
        if property.get("type") == Some(&Value::from("integer")) {
          property.insert("type".to_string(), Value::from("number"));
        }
      })
    }),
  ];

  println!("{:<42} {:>8}  {}", "generator behaviour, applied alone", "findings", "by kind");
  for (label, apply) in &cases {
    let mut spec = base.clone();
    apply(&mut spec);
    let derived = derive(&spec, "order-service", &names);
    let measurement = measure_owned(label, &derived.provider_shape);
    println!(
      "{:<42} {:>8}  {}",
      label,
      measurement.0,
      measurement
        .1
        .iter()
        .map(|(kind, count)| format!("{kind}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
    );
  }
}

fn measure_owned(label: &str, provider_shape: &Value) -> (usize, BTreeMap<String, usize>) {
  let published = read_provider_shape(provider_shape.to_string().as_bytes())
    .unwrap_or_else(|err| panic!("{label}: not readable: {err}"));
  let report = check(&consumer(), &published).expect("the check runs");
  let mut by_kind = BTreeMap::new();
  let mut findings = 0;
  for interaction in &report.interactions {
    for finding in &interaction.findings {
      if finding.severity == Severity::Finding {
        findings += 1;
      }
      *by_kind.entry(finding.kind.clone()).or_insert(0) += 1;
    }
  }
  (findings, by_kind)
}

/// Every component schema in the document.
fn each_schema(spec: &mut Value, apply: &dyn Fn(&mut serde_json::Map<String, Value>)) {
  let Some(schemas) = spec
    .get_mut("components")
    .and_then(|c| c.get_mut("schemas"))
    .and_then(Value::as_object_mut)
  else {
    return;
  };
  for schema in schemas.values_mut() {
    if let Some(object) = schema.as_object_mut() {
      apply(object);
    }
  }
}

/// Every property of every component schema.
fn each_property(spec: &mut Value, apply: &dyn Fn(&mut serde_json::Map<String, Value>)) {
  each_schema(spec, &|schema| {
    let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) else {
      return;
    };
    for property in properties.values_mut() {
      if let Some(object) = property.as_object_mut() {
        apply(object);
      }
    }
  });
}

fn report() {
  let names = operation_names();
  let hand = derive(&read_json("order-service.hand-authored.openapi.json"), "order-service", &names);
  let orm = derive(&read_json("order-service.orm-generated.openapi.json"), "order-service", &names);

  let measurements = [
    measure("recorded (task 7.2, live provider)", &recorded(), 0),
    measure("derived, hand-authored OpenAPI", &hand.provider_shape, hand.gaps.len()),
    measure("derived, ORM-generated OpenAPI", &orm.provider_shape, orm.gaps.len()),
  ];

  println!("One API, one consumer contract, three provider shapes of it.\n");
  println!(
    "{:<36} {:>8} {:>8} {:>8}  {}",
    "provider shape", "findings", "reviews", "gaps", "by kind"
  );
  for measurement in &measurements {
    let kinds: Vec<String> = measurement
      .by_kind
      .iter()
      .map(|(kind, count)| format!("{kind}={count}"))
      .collect();
    println!(
      "{:<36} {:>8} {:>8} {:>8}  {}",
      measurement.label,
      measurement.findings,
      measurement.reviews,
      measurement.gaps,
      kinds.join(" ")
    );
  }

  for measurement in &measurements {
    println!("\n--- {} ---", measurement.label);
    println!("{}", pact_janus_kernel::subsumption::render(&measurement.report));
  }

  println!("\n--- mapping gaps, hand-authored ---");
  for gap in &hand.gaps {
    println!("  [{}] {}: {}", gap.code, gap.path, gap.detail);
  }
  println!("\n--- mapping gaps, ORM-generated ---");
  for gap in &orm.gaps {
    println!("  [{}] {}: {}", gap.code, gap.path, gap.detail);
  }
}
