//! The text rendering (design 2.8 §6.4): findings grouped by interaction, then by `path`, one
//! line per side, in the RFC's own style.
//!
//! This is a second phrase table, deliberately separate from [`super::phrases`]. The spec's worked
//! example shows both for the same finding — a cardinality summary reads "0 to unbounded elements"
//! while its rendered line reads "provider may produce an empty list" — so the renderer reads the
//! *grammar* of a summary (§4.4 fixes it, and both tables are documented together in spec §6.5)
//! rather than printing it verbatim. That is also what lets this run over a report read back from
//! disk, which is what task 7.4 does when it combines this block with verification results.
//!
//! Three pieces are exported to [`crate::compatibility`] rather than kept private, because task
//! 7.4's `can-i-deploy` page is the same block under a decision: it needs the header line, the
//! per-finding body, and the not-published line, composed differently — around exemptions, which
//! the checker knows nothing about. Composing is all it does; the words are this module's.

use super::report::{Finding, NOT_PUBLISHED, Severity, SubsumptionReport};

/// Render a report as the text a person reads.
pub fn render(report: &SubsumptionReport) -> String {
  let has_findings = report.interactions.iter().any(|interaction| {
    interaction
      .findings
      .iter()
      .any(|f| f.severity == Severity::Finding)
  });
  let has_reviews = report.interactions.iter().any(|interaction| {
    interaction
      .findings
      .iter()
      .any(|f| f.severity == Severity::Review)
  });
  let any_matched = report.interactions.iter().any(|interaction| interaction.matched);

  let mut lines = vec![header(
    &report.consumer.name,
    &report.provider.name,
    has_findings,
    has_reviews,
    any_matched,
  )];
  for interaction in &report.interactions {
    if interaction.verdict == NOT_PUBLISHED {
      lines.push(not_published_line(
        &interaction.description,
        interaction.reason.as_deref(),
      ));
      continue;
    }
    for finding in &interaction.findings {
      lines.extend(finding_lines(&interaction.description, finding, None));
    }
  }
  lines.join("\n")
}

/// The header line (§6.4): the verdict, the two parties, and nothing else.
///
/// Two of its four forms are this checker's choice rather than the specification's, and are the
/// ones §6.4 left to task 7.4: the `?` for a report with no decided finding but something to
/// review, and the line for a pair where the provider published nothing at all (§6.3 fixes that
/// report state, not its rendering). `?` is the per-finding marker the spec *does* fix, so a
/// header and the lines under it cannot contradict each other.
pub(crate) fn header(
  consumer: &str,
  provider: &str,
  has_findings: bool,
  has_reviews: bool,
  any_matched: bool,
) -> String {
  if has_findings {
    format!("✗ {consumer} is not compatible with {provider}")
  } else if has_reviews {
    format!("? {consumer} needs review against {provider}")
  } else if any_matched {
    format!("✓ {consumer} is compatible with {provider}")
  } else {
    // Not "no shapes published": a provider that published shapes nobody's interactions match —
    // a derived shape named by operationId, say (spike 7.3 §2) — lands here too, and that line
    // read like reassurance to exactly the team that most needed to hear nothing was checked.
    format!("? {consumer} was not checked against {provider}: no published shape matched any interaction")
  }
}

pub(crate) fn not_published_line(description: &str, reason: Option<&str>) -> String {
  match reason {
    Some(reason) => format!("  interaction '{description}': not checked — {reason}"),
    None => {
      format!("  interaction '{description}': no published shape matches it by description or operation")
    }
  }
}

/// One finding, as the two-or-one-line block §6.4 fixes: its header line, then a line per side.
///
/// `marker` overrides the severity marker — what plan task 7.4's page needs for a finding an
/// exemption has silenced, which is neither a live finding nor a different severity. Everything
/// else here is the specification's own grammar, read off the `summary` pair (§6.5), which is what
/// lets this run over a report read back from a file.
pub(crate) fn finding_lines(description: &str, finding: &Finding, marker: Option<&str>) -> Vec<String> {
  let marker = marker.unwrap_or(match finding.severity {
    Severity::Finding => "",
    Severity::Review => "? ",
    Severity::Advisory => "! ",
  });
  let mut lines = vec![format!(
    "  {marker}interaction '{description}', {}:",
    locate(&finding.path)
  )];
  let provider_summary = finding
    .provider
    .as_ref()
    .map(|side| side.summary.as_str())
    .unwrap_or_default();
  let consumer_summary = finding
    .consumer
    .as_ref()
    .map(|side| side.summary.as_str())
    .unwrap_or_default();
  match finding.kind.as_str() {
    "unreviewable" => lines.push(format!(
      "    {} — review manually",
      uncomparable(provider_summary, consumer_summary)
    )),
    kind => {
      lines.push(format!(
        "    provider may produce{}",
        side_phrase(kind, provider_summary)
      ));
      lines.push(format!(
        "    consumer has only tested{}",
        side_phrase(kind, consumer_summary)
      ));
    }
  }
  for exclusion in &finding.excluded_by {
    if let Some(reason) = exclusion.get("reason").and_then(|reason| reason.as_str()) {
      lines.push(format!("    not exercised together: {reason}"));
    }
  }
  lines
}

