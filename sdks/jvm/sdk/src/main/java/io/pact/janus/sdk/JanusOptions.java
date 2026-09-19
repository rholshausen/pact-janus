package io.pact.janus.sdk;

import io.pact.janus.sdk.engine.Embedding;
import java.nio.file.Path;
import java.util.Objects;

/**
 * The options bag of {@code janus}: where the contract is written, and how the engine is reached.
 * Immutable; each {@code with...} returns a copy.
 */
public final class JanusOptions {
  private final Path contractDirectory;
  private final Embedding embedding;

  private JanusOptions(Path contractDirectory, Embedding embedding) {
    this.contractDirectory = contractDirectory;
    this.embedding = embedding;
  }

  /**
   * The defaults: contracts under {@code contracts} in the working directory, and the
   * {@code janus-engine} subprocess that {@code JANUS_ENGINE} names.
   */
  public static JanusOptions defaults() {
    return new JanusOptions(Path.of(System.getProperty("user.dir"), "contracts"), null);
  }

  /** Writes the contract into {@code directory} instead. */
  public JanusOptions withContractDirectory(Path directory) {
    return new JanusOptions(Objects.requireNonNull(directory, "contract directory"), embedding);
  }

  /** Reaches the engine through {@code embedding} instead. */
  public JanusOptions withEmbedding(Embedding embedding) {
    return new JanusOptions(contractDirectory, Objects.requireNonNull(embedding, "embedding"));
  }

  public Path contractDirectory() {
    return contractDirectory;
  }

  /** The embedding; resolved from the environment when the engine is first needed, if none was given. */
  public Embedding embedding() {
    return embedding == null ? Embedding.defaultEmbedding() : embedding;
  }
}
