//! `janus check` (plan task 7.4): the RFC's `can-i-deploy` question, answered locally.
//!
//! **It decides, it does not test.** The two inputs are documents that already exist — the
//! consumer contracts (or v1–v4 pacts) a provider has, the shapes that provider published, and
//! the verification summaries `janus verify --output json` wrote — so this command needs no provider
//! running, no network and no broker. That is what makes it a CI step rather than a second test
//! run: `verify` produces evidence, `check` reads it, and a pipeline that has already verified
//! does not verify again to find out whether it may deploy.
//!
//! Three flags, three layers, in design 2.8 §7.1's own order: the specification's defaults live
//! in the engine, `--policy` is the project's document, and `--on-finding`/`--on-review` are the
//! per-run override. They are two flags rather than the plan's single `--policy warn|block`
//! because §7.1 resolves two independent questions, and "block on decided findings, warn on what
//! the checker cannot decide" — the combination ADR 0016 expects a team to move to first — has no
//! spelling in one flag.
//!
//! What this command does *not* do is merge those layers itself, or match a single exemption to a
//! single finding. Both are design 2.8 §7's semantics, and both live behind `subsumption/decide`
//! (engine-protocol spec §8.6) for the reason every other command here is thin: an SDK, a broker
//! or a GitHub Action asking the same question must get the same answer, and a policy rule
//! implemented twice is a policy rule that will eventually disagree with itself.

use crate::Outcome;
use crate::args::{CheckArgs, Output};
use crate::engine;
use crate::error::Error;
use crate::io;
use serde_json::{Value, json};

pub fn run(args: &CheckArgs) -> Result<Outcome, Error> {
  let contracts = io::read_all_documents(&args.sources)?;

  let mut shapes = Vec::new();
  for path in &args.provider_shapes {
    shapes.extend(io::read_provider_shapes(path)?);
  }

  let mut verifications = Vec::new();
  for path in &args.verifications {
    verifications.push(io::read_json(path)?);
  }

  // Layer 2 (design 2.8 §7.1). The layers travel separately — the engine resolves them, because
  // "scalars override, exemptions accumulate" is the rule a host must not own (ADR 0016).
  let mut policy_layers = Vec::new();
  if let Some(path) = &args.policy {
    policy_layers.push(io::read_config(path)?);
  }
  // Layer 3: the per-run override, as a policy document with only what was overridden in it.
  let mut override_layer = serde_json::Map::new();
  if let Some(severity) = args.on_finding {
    override_layer.insert("on-finding".to_string(), json!(severity.as_str()));
  }
  if let Some(severity) = args.on_review {
    override_layer.insert("on-review".to_string(), json!(severity.as_str()));
  }
  if !override_layer.is_empty() {
    policy_layers.push(Value::Object(override_layer));
  }

  let as_of = args.as_of.clone().unwrap_or_else(crate::date::today);

  let mut engine = engine::start()?;

  // One `subsumption/check` per pair the host can form, then one `subsumption/decide` over the
  // lot. The pairing is the CLI's own work — which files it was handed is exactly the knowledge
  // the engine deliberately does not have (protocol spec §8.3's "fetching from files, URLs or a
  // broker is host/CLI business").
  let shape_for = |contract: &Value| {
    let provider = &contract["provider"]["name"];
    shapes
      .iter()
      .find(|(_, shape)| &shape["provider"]["name"] == provider)
  };
  let mut pairs = Vec::new();
  for (path, contract) in &contracts {
    let mut pair = json!({
      "consumer": { "name": contract["consumer"]["name"].as_str().unwrap_or_default() },
      "provider": { "name": contract["provider"]["name"].as_str().unwrap_or_default() },
    });
    if let Some((shape_path, shape)) = shape_for(contract) {
      let result = engine::call(
        &mut engine,
        "subsumption/check",
        json!({ "contract": contract, "provider-shape": shape }),
      )
      .map_err(|err| format!("{path} against {shape_path}: {err}"))?;
      pair["subsumption"] = result["report"].clone();
      pair["format"] = result["format"].clone();
    }
    pairs.push(pair);
  }

  let decided = engine::call(
    &mut engine,
    "subsumption/decide",
    json!({
      "pairs": pairs,
      "verification": verifications,
      "policy": policy_layers,
      "as-of": as_of,
    }),
  )?;

  match args.output {
    Output::Json => outln!("{}", crate::output::pretty(&decided["report"])),
    Output::Text => outln!("{}", decided["text"].as_str().unwrap_or_default()),
  }
  // Which shapes were found, and which were not, is the CLI's half of the answer: the engine was
  // never told a shape file existed for a provider it has no document for. On stderr, so it is said
  // whichever form stdout is in.
  for (path, contract) in &contracts {
    if shape_for(contract).is_none() {
      let provider = contract["provider"]["name"].as_str().unwrap_or_default();
      errln!("janus check: no provider shape supplied for '{provider}' ({path})");
    }
  }

  // A `block` is the subject failing, not the command failing (protocol spec §10.2's own
  // distinction): the check ran, and its answer is no.
  Ok(match decided["report"]["decision"].as_str() {
    Some("block") => Outcome::SubjectFailed,
    _ => Outcome::Done,
  })
}
