package io.pact.janus.sdk;

import io.pact.janus.sdk.engine.Embedding;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * The options bag of {@code janus}: where the contract is written, how the engine is reached, and the
 * project's declared components. Immutable; each {@code with...} returns a copy.
 */
public final class JanusOptions {
  private final Path contractDirectory;
  private final Embedding embedding;
  private final List<Map<String, Object>> components;

  private JanusOptions(Path contractDirectory, Embedding embedding, List<Map<String, Object>> components) {
    this.contractDirectory = contractDirectory;
    this.embedding = embedding;
    this.components = components;
  }

  /**
   * The defaults: contracts under {@code contracts} in the working directory, and the
   * {@code janus-engine} subprocess that {@code JANUS_ENGINE} names.
   */
  public static JanusOptions defaults() {
    return new JanusOptions(Path.of(System.getProperty("user.dir"), "contracts"), null, List.of());
  }

  /** Writes the contract into {@code directory} instead. */
  public JanusOptions withContractDirectory(Path directory) {
    return new JanusOptions(Objects.requireNonNull(directory, "contract directory"), embedding, components);
  }

  /** Reaches the engine through {@code embedding} instead. */
  public JanusOptions withEmbedding(Embedding embedding) {
    return new JanusOptions(contractDirectory, Objects.requireNonNull(embedding, "embedding"), components);
  }

  /**
   * Declares the project's components (component-interfaces spec §10.2): each a component
   * declaration, as the project's configuration writes it. They are handed to the engine when the
   * session is created, with a {@code file} source's relative {@code reference} made absolute against
   * the working directory; whether one loads is the engine's answer, at the first {@code execute}.
   */
  public JanusOptions withComponents(List<? extends Map<String, ?>> components) {
    List<Map<String, Object>> copy = new ArrayList<>();
    int i = 0;
    for (Map<String, ?> declaration : Objects.requireNonNull(components, "components")) {
      @SuppressWarnings("unchecked")
      Map<String, Object> json = (Map<String, Object>) Json.value(declaration, "components[" + i++ + "]");
      copy.add(json);
    }
    return new JanusOptions(contractDirectory, embedding, List.copyOf(copy));
  }

  public Path contractDirectory() {
    return contractDirectory;
  }

  /** The declared components, as given; empty when none were. */
  public List<Map<String, Object>> components() {
    return components;
  }

  /** The embedding; resolved from the environment when the engine is first needed, if none was given. */
  public Embedding embedding() {
    return embedding == null ? Embedding.defaultEmbedding() : embedding;
  }
}
