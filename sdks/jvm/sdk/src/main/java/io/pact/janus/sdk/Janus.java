package io.pact.janus.sdk;

import io.pact.janus.bindings.protocol.v1.AddInteraction;
import io.pact.janus.bindings.protocol.v1.AddInteractionResult;
import io.pact.janus.bindings.protocol.v1.Create;
import io.pact.janus.bindings.protocol.v1.CreateResult;
import io.pact.janus.bindings.protocol.v1.Finalise;
import io.pact.janus.bindings.protocol.v1.FinaliseResult;
import io.pact.janus.bindings.protocol.v1.InteractionResult;
import io.pact.janus.bindings.protocol.v1.Party;
import io.pact.janus.bindings.protocol.v1.ServeVariant;
import io.pact.janus.bindings.protocol.v1.SessionConfig;
import io.pact.janus.bindings.protocol.v1.StartTransport;
import io.pact.janus.bindings.protocol.v1.StartTransportResult;
import io.pact.janus.bindings.protocol.v1.VariantDescriptor;
import io.pact.janus.bindings.protocol.v1.VariantResult;
import io.pact.janus.bindings.protocol.v1.Variants;
import io.pact.janus.bindings.protocol.v1.VariantsResult;
import io.pact.janus.bindings.protocol.v1.Vocabulary.InteractionResultStatus;
import io.pact.janus.bindings.protocol.v1.Vocabulary.RequestFrameOp;
import io.pact.janus.bindings.protocol.v1.Vocabulary.StartTransportTransport;
import io.pact.janus.bindings.protocol.v1.Vocabulary.VariantResultStatus;
import io.pact.janus.sdk.engine.FramePipe;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;

/**
 * The {@code janus} primitive: the SDK configured for one consumer/provider pair, which every other
 * primitive hangs off. A test suite uses one; the {@link JanusExtension} holds it for a JUnit test
 * class and runs {@link #finalise()} after the class's last test.
 *
 * <p>Constructing one makes no protocol call. The first {@link #execute} starts the engine and
 * sends {@code engine/hello}; one engine and at most one open consumer session belong to this
 * object, and {@link #finalise()} ends both.
 *
 * <p>Every method blocks until the engine has answered. Methods are synchronised: the protocol
 * orders a session's operations, and so does this object.
 */
public final class Janus implements AutoCloseable {
  /** What this SDK calls itself in {@code engine/hello}. */
  public static final String SDK_NAME = "janus-jvm";
  /** This SDK's version, as {@code engine/hello} reports it. */
  public static final String SDK_VERSION = "0.0.0";

  private final String consumer;
  private final String provider;
  private final JanusOptions options;

  private EngineClient engine;
  private String session;
  private Mock mock;
  /** Every interaction added in the open session, by handle. */
  private final Map<String, Execution> executions = new LinkedHashMap<>();
  /** The executions in the open session that failed — the one verdict the engine does not hold. */
  private final List<Execution> failed = new ArrayList<>();

  private Janus(String consumer, String provider, JanusOptions options) {
    this.consumer = Objects.requireNonNull(consumer, "consumer");
    this.provider = Objects.requireNonNull(provider, "provider");
    this.options = Objects.requireNonNull(options, "options");
  }

  /** {@code janus(consumer, provider)} with the default options. */
  public static Janus of(String consumer, String provider) {
    return new Janus(consumer, provider, JanusOptions.defaults());
  }

  /** {@code janus(consumer, provider, options)}. */
  public static Janus of(String consumer, String provider, JanusOptions options) {
    return new Janus(consumer, provider, options);
  }

  public String consumer() {
    return consumer;
  }

  public String provider() {
    return provider;
  }

  public JanusOptions options() {
    return options;
  }

  /** Where {@link #finalise()} writes the contract: {@code <contract directory>/<consumer>-<provider>.janus.json}. */
  public Path contractFile() {
    return options.contractDirectory().resolve(consumer + "-" + provider + ".janus.json");
  }

