package io.pact.janus.sdk;

import java.util.Objects;

/**
 * One selected variant of an interaction already added to the session, for a framework that runs a
 * test once per row of data, as Spock's {@code where:} does: {@link Janus#variants(Interaction)} answers
 * one per variant, and {@link Janus#run(VariantRun, VariantTest)} runs it. Its {@link #toString()} is
 * the variant's {@link Variant#displayName()}, which names the row.
 */
public final class VariantRun {
  private final Janus.Execution execution;
  private final Variant variant;

  VariantRun(Janus.Execution execution, Variant variant) {
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