/// `response.body.payment@card.last4` -> `response body $.payment.last4 (card)`: the same address
/// as `path` (§4.4), "rendered with a leading `$.` per part/slot convention rather than shown raw,
/// because this text is read by people who know JSONPath and not necessarily this project's
/// dimension-id grammar" (§6.4).
fn locate(path: &str) -> String {
  let mut segments = path.split('.');
  let part = segments.next().unwrap_or_default();
  let slot = segments.next().unwrap_or_default();
  let rest: Vec<&str> = segments.collect();

  let mut alternatives = Vec::new();
  let stripped: Vec<String> = rest
    .iter()
    .map(|segment| match segment.split_once('@') {
      Some((before, after)) => {
        // `@<alternative>` sits on the segment where the alternative is entered; the name goes to
        // the trailing parenthesis, where a reader expects a union's branch.
        let (alternative, tail) = match after.find(['[', '{']) {
          Some(index) => after.split_at(index),
          None => (after, ""),
        };
        alternatives.push(alternative.to_string());
        format!("{before}{tail}")
      }
      None => (*segment).to_string(),
    })
    .collect();

  let address = if stripped.is_empty() {
    "$".to_string()
  } else {
    format!("$.{}", stripped.join("."))
  };
  let suffix = if alternatives.is_empty() {
    String::new()
  } else {
    format!(" ({})", alternatives.join(", "))
  };
  format!("{part} {slot} {address}{suffix}")
}

/// The per-kind half-line that follows "provider may produce" / "consumer has only tested".
fn side_phrase(kind: &str, summary: &str) -> String {
  match kind {
    // "one of 'A' | 'B'" reads as a list once the sentence already says "may produce".
    "wider-values" => match summary.strip_prefix("one of ") {
      Some(options) => format!(": {options}"),
      None => format!(" {summary}"),
    },
    "wider-cardinality" => format!(" {}", cardinality(summary)),
    _ => format!(" {summary}"),
  }
}

/// A cardinality summary — §4.4's `"<min> to <max|unbounded> <unit>"` — as the RFC's own prose.
fn cardinality(summary: &str) -> String {
  let entries = summary.ends_with("entries");
  let (unit, empty) = if entries {
    ("entry", "an empty map")
  } else {
    ("item", "an empty list")
  };
  let Some((min, rest)) = summary.split_once(" to ") else {
    return summary.to_string();
  };
  let min: u64 = match min.parse() {
    Ok(min) => min,
    Err(_) => return summary.to_string(),
  };
  let max = rest.split_whitespace().next().unwrap_or("unbounded");
  match (min, max.parse::<u64>().ok()) {
    (0, None) => empty.to_string(),
    (1, None) => format!("at least one {unit}"),
    (min, None) => format!("at least {min} {unit}s"),
    (min, Some(max)) if min == max => format!("exactly {min} {unit}{}", plural(min)),
    (0, Some(max)) => format!("{empty}, or up to {max} {unit}{}", plural(max)),
    (min, Some(max)) => format!("between {min} and {max} {unit}s"),
  }
}

fn plural(count: u64) -> &'static str {
  if count == 1 { "" } else { "s" }
}

/// The one-line form a `review`-severity finding renders as. Two regexes get the spec's own
/// phrasing; anything else falls back to the two summaries, which always say something true.
fn uncomparable(provider: &str, consumer: &str) -> String {
  const PATTERN: &str = "strings matching ";
  match (provider.strip_prefix(PATTERN), consumer.strip_prefix(PATTERN)) {
    (Some(provider), Some(consumer)) => {
      format!("provider pattern {provider} cannot be compared against consumer pattern {consumer}")
    }
    _ => format!("provider {provider} cannot be compared against consumer {consumer}"),
  }
}
