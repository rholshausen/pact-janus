package io.pact.janus.facade.internal;

/** What the facade refuses by name, with what to write instead (ADR 0070 decisions 3 and 6). Not API. */
public final class Refused extends UnsupportedOperationException {
  private static final long serialVersionUID = 1L;

  public Refused(String message) {
    super(message);
  }
}
