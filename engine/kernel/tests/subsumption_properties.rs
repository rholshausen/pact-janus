//! Plan task 7.1's second half: "property-test it — generate value spaces, check `admits`
//! agreement between the checker's verdict and brute-force sampling."
//!
//! The oracle is the engine's own matcher, not a reimplementation of it: shape spec §7.1 defines
//! matching as `v ∈ admits(S)`, so compiling a shape to a plan (task 3.3) and executing it against
//! a candidate value (task 3.4) *is* `admits`. Every shape below is evaluated against one fixed
//! value universe to get its admitted set, and the checker's verdict for a pair is then held to
//! shape spec §8's contract:
//!
//! - `yes`  MUST mean `admits(P) ⊆ admits(C)` — the dangerous direction, since "reporting a wrong
//!   `yes` is how a checker loses its users";
//! - `no`   MUST mean it is not;
//! - `unknown` constrains nothing, and is checked only for not being used where §8 promises a
//!   decision (the exact-class vocabulary in [`vocabulary`] is entirely decidable, so an `unknown`
//!   anywhere in the exhaustive pass is itself a failure).
//!
//! The exhaustive pass covers every ordered pair of a hand-picked vocabulary, where the universe
//! is rich enough to witness every difference. The generated pass builds random nested trees,
//! where it is not — a `no` there may be correct without a witness in the universe — so it asserts
//! the soundness direction only, and says so.

use pact_janus_kernel::interaction_spec::parse as parse_spec;
use pact_janus_kernel::plan::{Assignment, CapturedValues, Status, compile, execute, outcome};
use pact_janus_kernel::shape::{ShapeNode, parse as parse_shape};
use pact_janus_kernel::subsumption::{Verdict, compare};
use serde_json::{Value, json};
use std::collections::BTreeMap;

// --- the value universe ------------------------------------------------------------------------

/// The candidate values a member may hold, including `⊥` (represented by the member being absent
/// from the enclosing object — shape spec §5.1's reason for testing at member position).
fn universe() -> Vec<Value> {
  vec![
    Value::Null,
    json!(true),
    json!(false),
    json!(0),
    json!(1),
    json!(2),
    json!(3),
    json!(-1),
    json!(1.5),
    json!(""),
    json!("a"),
    json!("b"),
    json!("1"),
    json!("1.0.0"),
    json!([]),
    json!([1]),
    json!([1, 2]),
    json!([1, 2, 3]),
    json!([1.5]),
    json!(["a"]),
    json!({}),
    json!({ "a": 1 }),
    json!({ "a": "x" }),
    json!({ "b": 2 }),
    json!({ "a": 1, "b": 2 }),
  ]
}

/// A candidate slot value, including absence: `None` is `⊥`.
fn candidates() -> Vec<Option<Value>> {
  std::iter::once(None)
    .chain(universe().into_iter().map(Some))
    .collect()
}

/// The set of candidates a shape admits, decided by the engine's own matcher (shape spec §7.1) and
/// nothing else. It was not always uncorrected: this test's first run found that a structural
/// operator's plan carried no kind assertion, so the compiler admitted a string where the
/// specification did not (phase-9 finding 8, since fixed — plan-grammar spec §5.2's kind guard).
/// Any correction here again would mean the oracle and the checker disagree about `admits`, which
/// is the thing this file exists to detect rather than to paper over.
fn admits(member: &Value) -> Vec<bool> {
  let document = json!({
    "description": "property",
    "parts": { "response": { "body": { "shape": "object", "members": { "x": member } } } }
  });
  let spec =
    parse_spec(&document).unwrap_or_else(|err| panic!("ill-formed spec for {member}: {:?}", err.problems));
  let plan = compile(&spec, &Assignment::new(), None);
  candidates()
    .iter()
    .map(|candidate| {
      let body = match candidate {
        None => json!({}),
        Some(value) => json!({ "x": value }),
      };
      let mut values = BTreeMap::new();
      values.insert("$.response.body".to_string(), body);
      let resolver = CapturedValues::from_json(&values);
      outcome(&execute(&plan, &resolver)).0 == Status::Matched
    })
    .collect()
}

