package io.pact.janus.sdk.engine;

import java.io.IOException;

/**
 * One embedding's byte-pipe to the engine (engine-protocol spec §3): one request frame in, the
 * response frame that answers it out.
 *
 * <p>This is the whole of what the idiomatic layer needs from an embedding, and it is deliberately
 * the shape of the WASM call pipe (§3.1, {@code call(request) -> response}) so that a Chicory
 * embedding can implement it directly; the subprocess embedding ({@link SubprocessEmbedding}) adds
 * the Content-Length framing of §3.2 and skips frames that answer nothing (events, unknown types).
 * Frames are opaque bytes here — the pipe never interprets a document beyond finding its answer.
 */
public interface FramePipe extends AutoCloseable {

  /**
   * Sends one request frame and returns the response frame that answers it — the response with
   * the same {@code id}, or a pipe-level error response whose {@code id} is empty (§4.4).
   *
   * @throws IOException when the pipe itself fails: the engine exited, or wrote something that is
   *     not a frame
   */
  byte[] call(byte[] requestFrame) throws IOException;

  /**
   * Ends the engine (engine-protocol spec §6): for the subprocess, closes its stdin — the engine's
   * lease on life — and reaps the process.
   */
  @Override
  void close();
}
