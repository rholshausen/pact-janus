package io.pact.janus.sdk;

import io.pact.janus.bindings.protocol.v1.InteractionResult;
import java.util.List;

/**
 * {@code finalise} wrote no contract: the engine withheld it (an interaction did not verify on a
 * variant it required), or an {@code execute} in the session failed and a contract would claim a
 * variant the consumer's own test failed on. The message names every such interaction and variant,
 * with the engine's mismatches.
 */
public class ContractWithheldException extends AssertionError {
  private static final long serialVersionUID = 1L;
  private final transient List<InteractionResult> results;
  private final transient List<String> failedInteractions;
  private final boolean engineWithheld;

  ContractWithheldException(String message, List<InteractionResult> results, List<String> failedInteractions,
      boolean engineWithheld) {
    super(message);
    this.results = results == null ? List.of() : List.copyOf(results);
    this.failedInteractions = List.copyOf(failedInteractions);
    this.engineWithheld = engineWithheld;
  }

  /** {@code results} exactly as {@code consumer-session/finalise} returned them. */
  public List<InteractionResult> results() {
    return results;
  }

  /** The descriptions of the interactions whose {@code execute} failed. */
  public List<String> failedInteractions() {
    return failedInteractions;
  }

  /** Whether the engine itself returned no contract (as opposed to the SDK withholding one it returned). */
  public boolean engineWithheld() {
    return engineWithheld;
  }
}
