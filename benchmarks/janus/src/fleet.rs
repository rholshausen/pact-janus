//! Staged plan S4.5: whether a broker can recompute subsumption reports when its cache misses
//! (broker notes §3c, §12). A report is a function of a contract, a provider shape and the engine
//! version (notes §3), so the cache misses in three ways, each a scenario here:
//!
//! - **one pair**: a `can-i-deploy` for a pair no CI run published a report for. One instance of the
//!   engine, its handshake, `subsumption/check` and `subsumption/decide` (ADR 0092 decision 5: an
//!   instance per decision);
//! - **a provider publishes a shape**: every consumer's report against it, N pairs;
//! - **the broker moves its engine pin**: every report it holds, N consumers by M provider versions,
//!   on one thread and on several, since a broker answers requests on several.
//!
//! Each is measured in the WASM embedding a broker hosts (ADR 0092) and linked natively, which is
//! what the Rust port could do instead (S4.5). Both are spoken to in frames, so the difference is
//! the embedding and nothing else.
//!
//! The fleet is generated, not invented: one provider with [`OPERATIONS`] operations, whose shape at
//! each version is recorded by the engine from observed answers, as S4.1 records one, and consumers
//! whose v3 pacts each use some of those operations, written as pact_ffi writes them. Every pair has
//! findings, as real ones do: the shape answers statuses and empty lists the pacts never tested.

use crate::embed::{Client, Pipe};
use crate::util::{RunResults, Samples};
use serde_json::{Value, json};
use std::time::Instant;

/// The provider's operations: `GET /resource-<i>/{id}`.
const OPERATIONS: usize = 40;
/// A wider provider's, to see what a check costs as a shape grows: selection reads every entry.
const WIDE_OPERATIONS: usize = 200;
/// The consumers of the fleet and their contracts' sizes, cycled: most contracts are small, and a
/// few are large.
const CONSUMERS: usize = 50;
const CONTRACT_SIZES: [usize; 5] = [5, 10, 20, 50, 100];
/// Provider versions with a shape each.
const VERSIONS: usize = 10;
/// Threads for the cross-product, as a broker's request workers.
const THREADS: usize = 4;
/// The date every decision is judged against: decisions are a function of it (notes §4).
const AS_OF: &str = "2026-11-18";

const STATUSES: [&str; 3] = ["PENDING", "SHIPPED", "CANCELLED"];

/// One answer of operation `operation`, the `observation`th its tests saw: an order-like document
/// whose status, items and shipping date vary, as a provider's do. A later provider version answers
/// one more status.
fn document(operation: usize, observation: usize, version: usize) -> Value {
  let status = if version % 2 == 1 && observation == 5 {
    "REFUNDED"
  } else {
    STATUSES[observation % STATUSES.len()]
  };
  let items: Vec<Value> = (0..observation % 4)
    .map(|item| json!({ "sku": format!("sku-{item}"), "quantity": item + 1, "price": 2.5 }))
    .collect();
  let mut document = json!({
    "id": format!("{operation}-{observation}"),
    "status": status,
    "total": 10.5,
    "currency": "AUD",
    "customer": { "id": "c-1", "name": "A Customer", "email": "a@example.com" },
    "items": items,
    "createdAt": "2026-07-30T09:00:00Z",
    "tags": ["web", "priority"],
  });
  if status == "SHIPPED" {
    document["shippedAt"] = json!("2026-07-31T09:00:00Z");
  }
  document
}

fn operation(index: usize) -> Value {
  json!({ "request": {
    "method": { "shape": "equality", "example": "GET" },
    "path": { "shape": "http:path", "template": format!("/resource-{index}/{{id}}"), "example": format!("/resource-{index}/id") } } })
}

fn response(status: u16, body: &Value) -> Value {
  use base64::Engine as _;
  json!({
    "status": { "content": status },
    "headers": { "content": { "content-type": ["application/json"] } },
    "body": { "content": base64::engine::general_purpose::STANDARD.encode(body.to_string()), "encoded": "base64",
              "content-type": "application/json" },
  })
}

/// The provider's shape at `version`, recorded by the engine from six answers to each of `operations`
/// operations and one 404, as the provider's own tests would record it (ADR 0086).
fn provider_shape(client: &mut Client, operations: usize, version: usize) -> Value {
  let session = client.ok(
    "provider-shape-session/create",
    json!({ "provider": { "name": "fleet-provider" } }),
  )["session"]
    .clone();
  for index in 0..operations {
    let mut observe = |path: String, answer: Value| {
      client.ok(
        "provider-shape-session/observe",
        json!({ "session": session, "description": format!("GET /resource-{index}/{{id}}"),
                "operation": operation(index),
                "parts": { "request": { "method": { "content": "GET" }, "path": { "content": path } },
                           "response": answer } }),
      );
    };
    for observation in 0..6 {
      observe(
        format!("/resource-{index}/{observation}"),
        response(200, &document(index, observation, version)),
      );
    }
    observe(
      format!("/resource-{index}/missing"),
      response(404, &json!({ "error": "not-found" })),
    );
  }
  client.ok("provider-shape-session/finalise", json!({ "session": session }))["provider-shape"].clone()
}