  /** {@code interaction}: begins a new interaction specification. No protocol call happens. */
  public Interaction interaction(String description) {
    return new Interaction(description);
  }

  /**
   * {@code execute}: submits {@code interaction} and runs {@code test} once per variant the engine
   * selects, in the engine's order, with that variant armed on the mock.
   *
   * <p>A test that throws fails that variant only: every remaining variant still runs, and then
   * {@code execute} throws one {@link ExecuteFailedException} naming every failed variant. An engine
   * error is not a test's verdict and is not recorded as one: it ends the run at once, and the
   * variants after it do not run (ADR 0019). Whether the mock saw what the interaction describes is
   * the engine's verdict, not this method's — {@link #finalise()} reports it.
   *
   * @throws JanusEngineException before any variant runs, when the engine rejects the interaction
   *     ({@code interaction-invalid}, with its problems) or cannot select variants
   *     ({@code variant-budget-exceeded}); from the first {@code execute}, when the handshake fails;
   *     and from inside the loop, when the engine cannot arm a variant
   * @throws ExecuteFailedException when the test failed on one or more variants
   */
  public void execute(Interaction interaction, VariantTest test) {
    Objects.requireNonNull(test, "test");
    Execution run = begin(interaction);
    List<VariantFailure> failures = new ArrayList<>();
    for (Variant variant : run.variants()) {
      VariantFailure failure = run(run, variant, test);
      if (failure != null) {
        failures.add(failure);
      }
    }
    if (!failures.isEmpty()) {
      throw new ExecuteFailedException(run.description(), failures, run.variants().size());
    }
  }

  /**
   * {@code finalise}: ends the session and the engine, and writes the contract if the engine
   * produced one and every {@code execute} passed.
   *
   * @return the contract file written, or empty when there was no session to finalise
   * @throws ContractWithheldException when no contract was written, naming every interaction and
   *     variant that did not verify, and every interaction whose {@code execute} failed
   */
  public synchronized Optional<Path> finalise() {
    if (engine == null) {
      return Optional.empty();
    }
    String open = session;
    Map<String, Execution> added = new LinkedHashMap<>(executions);
    List<Execution> failedRuns = new ArrayList<>(failed);
    EngineClient.Response response = null;
    try {
      if (open != null) {
        Finalise body = new Finalise();
        body.setSession(open);
        response = engine.send(RequestFrameOp.CONSUMER_SESSION_FINALISE, body);
      }
    } finally {
      closeEngine();
    }
    if (response == null) {
      return Optional.empty();
    }
    FinaliseResult result = Json.MAPPER.convertValue(response.ok(), FinaliseResult.class);
    byte[] contract;
    try {
      contract = EngineClient.okMemberBytes(response.raw(), "contract");
    } catch (IOException e) {
      throw new JanusEmbeddingException("could not read the contract out of the finalise response", e);
    }
    boolean engineWithheld = result.getContract() == null || contract == null;
    if (engineWithheld || !failedRuns.isEmpty()) {
      throw withheld(result, added, failedRuns, engineWithheld);
    }
    Path file = contractFile();
    try {
      Files.createDirectories(file.getParent());
      byte[] bytes = new byte[contract.length + 1];
      System.arraycopy(contract, 0, bytes, 0, contract.length);
      bytes[contract.length] = '\n';
      Files.write(file, bytes);
    } catch (IOException e) {
      throw new UncheckedIOException("could not write the contract to " + file, e);
    }
    return Optional.of(file);
  }

  /** {@link #finalise()}, for try-with-resources outside a test framework. */
  @Override
  public void close() {
    finalise();
  }

  // -----------------------------------------------------------------------------------------------
  // The steps of execute, shared with JanusExtension's per-variant dynamic tests.

  /** One interaction added to the session: its handle, the engine's selection, and the mock. */
  static final class Execution {
    private final String description;
    private final String session;
    private final String handle;
    private final List<Variant> variants;
    private final Mock mock;
    private final List<VariantFailure> failures = new ArrayList<>();
    private Throwable error;

