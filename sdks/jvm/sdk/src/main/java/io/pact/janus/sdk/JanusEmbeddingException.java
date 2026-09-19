package io.pact.janus.sdk;

/**
 * The pipe to the engine failed — the engine could not be started, exited, wrote something that is
 * not a frame, or answered with a pipe-level error — as opposed to an operation failing with a
 * structured error ({@link JanusEngineException}).
 */
public class JanusEmbeddingException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  JanusEmbeddingException(String message, Throwable cause) {
    super(message, cause);
  }

  JanusEmbeddingException(String message) {
    super(message);
  }
}
