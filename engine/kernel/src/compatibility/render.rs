//! The `can-i-deploy` page (engine-protocol spec §8.6): design 2.8 §6.4's finding block, under a
//! decision, with the verification line the RFC asks for beside it.
//!
//! The block itself is not re-worded here. [`crate::subsumption::render`]'s own pieces produce
//! every finding line, which is what keeps the RFC's sketch reproducible verbatim — the header
//! line and the two `provider may produce` / `consumer has only tested` lines are the
//! specification's, and this module only decides which findings appear where.
//!
//! One marker is this page's own: `~` for a finding an exemption silenced. Design 2.8 §6.5 fixes
//! three (none, `?`, `!`) for the three severities, and an exempted finding is not a fourth
//! severity — it is a finding with a decision over it, which is exactly the distinction the
//! marker has to carry.

use super::decide::{CompatibilityReport, Decision, Disposition, ExemptionStatus, PairResult, ReasonAction};
use crate::display::{Kind, Marked};
use crate::subsumption::{Severity, finding_lines, header};

/// Render a compatibility report as the text a person reads.
pub fn render(report: &CompatibilityReport) -> String {
  render_marked(report).text()
}

/// [`render`], with its segments (ADR 0054): each marker by what it says, every decision by its
/// outcome, and an exempted finding, the policy and the notes quiet behind the rest.
pub fn render_marked(report: &CompatibilityReport) -> Marked {
  let mut lines = Vec::new();
  for pair in &report.pairs {
    lines.extend(render_pair(pair));
    lines.push(Marked::new());
  }
  lines.push(verdict_line(report));
  Marked::join(lines, "\n")
}

/// An outcome word's kind: a pass, a failure, or, for anything else, something to look at.
fn outcome(word: &str) -> Kind {
  match word {
    "pass" | "verified" => Kind::Pass,
    "block" | "failed" => Kind::Fail,
    _ => Kind::Warn,
  }
}

fn render_pair(pair: &PairResult) -> Vec<Marked> {
  let live: Vec<_> = pair
    .findings
    .iter()
    .filter(|entry| entry.disposition == Disposition::Live)
    .collect();
  let has_findings = live
    .iter()
    .any(|entry| entry.finding.severity == Severity::Finding);
  let has_reviews = live
    .iter()
    .any(|entry| entry.finding.severity == Severity::Review);

  let mut lines = vec![header(
    &pair.consumer.name,
    &pair.provider.name,
    has_findings,
    has_reviews,
    pair.subsumption.matched > 0,
  )];

  for entry in &live {
    lines.extend(finding_lines(
      &entry.interaction.description,
      &entry.finding,
      None,
    ));
  }
  for entry in pair
    .findings
    .iter()
    .filter(|entry| entry.disposition == Disposition::Exempt)
  {
    // A finding an exemption silenced is said, and said quietly: every line of it.
    lines.extend(
      finding_lines(&entry.interaction.description, &entry.finding, Some("~ "))
        .into_iter()
        .map(|line| line.all_as(Kind::Quiet)),
    );
    if let Some(exemption) = &entry.exemption {
      lines.push(Marked::of(
        Kind::Quiet,
        match &exemption.expires {
          Some(expires) => format!("    exempted until {expires}: {}", exemption.reason),
          None => format!("    exempted: {}", exemption.reason),
        },
      ));
    }
  }
  let (status, rest) = verification_phrase(pair);
  let mut verification = Marked::plain("  verification: ");
  verification
    .text_as(outcome(pair.verification.status.as_str()), status)
    .text_plain(rest);
  lines.push(verification);
  for reason in &pair.reasons {
    let line = match reason.action {
      ReasonAction::Block => reason_line(Kind::Fail, "✗", &reason.message),
      ReasonAction::Warn => reason_line(Kind::Warn, "!", &reason.message),
      ReasonAction::Note => Marked::of(Kind::Quiet, format!("  - {}", reason.message)),
    };
    lines.push(line);
  }
  for exemption in &pair.exemptions {
    if exemption.status == ExemptionStatus::Lapsed {
      lines.push(reason_line(
        Kind::Warn,
        "!",
        &format!(
          "lapsed exemption ({}): {}",
          exemption.exemption.expires.as_deref().unwrap_or("no expiry"),
          exemption.exemption.reason
        ),
      ));
    }
  }
  let mut decision = Marked::plain("  ");
  decision.text_as(
    outcome(pair.decision.as_str()),
    format!(
      "=> {}: {} -> {}",
      pair.decision.as_str().to_uppercase(),
      pair.consumer.name,
      pair.provider.name
    ),
  );
  lines.push(decision);
  lines
}

/// `  ✗ <message>`: a reason, its marker marked.
fn reason_line(kind: Kind, marker: &str, message: &str) -> Marked {
  let mut line = Marked::plain("  ");
  line.text_as(kind, marker).text_plain(format!(" {message}"));
  line
}

/// The verification half of the page, in one line — the RFC's `can-i-deploy` combines the two,
/// and a page that printed subsumption findings without saying whether the contract was ever
/// replayed would be answering half the question. Its status phrase, then the rest.
fn verification_phrase(pair: &PairResult) -> (String, String) {
  let status = pair.verification.status.as_str();
  let phrase = match status {
    "unknown" => "no result supplied".to_string(),
    other => other.to_string(),
  };
  let mut text = String::new();
  if let Some(variants) = &pair.verification.variants {
    let total = variants["total"].as_u64().unwrap_or(0);
    let verified = variants["verified"].as_u64().unwrap_or(0);
    text.push_str(&format!(
      " ({verified} of {})",
      super::count(total, "variant", "variants")
    ));
  } else if pair.verification.runs > 1 {
    text.push_str(&format!(" (across {} runs)", pair.verification.runs));
  }
  if pair.verification.filtered == Some(true) {
    text.push_str(", filtered");
  }
  (phrase, text)
}

fn verdict_line(report: &CompatibilityReport) -> Marked {
  let word = report.decision.as_str().to_uppercase();
  let kind = outcome(report.decision.as_str());
  if report.decision == Decision::Pass && report.summary.pairs == 0 {
    let mut line = Marked::of(kind, word);
    line
      .text_plain(": nothing to decide")
      .text_as(Kind::Quiet, " — no pairs were given");
    return line;
  }
  let summary = &report.summary;
  let mut policy = format!(
    " — policy on-finding {}, on-review {}",
    report.policy.on_finding.as_str(),
    report.policy.on_review.as_str()
  );
  if let Some(as_of) = &report.as_of {
    policy.push_str(&format!(", exemptions as of {as_of}"));
  }
  let mut line = Marked::of(kind, word);
  line
    .text_plain(format!(
      ": {} blocked, {} warned, {} passed",
      super::count(summary.blocked as u64, "pair", "pairs"),
      summary.warned,
      summary.passed
    ))
    .text_as(Kind::Quiet, policy);
  line
}
