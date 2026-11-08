package io.pact.janus.sdk;

import io.pact.janus.sdk.engine.Embedding;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * The options bag of {@code janus}: where the contract is written and whether that file is merged
 * into or overwritten, how the engine is reached, the project's declared components, and what the
 * contract records redacted. Immutable; each {@code with...} returns a copy.
 */
public final class JanusOptions {
  private final Path contractDirectory;
  private final Embedding embedding;
  private final List<Map<String, Object>> components;
  private final boolean overwriteContract;
  private final List<String> redact;
  private final List<String> keep;

  private JanusOptions(Path contractDirectory, Embedding embedding, List<Map<String, Object>> components,
      boolean overwriteContract, List<String> redact, List<String> keep) {
    this.contractDirectory = contractDirectory;
    this.embedding = embedding;
    this.components = components;
    this.overwriteContract = overwriteContract;
    this.redact = redact;
    this.keep = keep;
  }

  /**
   * The defaults: contracts under {@code contracts} in the working directory, and the
   * {@code janus-engine} subprocess that {@code JANUS_ENGINE} names.
   */
  public static JanusOptions defaults() {
    return new JanusOptions(Path.of(System.getProperty("user.dir"), "contracts"), null, List.of(), false, null,
        null);
  }

  /**
   * Writes the contract into {@code directory} instead. Several test classes recording one consumer
   * and provider merge into its one file (ADR 0065).
   */
  public JanusOptions withContractDirectory(Path directory) {
    return new JanusOptions(Objects.requireNonNull(directory, "contract directory"), embedding, components,
        overwriteContract, redact, keep);
  }

  /** With {@code true}, the engine replaces the contract file rather than merging into it (ADR 0065). */
  public JanusOptions withContractOverwrite(boolean overwrite) {
    return new JanusOptions(contractDirectory, embedding, components, overwrite, redact, keep);
  }

  /** Reaches the engine through {@code embedding} instead. */
  public JanusOptions withEmbedding(Embedding embedding) {
    return new JanusOptions(contractDirectory, Objects.requireNonNull(embedding, "embedding"), components,
        overwriteContract, redact, keep);
  }

  /**
   * Declares the project's components (component-interfaces spec §10.2): each a component
   * declaration, as the project's configuration writes it. They are handed to the engine when the
   * session is created, with a {@code file} source's {@code reference} resolved against the working
   * directory when it has no root; whether one loads is the engine's answer, at the first {@code execute}.
   */
  public JanusOptions withComponents(List<? extends Map<String, ?>> components) {
    List<Map<String, Object>> copy = new ArrayList<>();
    int i = 0;
    for (Map<String, ?> declaration : Objects.requireNonNull(components, "components")) {
      @SuppressWarnings("unchecked")
      Map<String, Object> json = (Map<String, Object>) Json.value(declaration, "components[" + i++ + "]");
      copy.add(json);
    }
    return new JanusOptions(contractDirectory, embedding, List.copyOf(copy), overwriteContract, redact, keep);
  }

  /**
   * Redacts these members in the contract too, beyond the credentials the transport declares (ADR
   * 0074): each a path, a part, a slot, then member names ({@code request.headers.x-api-key},
   * {@code request.body.password}). Handed to the engine as written.
   */
  public JanusOptions withRedacted(List<String> paths) {
    return new JanusOptions(contractDirectory, embedding, components, overwriteContract,
        List.copyOf(Objects.requireNonNull(paths, "paths")), keep);
  }

  /**
   * Records these members as they are, though the transport declares them as credentials: a fixed
   * token the provider's test environment accepts. HTTP's are the {@code authorization},
   * {@code proxy-authorization} and {@code cookie} request headers and the {@code set-cookie} response
   * header. Handed to the engine as written.
   */
  public JanusOptions withKept(List<String> paths) {
    return new JanusOptions(contractDirectory, embedding, components, overwriteContract, redact,
        List.copyOf(Objects.requireNonNull(paths, "paths")));
  }

  public Path contractDirectory() {
    return contractDirectory;
  }

  /** Whether the engine replaces the contract file rather than merging into it. */
  public boolean overwritesContract() {
    return overwriteContract;
  }

  /** The declared components, as given; empty when none were. */
  public List<Map<String, Object>> components() {
    return components;
  }

  /** The paths to redact beyond the transport's, as given; {@code null} when none were. */
  public List<String> redacted() {
    return redact;
  }

  /** The paths to record as they are, as given; {@code null} when none were. */
  public List<String> kept() {
    return keep;
  }

  /** The embedding; resolved from the environment when the engine is first needed, if none was given. */
  public Embedding embedding() {
    return embedding == null ? Embedding.defaultEmbedding() : embedding;
  }
}
