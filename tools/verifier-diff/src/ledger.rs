//! The disagreement ledger (`corpora/verifier/disagreements.json`), and what a run is judged by.
//!
//! "Exactly as today" is a number: the verdicts the two verifiers disagree on. Each known one is in
//! the ledger with its reason — a staged-plan item that closes it (`S1.6`) or the ADR that makes it
//! deliberate (`ADR 0007`) — and the harness passes when the disagreements it finds are exactly the
//! ledger's. A new one fails it, and so does one that has gone away: an entry nothing disagrees on
//! any more is a gap closed without anyone crossing it off.

use crate::fixture::RunVerdict;
use crate::run::{CaseResult, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
  #[serde(rename = "$comment", default, skip_serializing_if = "String::is_empty")]
  pub comment: String,
  pub disagreements: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
  pub case: String,
  /// `"run"`, or `"<pact>/<interaction>"`.
  pub key: String,
  pub today: Verdict,
  pub janus: Verdict,
  pub reason: String,
}

impl Ledger {
  /// Every entry's reason starts with what accounts for it: a plan item or an ADR.
  pub fn check(&self) -> Result<(), String> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in &self.disagreements {
      if !names_its_reason(&entry.reason) {
        problems.push(format!(
          "{} {}: the reason must start with a staged-plan item (S1.6) or an ADR (ADR 0007), not '{}'",
          entry.case, entry.key, entry.reason
        ));
      }
      if entry.today == entry.janus {
        problems.push(format!(
          "{} {}: both verdicts are {}; that is an agreement",
          entry.case, entry.key, entry.today
        ));
      }
      if !seen.insert((&entry.case, &entry.key)) {
        problems.push(format!("{} {}: listed twice", entry.case, entry.key));
      }
    }
    if problems.is_empty() {
      Ok(())
    } else {
      Err(problems.join("\n"))
    }
  }

  fn find(&self, case: &str, key: &str) -> Option<&Entry> {
    self
      .disagreements
      .iter()
      .find(|entry| entry.case == case && entry.key == key)
  }
}

fn names_its_reason(reason: &str) -> bool {
  let plan_item = reason
    .strip_prefix('S')
    .and_then(|rest| rest.split_once('.'))
    .is_some_and(|(stage, rest)| {
      !stage.is_empty()
        && stage.chars().all(|c| c.is_ascii_alphanumeric())
        && rest.chars().next().is_some_and(|c| c.is_ascii_digit())
    });
  let adr = reason
    .strip_prefix("ADR ")
    .is_some_and(|rest| rest.len() >= 4 && rest[..4].chars().all(|c| c.is_ascii_digit()));
  plan_item || adr
}

/// What a run found, against the ledger.
#[derive(Debug, Default, Serialize)]
pub struct Judgement {
  pub cases: usize,
  pub compared: usize,
  pub agreed: usize,
  pub known: Vec<Entry>,
  /// Disagreements the ledger does not list.
  pub unexpected: Vec<Entry>,
  /// Ledger entries for cases that ran, which nothing disagreed on.
  pub stale: Vec<Entry>,
  /// Both verifiers failed to give a verdict: nothing was compared.
  pub unjudged: Vec<String>,
  /// Today's verifier disagrees with the verdict the fixture's source records, for the run or for
  /// what a broker case publishes: the fixture, not Janus, is wrong.
  pub fixture_problems: Vec<String>,
}

impl Judgement {
  pub fn passed(&self) -> bool {
    self.unexpected.is_empty()
      && self.stale.is_empty()
      && self.unjudged.is_empty()
      && self.fixture_problems.is_empty()
  }
}

