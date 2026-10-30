package io.pact.janus.sdk;

import java.util.List;
import java.util.Map;

/**
 * What {@link Verifier#verify()} returns, and what its failure carries (behavioural spec
 * {@code verify}): the summary's {@code status} — {@code verified}, {@code filtered} (not a pass: not
 * every variant was replayed) or {@code failed} — the terminal event's summary document, and every
 * {@code verification/interaction-result} payload, in order, each as the engine sent it.
 */
public record VerificationResult(String status, Map<String, Object> summary, List<Map<String, Object>> results) {
  public VerificationResult {
    summary = Map.copyOf(summary);
    results = List.copyOf(results);
  }
}
