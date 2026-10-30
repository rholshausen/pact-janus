package io.pact.janus.sdk;

import java.util.List;
import java.util.Map;

/** The run's status was {@code failed}. Names each failed result that is not pending. */
public class VerificationFailedException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  private final transient VerificationResult result;
  private final transient List<Map<String, Object>> failures;

  VerificationFailedException(VerificationResult result) {
    this(result, result.results().stream()
        .filter(r -> "failed".equals(r.get("status")) && !Boolean.TRUE.equals(r.get("pending")))
        .toList());
  }

  private VerificationFailedException(VerificationResult result, List<Map<String, Object>> failures) {
    super(describe(failures));
    this.result = result;
    this.failures = failures;
  }

  public VerificationResult result() {
    return result;
  }

  /** The failed results that are not pending, as the engine sent them. */
  public List<Map<String, Object>> failures() {
    return failures;
  }

  private static String describe(List<Map<String, Object>> failures) {
    StringBuilder text = new StringBuilder("The provider verification failed");
    for (Map<String, Object> failure : failures) {
      text.append("\n  ✗ ").append(explain(failure).replace("\n", "\n  "));
    }
    return text.toString();
  }

  /** One result as text: its interaction and variant, then each mismatch, or its error. */
  static String explain(Map<String, Object> result) {
    Object interaction = result.get("interaction");
    Object description = interaction instanceof Map<?, ?> ref ? ref.get("description") : interaction;
    StringBuilder text = new StringBuilder().append(description).append(" (").append(result.get("variant")).append(')');
    if (result.get("mismatches") instanceof List<?> mismatches) {
      for (Object mismatch : mismatches) {
        Map<?, ?> m = mismatch instanceof Map<?, ?> map ? map : Map.of();
        text.append("\n    ").append(m.get("path")).append(' ').append(m.get("message"));
      }
    }
    if (result.get("error") instanceof Map<?, ?> error) {
      text.append("\n    ").append(error.get("code")).append(": ").append(error.get("message"));
    }
    return text.toString();
  }
}
