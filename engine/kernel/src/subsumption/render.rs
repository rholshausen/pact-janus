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
//! Two things here are this checker's choice rather than the spec's, and are marked as such below:
//! the header line for a report with no `finding`-severity result (§6.4 leaves the `review`-only
//! marker to task 7.4), and the line that names an interaction the provider published nothing for
//! (§6.3 fixes the report state, not its rendering).

use super::report::{NOT_PUBLISHED, Severity, SubsumptionReport};

/// Render a report as the text a person reads.
pub fn render(report: &SubsumptionReport) -> String {
  let consumer = &report.consumer.name;
  let provider = &report.provider.name;
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

  let mut lines = Vec::new();
  lines.push(if has_findings {
    format!("✗ {consumer} is not compatible with {provider}")
  } else if has_reviews {
    // §6.4 leaves the `review`-only marker to task 7.4; `?` matches the per-finding marker the
    // spec does fix, so the two do not contradict each other when 7.4 settles it.
    format!("? {consumer} needs review against {provider}")
  } else if any_matched {
    format!("✓ {consumer} is compatible with {provider}")
  } else {
    format!("? {consumer} was not checked against {provider}: no shapes published")
  });

  for interaction in &report.interactions {
    if interaction.verdict == NOT_PUBLISHED {
      lines.push(format!(
        "  interaction '{}': the provider has published no shape for it",
        interaction.description
      ));
      continue;
    }
    for finding in &interaction.findings {
      let marker = match finding.severity {
        Severity::Finding => "",
        Severity::Review => "? ",
        Severity::Advisory => "! ",
      };
      lines.push(format!(
        "  {marker}interaction '{}', {}:",
        interaction.description,
        locate(&finding.path)
      ));
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
    }
  }
  lines.join("\n")
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
