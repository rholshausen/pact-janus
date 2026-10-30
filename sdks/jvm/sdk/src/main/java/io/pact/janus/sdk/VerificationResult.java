package io.pact.janus.sdk;

import java.util.List;
import java.util.Map;

/**
 * What {@link Verifier#verify()} returns, and what its failure carries (behavioural spec
 * {@code verify}): the summary's {@code status} — {@code verified}, {@code filtered} (not a pass: not
 * every variant was replayed) or {@code failed} — the terminal event's summary document, every
 * {@code verification/interaction-result} payload, in order, each as the engine sent it, and every
 * {@code verification/warning} payload: what the run noticed and did not fail on, such as a provider
 * state no handler sets up (ADR 0058). Each warning is also written to standard error.
 */
public record VerificationResult(
    String status, Map<String, Object> summary, List<Map<String, Object>> results, List<Map<String, Object>> warnings) {
  public VerificationResult {
    summary = Map.copyOf(summary);
    results = List.copyOf(results);
    warnings = List.copyOf(warnings);
  }

  /** A result with no warnings. */
  public VerificationResult(String status, Map<String, Object> summary, List<Map<String, Object>> results) {
    this(status, summary, results, List.of());
  }
}