/// Consumer `consumer`'s v3 pact, of `size` interactions over the provider's operations, as
/// pact_ffi writes one: each an order fetched by id, its body under a type matcher, its items at
/// least one, and its status one of two.
fn consumer_pact(consumer: usize, size: usize) -> Value {
  let interactions: Vec<Value> = (0..size)
    .map(|n| {
      let index = (consumer * 7 + n) % OPERATIONS;
      json!({
        "description": format!("get resource {index}, case {n}"),
        "request": { "method": "GET", "path": format!("/resource-{index}/{n}") },
        "response": {
          "status": 200,
          "headers": { "Content-Type": "application/json" },
          "body": document(index, 1, 0),
          "matchingRules": { "body": {
            "$": { "matchers": [ { "match": "type" } ] },
            "$.items": { "matchers": [ { "match": "type", "min": 1 } ] },
            "$.status": { "matchers": [ { "match": "regex", "regex": "PENDING|SHIPPED" } ] } } }
        }
      })
    })
    .collect();
  json!({
    "consumer": { "name": format!("consumer-{consumer}") },
    "provider": { "name": "fleet-provider" },
    "interactions": interactions,
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  })
}

/// The documents a broker holds for one provider: a shape per version, and each consumer's pact.
pub struct Fleet {
  shapes: Vec<Value>,
  wide: Value,
  pacts: Vec<Value>,
}

impl Fleet {
  /// Recorded through `client`, which must be able to record HTTP answers.
  pub fn generate(client: &mut Client) -> Fleet {
    Fleet {
      shapes: (0..VERSIONS)
        .map(|version| provider_shape(client, OPERATIONS, version))
        .collect(),
      wide: provider_shape(client, WIDE_OPERATIONS, 0),
      pacts: (0..CONSUMERS)
        .map(|consumer| consumer_pact(consumer, CONTRACT_SIZES[consumer % CONTRACT_SIZES.len()]))
        .collect(),
    }
  }
}

/// A fresh engine, as a broker makes one for each decision.
pub trait Instances: Sync {
  fn instance(&self) -> Box<dyn Pipe>;
}

impl<F: Fn() -> Box<dyn Pipe> + Sync> Instances for F {
  fn instance(&self) -> Box<dyn Pipe> {
    self()
  }
}

fn check(client: &mut Client, pact: &Value, shape: &Value) -> Value {
  client.ok(
    "subsumption/check",
    json!({ "contract": pact, "provider-shape": shape }),
  )
}

fn decide(client: &mut Client, pact: &Value, checked: &Value) -> Value {
  client.ok(
    "subsumption/decide",
    json!({
      "pairs": [ { "consumer": pact["consumer"], "provider": pact["provider"],
                   "subsumption": checked["report"], "format": checked["format"] } ],
      "verification": [], "policy": [], "as-of": AS_OF,
    }),
  )
}

/// One cache miss, answered as a broker answers it: a fresh instance, its handshake, the walk and
/// the decision.
fn answer(instances: &dyn Instances, pact: &Value, shape: &Value) -> Value {
  let mut client = Client::new(instances.instance());
  let checked = check(&mut client, pact, shape);
  decide(&mut client, pact, &checked)
}

/// What checking `pact` against `shape` costs each interaction, apart from reading the two documents
/// (S4.11): the median check, less the median check of the same documents with no interactions,
/// shared among the interactions. Answers the reading's median and the share, in µs.
fn per_interaction(client: &mut Client, pact: &Value, shape: &Value, iterations: usize) -> (f64, f64) {
  let median = |samples: Samples| samples.metrics()["median_us"].as_f64().unwrap();
  let interactions = pact["interactions"].as_array().unwrap().len();
  let mut empty = pact.clone();
  empty["interactions"] = json!([]);
  let read = median(Samples::collect(2, iterations, || {
    check(client, &empty, shape);
  }));
  let whole = median(Samples::collect(2, iterations, || {
    check(client, pact, shape);
  }));
  (read, round((whole - read) / interactions as f64, 1))
}

fn round(value: f64, places: i32) -> f64 {
  let scale = 10f64.powi(places);
  (value * scale).round() / scale
}

fn median_of(mut values: Vec<f64>) -> f64 {
  values.sort_by(|a, b| a.partial_cmp(b).unwrap());
  values[values.len() / 2]
}

