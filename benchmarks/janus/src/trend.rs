//! Staged plan S0.3: a run's result documents, as the points a trend chart plots.
//!
//! A result document (schema 1, this directory's README) holds a scenario's every metric. A trend
//! wants one number per line on a chart, so this picks the numbers worth a line and says which way
//! is better. It is a table and not a guess: a scenario that records a metric this does not know
//! contributes nothing for it, and a test over the checked-in results fails when a scenario
//! contributes nothing at all.
//!
//! The output is what `benchmark-action/github-action-benchmark` reads as `customSmallerIsBetter`
//! and `customBiggerIsBetter`: a list of `{ name, unit, value, extra }`. It takes one list per
//! direction, so there are two files.

use serde_json::{Value, json};
use std::path::Path;

/// Which way is better for a metric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Better {
  Smaller,
  Bigger,
}

/// The metrics that get a line: the key a scenario records it under, what to add to the
/// scenario's name (nothing for its headline number), the unit, and the direction.
const METRICS: &[(&str, &str, &str, Better)] = &[
  ("median_us", "", "µs", Better::Smaller),
  // A mock exchange is a `serve-variant` frame and then the request; this is the frame's share,
  // and the one S2.3's plan cache is meant to move (performance report §5.3).
  ("arm_median_us", " · arming", "µs", Better::Smaller),
  (
    "median_ms_per_interaction",
    "",
    "ms per interaction",
    Better::Smaller,
  ),
  ("median_ms_all_variants", "", "ms", Better::Smaller),
  (
    "median_ms_per_extra_variant",
    " · per extra variant",
    "ms",
    Better::Smaller,
  ),
  ("median_ms_finalise", " · finalise", "ms", Better::Smaller),
  ("compile_median_ms", " · compile", "ms", Better::Smaller),
  ("component_bytes", " · component size", "bytes", Better::Smaller),
  ("rps", "", "requests per second", Better::Bigger),
];

/// One point on one chart.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
  pub name: String,
  pub unit: &'static str,
  pub value: f64,
  pub extra: String,
  pub better: Better,
}

/// The embedding a result document is for, from its `stack` (`janus-<embedding>-<commit>`).
fn embedding(document: &Value) -> Option<&str> {
  document
    .get("stack")?
    .as_str()?
    .strip_prefix("janus-")?
    .split('-')
    .next()
}

/// What sits beside a point in the chart's tooltip: the spread and the sample size, where the
/// scenario recorded them.
fn extra(metrics: &Value) -> String {
  let mut parts = Vec::new();
  for (key, label) in [("min_us", "min"), ("p95_us", "p95")] {
    if let Some(value) = metrics.get(key).and_then(Value::as_f64) {
      parts.push(format!("{label} {value} µs"));
    }
  }
  for key in ["iterations", "runs", "interactions", "variants", "engines"] {
    if let Some(value) = metrics.get(key).and_then(Value::as_u64) {
      parts.push(format!("{key} {value}"));
    }
  }
  parts.join(", ")
}

/// Every point one result document contributes, named `<scenario><what> (<embedding>)` so the
/// embeddings of one scenario sort together.
pub fn points(document: &Value) -> Vec<Point> {
  let Some(embedding) = embedding(document) else {
    return Vec::new();
  };
  let mut points = Vec::new();
  for (scenario, metrics) in document
    .get("scenarios")
    .and_then(Value::as_object)
    .into_iter()
    .flatten()
  {
    for (key, suffix, unit, better) in METRICS {
      if let Some(value) = metrics.get(*key).and_then(Value::as_f64) {
        points.push(Point {
          name: format!("{scenario}{suffix} ({embedding})"),
          unit,
          value,
          // The spread is the headline number's; beside any other it would be about something else.
          extra: if suffix.is_empty() {
            extra(metrics)
          } else {
            String::new()
          },
          better: *better,
        });
      }
    }
  }
  points
}

fn as_json(points: &[Point], better: Better) -> Value {
  Value::Array(
    points
      .iter()
      .filter(|point| point.better == better)
      .map(
        |point| json!({ "name": point.name, "unit": point.unit, "value": point.value, "extra": point.extra }),
      )
      .collect(),
  )
}

