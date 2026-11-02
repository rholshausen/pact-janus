package io.pact.janus.sdk;

import io.pact.janus.bindings.protocol.v1.InteractionResult;
import io.pact.janus.bindings.protocol.v1.RefusedInteraction;
import java.util.List;

/**
 * {@code finalise} wrote no contract, because the engine withheld it: an interaction did not verify
 * on a variant it required (its request did not match, nothing exercised it, or its test failed), or
 * the engine refused an interaction the suite added (ADR 0064). The message names every such
 * interaction and variant, with the engine's mismatches and the test's reason.
 */
public class ContractWithheldException extends AssertionError {
  private static final long serialVersionUID = 1L;
  private final transient List<InteractionResult> results;
  private final transient List<RefusedInteraction> refused;
  private final List<String> interactions;

  ContractWithheldException(String message, List<InteractionResult> results, List<RefusedInteraction> refused,
      List<String> interactions) {
    super(message);
    this.results = results == null ? List.of() : List.copyOf(results);
    this.refused = refused == null ? List.of() : List.copyOf(refused);
    this.interactions = List.copyOf(interactions);
  }

  /** {@code results} exactly as {@code consumer-session/finalise} returned them. */
  public List<InteractionResult> results() {
    return results;
  }

  /** The interactions the engine refused when the suite added them, as {@code finalise} returned them. */
  public List<RefusedInteraction> refused() {
    return refused;
  }

  /** The descriptions of the interactions that did not verify, then of the ones the engine refused. */
  public List<String> interactions() {
    return interactions;
  }
}