fn subset(provider: &[bool], consumer: &[bool]) -> bool {
  provider
    .iter()
    .zip(consumer)
    .all(|(in_provider, in_consumer)| !in_provider || *in_consumer)
}

/// The first candidate the provider admits and the consumer does not — the witness that makes a
/// `no` checkable rather than merely asserted.
fn witness(provider: &[bool], consumer: &[bool]) -> Option<String> {
  let candidates = candidates();
  provider
    .iter()
    .zip(consumer)
    .position(|(in_provider, in_consumer)| *in_provider && !*in_consumer)
    .map(|index| match &candidates[index] {
      None => "⊥ (absent)".to_string(),
      Some(value) => value.to_string(),
    })
}

/// Compare two member shapes, as the walk sees them: at member position inside an object, which is
/// the one place a `⊥`-admitting shape may sit (shape spec §5.1).
fn verdict(provider: &Value, consumer: &Value) -> Verdict {
  compare(
    &member_object(provider),
    &member_object(consumer),
    "response.body",
  )
  .0
}

fn member_object(member: &Value) -> ShapeNode {
  let document = json!({ "shape": "object", "members": { "x": member } });
  parse_shape(&document, "/body").unwrap_or_else(|problems| panic!("ill-formed shape {member}: {problems:?}"))
}

// --- the exhaustive pass -------------------------------------------------------------------------

/// Shapes from shape spec §8's **exact** class only, every one of which §8 promises a decision
/// for against every other. The universe above witnesses every difference between them.
fn vocabulary() -> Vec<Value> {
  vec![
    json!({ "shape": "any" }),
    json!({ "shape": "string", "example": "a" }),
    json!({ "shape": "number", "example": 1 }),
    json!({ "shape": "integer", "example": 1 }),
    json!({ "shape": "decimal", "example": 1.5 }),
    json!({ "shape": "boolean", "example": true }),
    json!({ "shape": "null" }),
    json!({ "shape": "equality", "example": 1 }),
    json!({ "shape": "equality", "example": "a" }),
    json!({ "shape": "type", "example": 1 }),
    json!({ "shape": "type", "example": "a" }),
    json!({ "shape": "semver", "example": "1.0.0" }),
    json!({ "shape": "any-of", "options": [1, 2], "example": 1 }),
    json!({ "shape": "any-of", "options": [1, 2, 3], "example": 1 }),
    json!({ "shape": "any-of", "options": ["a", "b"], "example": "a" }),
    json!({ "shape": "any-of", "options": [null, "a"], "example": "a" }),
    json!({ "shape": "optional", "of": { "shape": "integer", "example": 1 } }),
    json!({ "shape": "optional", "of": { "shape": "any-of", "options": [1, 2], "example": 1 } }),
    json!({ "shape": "nullable", "of": { "shape": "integer", "example": 1 } }),
    json!({ "shape": "optional",
            "of": { "shape": "nullable", "of": { "shape": "integer", "example": 1 } } }),
    json!({ "shape": "forbidden" }),
    json!({ "shape": "each-like", "min": 0, "items": { "shape": "integer", "example": 1 } }),
    json!({ "shape": "each-like", "min": 1, "items": { "shape": "integer", "example": 1 } }),
    json!({ "shape": "each-like", "min": 1, "max": 2, "items": { "shape": "integer", "example": 1 } }),
    json!({ "shape": "each-like", "min": 1, "items": { "shape": "number", "example": 1 } }),
    json!({ "shape": "array", "entries": [ { "shape": "integer", "example": 1 } ] }),
    json!({ "shape": "object", "members": { } }),
    json!({ "shape": "object", "members": { "a": { "shape": "integer", "example": 1 } } }),
    json!({ "shape": "object",
            "members": { "a": { "shape": "optional", "of": { "shape": "integer", "example": 1 } } } }),
  ]
}