    Execution(String description, String session, String handle, List<Variant> variants, Mock mock) {
      this.description = description;
      this.session = session;
      this.handle = handle;
      this.variants = variants;
      this.mock = mock;
    }

    String description() {
      return description;
    }

    List<Variant> variants() {
      return variants;
    }

    Variant variant(String id) {
      for (Variant v : variants) {
        if (v.id().equals(id)) {
          return v;
        }
      }
      return null;
    }
  }

  /**
   * {@code create} if no session is open, {@code add-interaction}, {@code start-transport} if the
   * session has none, then {@code variants}.
   */
  synchronized Execution begin(Interaction interaction) {
    Objects.requireNonNull(interaction, "interaction");
    Map<String, Object> document = interaction.toDocument();
    try {
      EngineClient client = engine();
      if (session == null) {
        Create create = new Create();
        SessionConfig config = new SessionConfig();
        config.setConsumer(party(consumer));
        config.setProvider(party(provider));
        create.setConfig(config);
        session = client.call(RequestFrameOp.CONSUMER_SESSION_CREATE, create, CreateResult.class).getSession();
      }
      AddInteraction add = new AddInteraction();
      add.setSession(session);
      add.setInteraction(document);
      String handle = client.call(RequestFrameOp.CONSUMER_SESSION_ADD_INTERACTION, add, AddInteractionResult.class)
          .getHandle();
      if (mock == null) {
        StartTransport start = new StartTransport();
        start.setSession(session);
        start.setTransport(StartTransportTransport.HTTP);
        mock = new Mock(client.call(RequestFrameOp.CONSUMER_SESSION_START_TRANSPORT, start, StartTransportResult.class)
            .getEndpoint());
      }
      Variants ask = new Variants();
      ask.setSession(session);
      ask.setHandle(handle);
      VariantsResult selection = client.call(RequestFrameOp.CONSUMER_SESSION_VARIANTS, ask, VariantsResult.class);
      List<Variant> variants = new ArrayList<>();
      for (VariantDescriptor descriptor : selection.getVariants()) {
        variants.add(Variant.of(descriptor));
      }
      Execution run = new Execution(interaction.description(), session, handle,
          Collections.unmodifiableList(variants), mock);
      executions.put(handle, run);
      return run;
    } catch (RuntimeException e) {
      if (session != null) {
        // The suite's session stays open, and a contract must not be written without this
        // interaction in it.
        Execution run = new Execution(interaction.description(), session, null, List.of(), mock);
        run.error = e;
        failed.add(run);
      }
      throw e;
    }
  }

  /**
   * {@code serve-variant}, then the test. Returns the failure, or null when the variant passed.
   *
   * <p>Arming the variant is machinery, not a verdict: an engine error there ends the run at once
   * (ADR 0019) rather than being recorded as this variant's failure, because an engine that cannot
   * arm this variant cannot arm the next one either, and one failure per remaining variant would say
   * the tests failed when the engine did. The interaction is still marked failed, so {@code finalise}
   * withholds the contract.
   */
  VariantFailure run(Execution run, Variant variant, VariantTest test) {
    try {
      synchronized (this) {
        if (engine == null || !run.session.equals(session)) {
          throw new IllegalStateException(
              "the session '" + run.description + "' was added to has been finalised; execute it again");
        }
        ServeVariant serve = new ServeVariant();
        serve.setSession(run.session);
        serve.setHandle(run.handle);
        serve.setVariant(variant.id());
        engine.send(RequestFrameOp.CONSUMER_SESSION_SERVE_VARIANT, serve);
      }
    } catch (RuntimeException machinery) {
      synchronized (this) {
        if (!failed.contains(run) && run.session.equals(session)) {
          failed.add(run);
        }
      }
      throw machinery;
    }
    try {
      test.run(run.mock, variant);
      return null;
    } catch (VirtualMachineError e) {
      throw e;
    } catch (Throwable t) {
      if (t instanceof InterruptedException) {
        Thread.currentThread().interrupt();
      }
      VariantFailure failure = new VariantFailure(variant, t);
      synchronized (this) {
        run.failures.add(failure);
        if (!failed.contains(run) && run.session.equals(session)) {
          failed.add(run);
        }
      }
      return failure;
    }
  }

