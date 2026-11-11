package io.pact.janus.sdk;

import java.util.Objects;

/**
 * {@link VariantRun} for a message: one selected variant of a message interaction, which
 * {@link Janus#variants(MessageInteraction)} answers and {@link Janus#run(MessageVariantRun, MessageTest)}
 * runs. A type of its own, so a Groovy closure passed to {@code run} is read as the one kind of test the
 * variant can run.
 */
public final class MessageVariantRun {
  private final Janus.Execution execution;
  private final Variant variant;

  MessageVariantRun(Janus.Execution execution, Variant variant) {
    this.execution = Objects.requireNonNull(execution, "execution");
    this.variant = Objects.requireNonNull(variant, "variant");
  }

  Janus.Execution execution() {
    return execution;
  }

  /** The variant, exactly as the engine selected it. */
  public Variant variant() {
    return variant;
  }

  /** {@code label [id]}, or the id alone when the label is the id: {@code base}. */
  @Override
  public String toString() {
    return variant.displayName();
  }
}
