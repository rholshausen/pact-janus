package io.pact.janus.sdk;

import java.util.List;

/**
 * {@code execute} failed: the closure threw on one or more variants. Thrown once, after every
 * selected variant has run, naming each failed variant by label and id with its cause (each cause
 * is also attached as suppressed, the first as the cause).
 */
public class ExecuteFailedException extends AssertionError {
  private static final long serialVersionUID = 1L;
  private final String interaction;
  private final transient List<VariantFailure> failures;
  private final int variantCount;

  ExecuteFailedException(String interaction, List<VariantFailure> failures, int variantCount) {
    super(describe(interaction, failures, variantCount), failures.get(0).cause());
    this.interaction = interaction;
    this.failures = List.copyOf(failures);
    this.variantCount = variantCount;
    for (VariantFailure f : failures.subList(1, failures.size())) {
      addSuppressed(f.cause());
    }
  }

  /** The interaction's description. */
  public String interaction() {
    return interaction;
  }

  /** The variants that failed, in run order. */
  public List<VariantFailure> failures() {
    return failures;
  }

  /** How many variants ran. */
  public int variantCount() {
    return variantCount;
  }

  private static String describe(String interaction, List<VariantFailure> failures, int variantCount) {
    StringBuilder text = new StringBuilder("'").append(interaction).append("' failed on ")
        .append(failures.size()).append(" of ").append(variantCount).append(" variant")
        .append(variantCount == 1 ? "" : "s").append(':');
    for (VariantFailure f : failures) {
      text.append("\n  - ").append(f.variant().label()).append(" [").append(f.variant().id()).append("]: ")
          .append(f.cause());
    }
    return text.toString();
  }
}
