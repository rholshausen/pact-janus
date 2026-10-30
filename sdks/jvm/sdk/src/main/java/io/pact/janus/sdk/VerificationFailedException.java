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
      Object interaction = failure.get("interaction");
      Object description = interaction instanceof Map<?, ?> ref ? ref.get("description") : interaction;
      text.append("\n  ✗ ").append(description).append(" (").append(failure.get("variant")).append(')');
      if (failure.get("mismatches") instanceof List<?> mismatches) {
        for (Object mismatch : mismatches) {
          Map<?, ?> m = mismatch instanceof Map<?, ?> map ? map : Map.of();
          text.append("\n      ").append(m.get("path")).append(' ').append(m.get("message"));
        }
      }
    }
    return text.toString();
  }
}