#[test]
fn every_exact_class_pair_agrees_with_brute_force_admits() {
  let vocabulary = vocabulary();
  let sets: Vec<Vec<bool>> = vocabulary.iter().map(admits).collect();

  let mut checked = 0;
  for (p_index, provider) in vocabulary.iter().enumerate() {
    for (c_index, consumer) in vocabulary.iter().enumerate() {
      let verdict = verdict(provider, consumer);
      let contained = subset(&sets[p_index], &sets[c_index]);
      match verdict {
        Verdict::Yes => assert!(
          contained,
          "the checker said yes, but {} admits {} which {} does not",
          provider,
          witness(&sets[p_index], &sets[c_index]).unwrap_or_default(),
          consumer
        ),
        Verdict::No => assert!(
          !contained,
          "the checker said no, but every value {provider} admits is admitted by {consumer}"
        ),
        // Shape spec §8's exact class promises a decision for every pair in it; an `unknown`
        // here is a column missing from the table, not a conservative answer.
        Verdict::Unknown => {
          panic!("the checker said unknown for an exact-class pair: {provider} against {consumer}")
        }
      }
      checked += 1;
    }
  }
  assert_eq!(checked, vocabulary.len() * vocabulary.len());
}

#[test]
fn the_universe_witnesses_every_difference_the_vocabulary_has() {
  // The exhaustive test's `no` assertions are only as strong as the universe: two shapes that
  // differ but that no candidate tells apart would make a wrong `no` look right. This is the
  // guard on that — every pair the checker calls `no` has a concrete witness.
  let vocabulary = vocabulary();
  let sets: Vec<Vec<bool>> = vocabulary.iter().map(admits).collect();
  for (p_index, provider) in vocabulary.iter().enumerate() {
    for (c_index, consumer) in vocabulary.iter().enumerate() {
      if verdict(provider, consumer) == Verdict::No {
        assert!(
          witness(&sets[p_index], &sets[c_index]).is_some(),
          "no witness in the value universe for {provider} against {consumer}"
        );
      }
    }
  }
}

// --- the generated pass --------------------------------------------------------------------------

/// A deliberately small, deliberately deterministic PRNG: the point is a reproducible corpus of
/// trees, not statistical quality, and a failing case has to be reproducible from the test alone.
struct Rng(u64);

impl Rng {
  fn next(&mut self) -> u64 {
    // Numerical Recipes' LCG constants.
    self.0 = self
      .0
      .wrapping_mul(6364136223846793005)
      .wrapping_add(1442695040888963407);
    self.0 >> 33
  }

  fn below(&mut self, bound: usize) -> usize {
    (self.next() % bound as u64) as usize
  }

  /// `true` one time in `n`.
  fn sometimes(&mut self, n: usize) -> bool {
    self.below(n) == 0
  }
}

