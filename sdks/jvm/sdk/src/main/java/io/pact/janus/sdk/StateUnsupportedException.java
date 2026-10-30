package io.pact.janus.sdk;

/**
 * Thrown by a state handler that cannot put the provider in its state: the variant is
 * {@code state-unavailable} (lifecycle-hooks spec §5.5), a finding about the contract rather than a
 * broken setup.
 */
public class StateUnsupportedException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  public StateUnsupportedException(String reason) {
    super(reason);
  }
}