/// `trend <results-dir> <out-dir>`: every result document in `results-dir`, as
/// `smaller-is-better.json` and `bigger-is-better.json` in `out-dir`. Returns how many points.
pub fn write(results: &Path, out: &Path) -> Result<usize, String> {
  let mut files: Vec<_> = std::fs::read_dir(results)
    .map_err(|err| format!("reading {}: {err}", results.display()))?
    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
    .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
    .collect();
  files.sort();
  if files.is_empty() {
    return Err(format!("no result documents in {}", results.display()));
  }
  let mut all = Vec::new();
  for file in &files {
    let text = std::fs::read_to_string(file).map_err(|err| format!("reading {}: {err}", file.display()))?;
    let document: Value =
      serde_json::from_str(&text).map_err(|err| format!("parsing {}: {err}", file.display()))?;
    let found = points(&document);
    if found.is_empty() {
      return Err(format!(
        "{} contributes no points: not a Janus result document?",
        file.display()
      ));
    }
    all.extend(found);
  }
  all.sort_by(|a, b| a.name.cmp(&b.name));
  std::fs::create_dir_all(out).map_err(|err| format!("creating {}: {err}", out.display()))?;
  for (name, better) in [
    ("smaller-is-better.json", Better::Smaller),
    ("bigger-is-better.json", Better::Bigger),
  ] {
    let text = serde_json::to_string_pretty(&as_json(&all, better)).expect("points are JSON");
    std::fs::write(out.join(name), text).map_err(|err| format!("writing {name}: {err}"))?;
  }
  Ok(all.len())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn document() -> Value {
    json!({
      "schema": 1, "date": "2026-10-11", "stack": "janus-subprocess-fce02a0", "os": "linux", "arch": "x86_64",
      "scenarios": {
        "mock-server-latency-small": { "arm_median_us": 14.9, "iterations": 5000, "median_us": 34.7, "min_us": 30.7, "p95_us": 56.1 },
        "mock-server-throughput-4engines": { "engines": 4, "requests": 157465, "rps": 78733.0, "seconds": 2 },
        "verify-corpus-small": { "interactions": 50, "median_ms_per_interaction": 0.29, "median_total_ms": 14.4, "pacts": 10, "runs": 5 },
        "unsupported": { "verification/verify": { "code": "component-unavailable" } }
      }
    })
  }

  #[test]
  fn a_scenario_gets_a_line_for_each_metric_worth_one_named_for_its_embedding() {
    let points = points(&document());
    let names: Vec<&str> = points.iter().map(|point| point.name.as_str()).collect();
    assert_eq!(
      names,
      vec![
        "mock-server-latency-small (subprocess)",
        "mock-server-latency-small · arming (subprocess)",
        "mock-server-throughput-4engines (subprocess)",
        "verify-corpus-small (subprocess)",
      ]
    );
    assert_eq!(points[0].value, 34.7);
    assert_eq!(points[0].unit, "µs");
    assert_eq!(points[0].extra, "min 30.7 µs, p95 56.1 µs, iterations 5000");
  }

  #[test]
  fn throughput_is_the_one_where_bigger_is_better() {
    let points = points(&document());
    let bigger = as_json(&points, Better::Bigger);
    assert_eq!(
      bigger,
      json!([ { "name": "mock-server-throughput-4engines (subprocess)", "unit": "requests per second",
                "value": 78733.0, "extra": "engines 4" } ])
    );
    assert_eq!(as_json(&points, Better::Smaller).as_array().unwrap().len(), 3);
  }

  /// The checked-in results are what the harness really writes. Every scenario in them is on the
  /// chart, except the one that records what an embedding refuses rather than how long it takes —
  /// so a scenario added with metrics this table does not know is noticed here, not as a gap in a
  /// chart months later.
  #[test]
  fn every_recorded_scenario_reaches_the_chart() {
    let results = Path::new(env!("CARGO_MANIFEST_DIR")).join("../results");
    let mut checked = 0;
    for entry in std::fs::read_dir(&results).expect("benchmarks/results") {
      let path = entry.unwrap().path();
      let document: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
      let Some(embedding) = embedding(&document) else {
        continue; // the pact_ffi baseline: not Janus, and not on this chart
      };
      let points = points(&document);
      for scenario in document["scenarios"].as_object().unwrap().keys() {
        if scenario == "unsupported" {
          continue;
        }
        assert!(
          points
            .iter()
            .any(|point| point.name.starts_with(scenario.as_str())
              && point.name.ends_with(&format!("({embedding})"))),
          "{}: '{scenario}' records nothing the trend knows how to plot",
          path.display()
        );
        checked += 1;
      }
    }
    assert!(
      checked > 40,
      "expected the checked-in Janus results, checked {checked} scenarios"
    );
  }

  #[test]
  fn a_directory_of_results_becomes_the_two_files_the_chart_reads() {
    let dir = std::env::temp_dir().join(format!("janus-trend-{}", std::process::id()));
    let (results, out) = (dir.join("results"), dir.join("out"));
    std::fs::create_dir_all(&results).unwrap();
    std::fs::write(results.join("a.json"), document().to_string()).unwrap();
    assert_eq!(write(&results, &out), Ok(4));
    let smaller: Value =
      serde_json::from_str(&std::fs::read_to_string(out.join("smaller-is-better.json")).unwrap()).unwrap();
    assert_eq!(smaller.as_array().unwrap().len(), 3);
    assert!(out.join("bigger-is-better.json").exists());
    assert!(
      write(&out, &dir.join("again")).is_err(),
      "a list of points is not a result document"
    );
    std::fs::remove_dir_all(&dir).unwrap();
  }
}