/// A *pair* of shape trees generated in lockstep: the same structure on both sides, diverging at
/// individual nodes. Two independently random trees would almost never subsume one another and
/// would make the property vacuous; a provider shape and a consumer contract for the same
/// operation are the same tree with differences, which is what this generates.
fn generate_pair(rng: &mut Rng, depth: usize) -> (Value, Value) {
  let vocabulary = vocabulary();
  if depth == 0 {
    let provider = rng.below(vocabulary.len());
    let consumer = if rng.sometimes(3) {
      rng.below(vocabulary.len())
    } else {
      provider
    };
    return (vocabulary[provider].clone(), vocabulary[consumer].clone());
  }
  match rng.below(5) {
    0 => {
      let (p_a, c_a) = generate_pair(rng, depth - 1);
      let (p_b, c_b) = generate_pair(rng, depth - 1);
      (
        json!({ "shape": "object", "members": { "a": p_a, "b": p_b } }),
        json!({ "shape": "object", "members": { "a": c_a, "b": c_b } }),
      )
    }
    1 => {
      let (provider, consumer) = generate_pair(rng, depth - 1);
      (
        json!({ "shape": "object", "members": { "a": provider } }),
        json!({ "shape": "object", "members": { "a": consumer } }),
      )
    }
    2 => {
      let (provider, consumer) = value_pair(rng, depth - 1);
      let p_min = rng.below(3);
      let c_min = if rng.sometimes(3) { rng.below(3) } else { p_min };
      (
        json!({ "shape": "each-like", "min": p_min, "items": provider }),
        json!({ "shape": "each-like", "min": c_min, "items": consumer }),
      )
    }
    3 => {
      let (provider, consumer) = value_pair(rng, depth - 1);
      (
        json!({ "shape": "array", "entries": [ provider ] }),
        json!({ "shape": "array", "entries": [ consumer ] }),
      )
    }
    _ => {
      let (provider, consumer) = value_pair(rng, depth - 1);
      // One side optional and the other not is the RFC's "nullable column" in its presence form,
      // and it is the difference most worth generating often.
      let wrap = |shape: Value, wrapped: bool| {
        if wrapped {
          json!({ "shape": "optional", "of": shape })
        } else {
          shape
        }
      };
      let p_optional = !rng.sometimes(3);
      let c_optional = if rng.sometimes(2) { !p_optional } else { p_optional };
      (wrap(provider, p_optional), wrap(consumer, c_optional))
    }
  }
}

/// The same, restricted to shapes that may sit outside a slot — shape spec §5.1 forbids a
/// `⊥`-admitting shape as `each-like` items, an `array` entry, or the `of` of `optional`.
fn value_pair(rng: &mut Rng, depth: usize) -> (Value, Value) {
  for _ in 0..8 {
    let (provider, consumer) = generate_pair(rng, depth);
    if !admits_absence(&provider) && !admits_absence(&consumer) {
      return (provider, consumer);
    }
  }
  (
    json!({ "shape": "integer", "example": 1 }),
    json!({ "shape": "integer", "example": 1 }),
  )
}

fn admits_absence(shape: &Value) -> bool {
  matches!(
    shape.get("shape").and_then(Value::as_str),
    Some("optional") | Some("forbidden")
  )
}

#[test]
fn a_generated_yes_is_never_a_wrong_yes() {
  let mut rng = Rng(0x5EED);
  let mut yeses = 0;
  let mut nos = 0;
  let mut unknowns = 0;

  for _ in 0..400 {
    let (provider, consumer) = generate_pair(&mut rng, 3);
    let provider_admits = admits(&provider);
    let consumer_admits = admits(&consumer);
    match verdict(&provider, &consumer) {
      Verdict::Yes => {
        yeses += 1;
        assert!(
          subset(&provider_admits, &consumer_admits),
          "the checker said yes, but {} admits {} which {} does not",
          provider,
          witness(&provider_admits, &consumer_admits).unwrap_or_default(),
          consumer
        );
      }
      // A `no` on a nested tree may be correct without a witness in this fixed universe — the
      // difference can live at a depth no candidate value reaches — so this direction is asserted
      // only where a witness does exist to contradict it.
      Verdict::No => {
        nos += 1;
        assert!(
          !subset(&provider_admits, &consumer_admits)
            || witness(&provider_admits, &consumer_admits).is_none(),
          "the checker said no, but every value {provider} admits is admitted by {consumer}"
        );
      }
      Verdict::Unknown => unknowns += 1,
    }
  }

  // Not assertions about the checker so much as about the corpus: a run that decided almost
  // nothing, or decided everything one way, would make the property above vacuous.
  assert!(yeses > 50, "only {yeses} of 400 generated pairs decided yes");
  assert!(nos > 50, "only {nos} of 400 generated pairs decided no");
  assert!(
    unknowns * 4 < 400,
    "{unknowns} of 400 generated pairs went undecided, which is more degradation than a corpus \
     built entirely from the exact class should produce"
  );
}
