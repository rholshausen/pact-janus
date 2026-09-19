package io.pact.janus.sdk.engine;

import java.io.IOException;

/**
 * How the engine is reached (SDK spec §2; ADR 0003): something that can start an engine and hand
 * back a {@link FramePipe} to it. The JVM SDK ships the subprocess embedding only — a WASM guest
 * cannot host a consumer test's mock server (Phase 9 finding 3) — and this interface is the seam a
 * Chicory embedding would implement.
 */
@FunctionalInterface
public interface Embedding {

  /** Starts an engine. Called lazily, by the first {@code execute} that needs one. */
  FramePipe open() throws IOException;

  /** The default: the {@code janus-engine} subprocess, located as {@link SubprocessEmbedding#fromEnvironment()} says. */
  static Embedding defaultEmbedding() {
    return SubprocessEmbedding.fromEnvironment();
  }
}