pub fn judge(results: &[CaseResult], ledger: &Ledger) -> Judgement {
  let mut judgement = Judgement {
    cases: results.len(),
    ..Judgement::default()
  };
  let mut matched = BTreeSet::new();
  for result in results {
    for compared in &result.compared {
      judgement.compared += 1;
      if compared.today == compared.janus {
        if compared.today == Verdict::Error {
          judgement.unjudged.push(format!(
            "{} {}: {}",
            result.id,
            compared.key,
            compared
              .today_detail
              .as_deref()
              .unwrap_or("neither verifier gave a verdict")
          ));
        } else {
          judgement.agreed += 1;
        }
        continue;
      }
      let found = Entry {
        case: result.id.clone(),
        key: compared.key.clone(),
        today: compared.today,
        janus: compared.janus,
        reason: String::new(),
      };
      match ledger.find(&result.id, &compared.key) {
        Some(entry) if entry.today == compared.today && entry.janus == compared.janus => {
          matched.insert((entry.case.clone(), entry.key.clone()));
          judgement.known.push(entry.clone());
        }
        _ => judgement.unexpected.push(Entry {
          reason: format!(
            "today: {}; janus: {}",
            compared.today_detail.as_deref().unwrap_or("-"),
            compared.janus_detail.as_deref().unwrap_or("-")
          ),
          ..found
        }),
      }
    }
    if let (Some(expected), Some(run)) = (result.expected, result.compared.first())
      && run.today != Verdict::from(expected)
    {
      let expected = match expected {
        RunVerdict::Pass => "pass",
        RunVerdict::Fail => "fail",
      };
      judgement.fixture_problems.push(format!(
        "{}: the source says the run should {expected}, and today's verifier says {}{}",
        result.id,
        run.today,
        run
          .today_detail
          .as_deref()
          .map(|detail| format!(" ({detail})"))
          .unwrap_or_default()
      ));
    }
    for (key, expected) in &result.expected_published {
      if let Some(compared) = result.compared.iter().find(|compared| &compared.key == key)
        && compared.today != *expected
      {
        judgement.fixture_problems.push(format!(
          "{} {key}: the source says the run publishes {expected}, and today's verifier published {}",
          result.id, compared.today
        ));
      }
    }
  }
  let ran: BTreeSet<&str> = results.iter().map(|result| result.id.as_str()).collect();
  judgement.stale = ledger
    .disagreements
    .iter()
    .filter(|entry| {
      ran.contains(entry.case.as_str()) && !matched.contains(&(entry.case.clone(), entry.key.clone()))
    })
    .cloned()
    .collect();
  judgement
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::run::Compared;
  use rstest::rstest;

  fn result(id: &str, today: Verdict, janus: Verdict) -> CaseResult {
    CaseResult {
      id: id.to_string(),
      millis: 0,
      compared: vec![Compared {
        key: "run".to_string(),
        description: "the whole run".to_string(),
        today,
        janus,
        today_detail: None,
        janus_detail: None,
      }],
      expected: None,
      expected_published: Default::default(),
      broker: None,
    }
  }

  fn entry(case: &str, today: Verdict, janus: Verdict) -> Entry {
    Entry {
      case: case.to_string(),
      key: "run".to_string(),
      today,
      janus,
      reason: "S1.6: XML bodies".to_string(),
    }
  }

  #[rstest]
  #[case("S1.6: XML bodies", true)]
  #[case("S0.13", true)]
  #[case("SA.3: the AI layer", true)]
  #[case("ADR 0007: shapes are must-ignore", true)]
  #[case("because", false)]
  #[case("S: nothing", false)]
  #[case("ADR 7", false)]
  fn a_reason_names_what_accounts_for_it(#[case] reason: &str, #[case] names: bool) {
    assert_eq!(names_its_reason(reason), names);
  }

  #[test]
  fn a_listed_disagreement_is_known_and_an_unlisted_one_fails_the_run() {
    let ledger = Ledger {
      disagreements: vec![entry("a", Verdict::Fail, Verdict::Pass)],
      ..Ledger::default()
    };
    let judgement = judge(
      &[
        result("a", Verdict::Fail, Verdict::Pass),
        result("b", Verdict::Pass, Verdict::Fail),
        result("c", Verdict::Pass, Verdict::Pass),
      ],
      &ledger,
    );
    assert_eq!(judgement.known.len(), 1);
    assert_eq!(judgement.unexpected.len(), 1);
    assert_eq!(judgement.unexpected[0].case, "b");
    assert_eq!(judgement.agreed, 1);
    assert!(!judgement.passed());
  }

  #[test]
  fn an_entry_nothing_disagrees_on_any_more_is_stale() {
    let ledger = Ledger {
      disagreements: vec![
        entry("a", Verdict::Fail, Verdict::Pass),
        entry("not-run", Verdict::Fail, Verdict::Pass),
      ],
      ..Ledger::default()
    };
    let judgement = judge(&[result("a", Verdict::Pass, Verdict::Pass)], &ledger);
    assert_eq!(judgement.stale.len(), 1, "only for a case that ran");
    assert!(!judgement.passed());
  }

  #[test]
  fn a_disagreement_that_changed_direction_is_not_the_listed_one() {
    let ledger = Ledger {
      disagreements: vec![entry("a", Verdict::Fail, Verdict::Pass)],
      ..Ledger::default()
    };
    let judgement = judge(&[result("a", Verdict::Fail, Verdict::Error)], &ledger);
    assert_eq!(judgement.unexpected.len(), 1);
    assert_eq!(judgement.stale.len(), 1);
  }

  #[test]
  fn a_ledger_checks_its_own_entries() {
    let mut bad = entry("a", Verdict::Pass, Verdict::Pass);
    bad.reason = "it just does".to_string();
    let ledger = Ledger {
      disagreements: vec![bad],
      ..Ledger::default()
    };
    let problems = ledger.check().unwrap_err();
    assert!(problems.contains("must start with"), "{problems}");
    assert!(problems.contains("that is an agreement"), "{problems}");
  }
}