/// Every scenario, against the embedding `instances` makes.
pub fn scenarios(results: &mut RunResults, fleet: &Fleet, instances: &dyn Instances) {
  let shape = &fleet.shapes[0];

  // One pair, at three sizes of contract, with what each step costs.
  for (size, iterations) in [(5, 200), (50, 40), (500, 10)] {
    let pact = consumer_pact(0, size);
    let samples = Samples::collect(2, iterations, || {
      answer(instances, &pact, shape);
    });
    let mut client = Client::new(instances.instance());
    let checked = check(&mut client, &pact, shape);
    let decided = decide(&mut client, &pact, &checked);
    let pair = &decided["report"]["pairs"][0];
    assert_eq!(
      (&pair["decision"], &pair["subsumption"]["findings"]),
      (&json!("warn"), &json!(size * 3)),
      "each interaction's three findings warn, as the default policy does: {pair}"
    );
    let report_bytes = serde_json::to_vec(&checked).unwrap().len();
    let check_samples = Samples::collect(2, iterations, || {
      check(&mut client, &pact, shape);
    });
    let decide_samples = Samples::collect(2, iterations, || {
      decide(&mut client, &pact, &checked);
    });
    let instance_samples = Samples::collect(2, iterations, || {
      drop(Client::new(instances.instance()));
    });
    let mut metrics = samples.metrics();
    metrics["interactions"] = json!(size);
    metrics["check_median_us"] = check_samples.metrics()["median_us"].clone();
    metrics["decide_median_us"] = decide_samples.metrics()["median_us"].clone();
    metrics["instance_median_us"] = instance_samples.metrics()["median_us"].clone();
    metrics["report_bytes"] = json!(report_bytes);
    if size == 50 {
      let (read, share) = per_interaction(&mut client, &pact, shape, iterations);
      metrics["read_median_us"] = json!(read);
      metrics["per_interaction_us"] = json!(share);
    }
    results.record(&format!("subsumption-pair-{size}"), metrics);
  }

  // The same 50 interactions against a provider with five times the operations. Reading a document
  // five times the size costs five times as much; what each interaction costs is selection's share,
  // which grew with the entries until S4.11 (ADR 0096).
  let pact = consumer_pact(0, 50);
  let samples = Samples::collect(2, 40, || {
    answer(instances, &pact, &fleet.wide);
  });
  let mut metrics = samples.metrics();
  metrics["interactions"] = json!(50);
  metrics["entries"] = json!(fleet.wide["interactions"].as_array().unwrap().len());
  let (read, share) = per_interaction(&mut Client::new(instances.instance()), &pact, &fleet.wide, 40);
  metrics["read_median_us"] = json!(read);
  metrics["per_interaction_us"] = json!(share);
  results.record("subsumption-pair-50-wide-shape", metrics);

  // The cache hit: a stored report, decided again today, which a broker does on every read (notes §4).
  let pact = consumer_pact(0, 50);
  let mut client = Client::new(instances.instance());
  let checked = check(&mut client, &pact, shape);
  let samples = Samples::collect(5, 200, || {
    let mut client = Client::new(instances.instance());
    decide(&mut client, &pact, &checked);
  });
  results.record("subsumption-decide-stored-50", samples.metrics());

  let interactions: usize = fleet
    .pacts
    .iter()
    .map(|pact| pact["interactions"].as_array().unwrap().len())
    .sum();

  // A provider publishes a shape: each consumer's report against it, one after another.
  let runs: Vec<f64> = (0..5)
    .map(|_| {
      let started = Instant::now();
      for pact in &fleet.pacts {
        answer(instances, pact, shape);
      }
      started.elapsed().as_secs_f64() * 1e3
    })
    .collect();
  let median = median_of(runs);
  results.record(
    "subsumption-fleet-new-shape",
    json!({ "runs": 5, "pairs": fleet.pacts.len(), "interactions": interactions,
            "median_ms": round(median, 1), "median_ms_per_pair": round(median / fleet.pacts.len() as f64, 2) }),
  );

  // The engine pin moves: every report, on one thread and on several.
  let pairs: Vec<(&Value, &Value)> = fleet
    .shapes
    .iter()
    .flat_map(|shape| fleet.pacts.iter().map(move |pact| (pact, shape)))
    .collect();
  for threads in [1, THREADS] {
    let runs: Vec<f64> = (0..3)
      .map(|_| {
        let started = Instant::now();
        std::thread::scope(|scope| {
          for worker in 0..threads {
            let pairs = &pairs;
            scope.spawn(move || {
              for (pact, shape) in pairs.iter().skip(worker).step_by(threads) {
                answer(instances, pact, shape);
              }
            });
          }
        });
        started.elapsed().as_secs_f64() * 1e3
      })
      .collect();
    let median = median_of(runs);
    results.record(
      &format!("subsumption-fleet-all-pairs-{threads}threads"),
      json!({ "runs": 3, "pairs": pairs.len(), "threads": threads,
              "interactions": interactions * fleet.shapes.len(),
              "median_ms": round(median, 1),
              "pairs_per_second": round(pairs.len() as f64 / (median / 1e3), 1) }),
    );
  }
}
