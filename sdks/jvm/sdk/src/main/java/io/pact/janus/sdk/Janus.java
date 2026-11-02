package io.pact.janus.sdk;

import io.pact.janus.bindings.protocol.v1.AddInteraction;
import io.pact.janus.bindings.protocol.v1.AddInteractionResult;
import io.pact.janus.bindings.protocol.v1.ContractWrite;
import io.pact.janus.bindings.protocol.v1.Create;
import io.pact.janus.bindings.protocol.v1.CreateResult;
import io.pact.janus.bindings.protocol.v1.Finalise;
import io.pact.janus.bindings.protocol.v1.FinaliseResult;
import io.pact.janus.bindings.protocol.v1.InteractionResult;
import io.pact.janus.bindings.protocol.v1.Party;
import io.pact.janus.bindings.protocol.v1.RefusedInteraction;
import io.pact.janus.bindings.protocol.v1.ReportVariant;
import io.pact.janus.bindings.protocol.v1.ReportVariantResult;
import io.pact.janus.bindings.protocol.v1.ServeVariant;
import io.pact.janus.bindings.protocol.v1.SessionConfig;
import io.pact.janus.bindings.protocol.v1.StartTransport;
import io.pact.janus.bindings.protocol.v1.StartTransportResult;
import io.pact.janus.bindings.protocol.v1.VariantDescriptor;
import io.pact.janus.bindings.protocol.v1.VariantResult;
import io.pact.janus.bindings.protocol.v1.Variants;
import io.pact.janus.bindings.protocol.v1.VariantsResult;
import io.pact.janus.bindings.protocol.v1.Vocabulary.InteractionResultStatus;
import io.pact.janus.bindings.protocol.v1.Vocabulary.ReportVariantStatus;
import io.pact.janus.bindings.protocol.v1.Vocabulary.RequestFrameOp;
import io.pact.janus.bindings.protocol.v1.Vocabulary.StartTransportTransport;
import io.pact.janus.bindings.protocol.v1.Vocabulary.VariantResultStatus;
import io.pact.janus.sdk.engine.FramePipe;
import java.io.File;
import java.io.IOException;
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
   * selects, in the engine's order, with that variant armed on the mock, and reports each test's
   * verdict to the engine, which answers with the variant's result (ADR 0064).
   *
   * <p>A variant fails when that result is not {@code verified}: the test threw, the mock rejected its
   * request, or nothing exercised the mock. It fails that variant only: every remaining variant still
   * runs, and then {@code execute} throws one {@link ExecuteFailedException} naming every failed
   * variant. An engine error is not a test's verdict and is not recorded as one: it ends the run at
   * once, and the variants after it do not run (ADR 0019).
   *
   * @throws JanusEngineException before any variant runs, when the engine rejects the interaction
   *     ({@code interaction-invalid}, with its problems) or cannot select variants
   *     ({@code variant-budget-exceeded}); from the first {@code execute}, when the handshake fails;
   *     and from inside the loop, when the engine cannot arm a variant
   * @throws ExecuteFailedException when the engine did not verify one or more variants
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
   * produced one.
   *
   * @return the contract file written, or empty when there was no session to finalise
   * @throws ContractWithheldException when the engine withheld the contract, naming every interaction
   *     and variant that did not verify, and every interaction the engine refused
   */
  public synchronized Optional<Path> finalise() {
    if (engine == null) {
      return Optional.empty();
    }
    String open = session;
    Map<String, Execution> added = new LinkedHashMap<>(executions);
    EngineClient.Response response = null;
    try {
      if (open != null) {
        Finalise body = new Finalise();
        body.setSession(open);
        // The engine's host writes the file, merging into it (ADR 0065): the SDK names it, and writes nothing.
        ContractWrite write = new ContractWrite();
        write.setPath(contractFile().toAbsolutePath().toString());
        write.setMode(options.overwritesContract() ? "overwrite" : "merge");
        body.setWrite(write);
        response = engine.send(RequestFrameOp.CONSUMER_SESSION_FINALISE, body);
      }
    } finally {
      closeEngine();
    }
    if (response == null) {
      return Optional.empty();
    }
    FinaliseResult result = Json.MAPPER.convertValue(response.ok(), FinaliseResult.class);
    String written = result.getWritten();
    if (written == null && (result.getContractText() != null || result.getContract() != null)) {
      // An engine older than ADR 0065 ignored 'write' and answered the contract: nothing was withheld.
      throw new JanusEmbeddingException("the engine answered finalise with the contract instead of writing it: it "
          + "predates 'contract-writing' (ADR 0065), which this SDK needs; use the engine this SDK pins", null);
    }
    if (written == null) {
      throw withheld(result, added);
    }
    return Optional.of(Path.of(written));
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
    EngineClient client = engine();
    if (session == null) {
      Create create = new Create();
      SessionConfig config = new SessionConfig();
      config.setConsumer(party(consumer));
      config.setProvider(party(provider));
      if (!options.components().isEmpty()) {
        config.setComponents(options.components().stream().map(Janus::resolved).toList());
      }
      create.setConfig(config);
      session = client.call(RequestFrameOp.CONSUMER_SESSION_CREATE, create, CreateResult.class).getSession();
    }
    AddInteraction add = new AddInteraction();
    add.setSession(session);
    add.setInteraction(document);
    String handle = client.call(RequestFrameOp.CONSUMER_SESSION_ADD_INTERACTION, add, AddInteractionResult.class)
        .getHandle();
    // Known by its handle from here, so a withheld contract names it as the test did.
    executions.put(handle, new Execution(interaction.description(), session, handle, List.of(), mock));
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
  }

  /**
   * {@code serve-variant}, the test, then {@code report-variant} with its verdict. Returns the failure,
   * or null when the engine verified the variant.
   *
   * <p>Arming the variant and reporting on it are machinery, not a verdict: an engine error there ends
   * the run at once (ADR 0019) rather than being recorded as this variant's failure, because an engine
   * that cannot arm this variant cannot arm the next one either, and one failure per remaining variant
   * would say the tests failed when the engine did. Whether the variant passed is the engine's answer
   * to the report (ADR 0064): this SDK keeps no verdict of its own.
   */
  VariantFailure run(Execution run, Variant variant, VariantTest test) {
    synchronized (this) {
      requireOpen(run);
      ServeVariant serve = new ServeVariant();
      serve.setSession(run.session);
      serve.setHandle(run.handle);
      serve.setVariant(variant.id());
      engine.send(RequestFrameOp.CONSUMER_SESSION_SERVE_VARIANT, serve);
    }
    Throwable thrown = null;
    try {
      test.run(run.mock, variant);
    } catch (VirtualMachineError e) {
      throw e;
    } catch (Throwable t) {
      if (t instanceof InterruptedException) {
        Thread.currentThread().interrupt();
      }
      thrown = t;
    }
    VariantResult result;
    synchronized (this) {
      requireOpen(run);
      ReportVariant report = new ReportVariant();
      report.setSession(run.session);
      report.setHandle(run.handle);
      report.setVariant(variant.id());
      report.setStatus(thrown == null ? ReportVariantStatus.PASSED : ReportVariantStatus.FAILED);
      if (thrown != null) {
        report.setReason(thrown.toString());
      }
      result = engine.call(RequestFrameOp.CONSUMER_SESSION_REPORT_VARIANT, report, ReportVariantResult.class)
          .getResult();
    }
    if (VariantResultStatus.VERIFIED.equals(result.getStatus())) {
      return null;
    }
    return new VariantFailure(variant, result, thrown != null ? thrown : notVerified(result));
  }

  private void requireOpen(Execution run) {
    if (engine == null || !run.session.equals(session)) {
      throw new IllegalStateException(
          "the session '" + run.description + "' was added to has been finalised; execute it again");
    }
  }

  /** The engine's verdict on a variant whose test passed, which the variant fails with. */
  static final class NotVerified extends AssertionError {
    private static final long serialVersionUID = 1L;

    NotVerified(String message) {
      super(message);
    }
  }

  /** The engine's verdict on a variant whose test passed, as the failure a test framework reports. */
  private static AssertionError notVerified(VariantResult result) {
    StringBuilder text = new StringBuilder();
    if (VariantResultStatus.NOT_EXERCISED.equals(result.getStatus())) {
      text.append("nothing exercised the mock under this variant");
    } else {
      text.append("the engine found this variant ").append(result.getStatus());
    }
    appendMismatches(text, result.getMismatches(), "\n  ");
    return new NotVerified(text.toString());
  }

  /** Each mismatch on a line of its own, after {@code indent}: its path, then its message. */
  static void appendMismatches(StringBuilder text, List<Map<String, Object>> mismatches, String indent) {
    for (Map<String, Object> mismatch : mismatches == null ? List.<Map<String, Object>>of() : mismatches) {
      Object path = mismatch.get("path");
      text.append(indent).append(path == null ? "" : path + ": ").append(mismatch.get("message"));
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
    if (client != null) {
      client.close();
    }
  }

  private static Party party(String name) {
    Party party = new Party();
    party.setName(name);
    return party;
  }

  private ContractWithheldException withheld(FinaliseResult result, Map<String, Execution> added) {
    StringBuilder text = new StringBuilder("No contract was written for ").append(consumer).append(" -> ")
        .append(provider).append(" (").append(contractFile()).append(").");
    text.append("\nThe engine withheld it: not every interaction verified on every variant it required.");
    List<String> interactions = new ArrayList<>();
    for (InteractionResult interaction : result.getResults() == null ? List.<InteractionResult>of() : result.getResults()) {
      if (InteractionResultStatus.VERIFIED.equals(interaction.getStatus())) {
        continue;
      }
      Execution run = added.get(interaction.getHandle());
      String description = run == null ? interaction.getHandle() : run.description();
      interactions.add(description);
      text.append("\n  - '").append(description).append("': ").append(interaction.getStatus());
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
        if (v.getTest() != null && v.getTest().getReason() != null) {
          text.append("\n          its test failed: ").append(v.getTest().getReason());
        }
        appendMismatches(text, v.getMismatches(), "\n          ");
      }
    }
    List<RefusedInteraction> refused = result.getRefused() == null ? List.of() : result.getRefused();
    for (RefusedInteraction refusal : refused) {
      String description = refusal.getDescription() == null ? "an interaction" : refusal.getDescription();
      interactions.add(description);
      text.append("\n  - '").append(description).append("': refused, ").append(refusal.getMessage())
          .append(" [").append(refusal.getCode()).append("]");
    }
    if (interactions.isEmpty()) {
      text.append("\n  (the engine returned no contract, and named no interaction that did not verify)");
    }
    return new ContractWithheldException(text.toString(), result.getResults(), refused, interactions);
  }

  @Override
  public String toString() {
    return "Janus[" + consumer + " -> " + provider + "]";
  }

  /**
   * Whether a reference has a root, and so is passed as written (behavioural spec, {@code janus}). The
   * rule is on the text: on Windows {@code /components/x} names no drive, which {@link Path#isAbsolute}
   * calls relative and other languages' libraries call absolute.
   */
  static boolean hasRoot(String reference) {
    return Path.of(reference).isAbsolute() || reference.startsWith("/") || reference.startsWith(File.separator);
  }

  /**
   * A declaration as the engine receives it (lifecycle-hooks spec §7.1: the loader resolves paths, the
   * engine resolves none against a directory of its own). Everything else is passed as written.
   */
  @SuppressWarnings("unchecked")
  static Map<String, Object> resolved(Map<String, Object> declaration) {
    if (!(declaration.get("source") instanceof Map<?, ?> source)
        || !"file".equals(source.get("kind"))
        || !(source.get("reference") instanceof String reference)
        || hasRoot(reference)) {
      return declaration;
    }
    Map<String, Object> resolvedSource = new LinkedHashMap<>((Map<String, Object>) source);
    resolvedSource.put("reference", Path.of(System.getProperty("user.dir")).resolve(reference).toString());
    Map<String, Object> copy = new LinkedHashMap<>(declaration);
    copy.put("source", resolvedSource);
    return copy;
  }
}
