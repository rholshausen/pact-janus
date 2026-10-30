package io.pact.janus.sdk.engine;

import java.io.IOException;

/**
 * The SDK could not start {@code janus-engine}. Its message is the reason, then each way to get an
 * engine. It carries no stack trace and no cause: where the SDK noticed is no help to a reader, and
 * the reason already says what the operating system answered.
 */
public final class EngineNotStartedException extends IOException {
  EngineNotStartedException(String message) {
    super(message, null);
  }

  @Override
  public synchronized Throwable fillInStackTrace() {
    return this;
  }
}