  // -----------------------------------------------------------------------------------------------

  private EngineClient engine() {
    if (engine == null) {
      FramePipe pipe;
      try {
        pipe = options.embedding().open();
      } catch (IOException e) {
        throw new JanusEmbeddingException(e.getMessage(), e);
      }
      EngineClient client = new EngineClient(pipe);
      try {
        client.hello(SDK_NAME, SDK_VERSION);
      } catch (RuntimeException e) {
        client.close(); // nothing is retried
        throw e;
      }
      engine = client;
    }
    return engine;
  }

  private void closeEngine() {
    EngineClient client = engine;
    engine = null;
    session = null;
    mock = null;
    executions.clear();
    failed.clear();
    if (client != null) {
      client.close();
    }
  }

  private static Party party(String name) {
    Party party = new Party();
    party.setName(name);
    return party;
  }

  private ContractWithheldException withheld(FinaliseResult result, Map<String, Execution> added,
      List<Execution> failedRuns, boolean engineWithheld) {
    StringBuilder text = new StringBuilder("No contract was written for ").append(consumer).append(" -> ")
        .append(provider).append(" (").append(contractFile()).append(").");
    if (engineWithheld) {
      text.append("\nThe engine withheld it: not every interaction verified on every variant it required.");
      boolean named = false;
      for (InteractionResult interaction : result.getResults() == null ? List.<InteractionResult>of() : result.getResults()) {
        if (InteractionResultStatus.VERIFIED.equals(interaction.getStatus())) {
          continue;
        }
        named = true;
        Execution run = added.get(interaction.getHandle());
        text.append("\n  - '").append(run == null ? interaction.getHandle() : run.description()).append("': ")
            .append(interaction.getStatus());
        for (VariantResult v : interaction.getVariants() == null ? List.<VariantResult>of() : interaction.getVariants()) {
          if (VariantResultStatus.VERIFIED.equals(v.getStatus())) {
            continue;
          }
          Variant variant = run == null ? null : run.variant(v.getVariant());
          text.append("\n      - ").append(variant == null ? v.getVariant() : variant.label() + " [" + v.getVariant() + "]")
              .append(": ").append(v.getStatus());
          if (VariantResultStatus.NOT_EXERCISED.equals(v.getStatus())) {
            text.append(" (nothing exercised the mock under this variant)");
          }
          for (Map<String, Object> mismatch : v.getMismatches() == null ? List.<Map<String, Object>>of() : v.getMismatches()) {
            Object path = mismatch.get("path");
            text.append("\n          ").append(path == null ? "" : path + ": ").append(mismatch.get("message"));
          }
        }
      }
      if (!named) {
        text.append("\n  (the engine returned no contract, and named no interaction that did not verify)");
      }
    }
    List<String> failedDescriptions = new ArrayList<>();
    if (!failedRuns.isEmpty()) {
      text.append("\nThese interactions' tests failed, so a contract would claim variants the consumer did not handle:");
      for (Execution run : failedRuns) {
        failedDescriptions.add(run.description());
        text.append("\n  - '").append(run.description()).append("'");
        if (run.error != null) {
          text.append(": ").append(run.error);
        }
        for (VariantFailure f : run.failures) {
          text.append("\n      - ").append(f.variant().label()).append(" [").append(f.variant().id()).append("]: ")
              .append(f.cause());
        }
      }
    }
    return new ContractWithheldException(text.toString(), result.getResults(), failedDescriptions, engineWithheld);
  }

  @Override
  public String toString() {
    return "Janus[" + consumer + " -> " + provider + "]";
  }
}
