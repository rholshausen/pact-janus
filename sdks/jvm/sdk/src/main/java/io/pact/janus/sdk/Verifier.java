package io.pact.janus.sdk;

import io.pact.janus.bindings.protocol.v1.Vocabulary.RequestFrameOp;
import io.pact.janus.sdk.engine.Embedding;
import io.pact.janus.sdk.engine.FramePipe;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * The provider primitives (behavioural spec {@code verifier}, {@code state-handler},
 * {@code request-filter}, {@code message-producer}, {@code verify}; ADR 0055, ADR 0056). A verification is one protocol run: this class
 * reads the pact files the test names, writes the handlers as loopback {@code http} hook entries,
 * sends one {@code verification/verify} and polls its stream. It decides no verdict: the run's status
 * is the engine's, and a failed run fails here.
 *
 * <p>Nothing is sent until {@link #verify()}. The {@code with...} methods and the handler
 * declarations return this verifier, so a test configures it in one chain.
 */
public final class Verifier {
  /** How long one {@code events/poll} may wait. An empty answer only means nothing happened within it. */
  private static final long POLL_WAIT_MS = 10_000;

  /** A state's setup: receives its parameters, and may return provider-state values, or null for none. */
  @FunctionalInterface
  public interface StateSetup {
    Map<String, Object> setup(Map<String, Object> params) throws Exception;
  }

  /** A state's teardown: receives its parameters. */
  @FunctionalInterface
  public interface StateTeardown {
    void teardown(Map<String, Object> params) throws Exception;
  }

  /** Receives the request, one member per slot ({@code method}, {@code path}, {@code headers}, …), and returns it; null changes nothing. */
  @FunctionalInterface
  public interface RequestFilter {
    Map<String, Object> filter(Map<String, Object> request) throws Exception;
  }

  /** Receives today's request for the message (its description, provider states and any request), and returns it. */
  @FunctionalInterface
  public interface MessageProducer {
    Message produce(Map<String, Object> request) throws Exception;
  }

  private record State(String state, StateSetup setup, StateTeardown teardown) {}

  private record Producer(String description, MessageProducer producer) {}

  private record Filter(RequestFilter filter, List<String> changes) {}

  private final String providerUrl;
  private List<Path> pacts;
  private Map<String, Object> broker;
  private Map<String, Object> urls;
  private Map<String, Object> options;
  private Map<String, Object> transport;
  private Path config;
  private List<Map<String, Object>> components;
  private Embedding embedding;
  private final List<State> states = new ArrayList<>();
  private final List<Filter> filters = new ArrayList<>();
  private final List<Producer> producers = new ArrayList<>();

  private Verifier(String providerUrl) {
    this.providerUrl = Objects.requireNonNull(providerUrl, "provider URL");
  }

  /** A verifier for the provider at {@code providerUrl}: the HTTP transport's {@code base-url}. */
  public static Verifier forProvider(String providerUrl) {
    return new Verifier(providerUrl);
  }

  /** Janus contracts or v1–v4 pacts, read from these files and sent inline. One source only. */
  public Verifier withPacts(List<Path> pacts) {
    this.pacts = List.copyOf(pacts);
    return this;
  }

  /** A broker source, as engine-protocol spec §8.3 spells it without its {@code kind}. */
  public Verifier withBroker(Map<String, ?> broker) {
    this.broker = json(broker, "broker");
    return this;
  }

  /** A urls source, as engine-protocol spec §8.3 spells it without its {@code kind}. */
  public Verifier withUrls(Map<String, ?> urls) {
    this.urls = json(urls, "urls");
    return this;
  }

  /**
   * The run's options, as {@code verification/verify} spells them: publish, filter, variants, … The
   * run's report is asked for unless they give {@code report} themselves (ADR 0059).
   */
  public Verifier withOptions(Map<String, ?> options) {
    this.options = json(options, "options");
    return this;
  }

  /** The HTTP transport's drive options beside {@code base-url}: {@code headers}, {@code client}. Passed as given. */
  public Verifier withTransport(Map<String, ?> transport) {
    this.transport = json(transport, "transport");
    return this;
  }

  /** A project configuration file ({@code verifier.janus.yaml}), loaded through the engine. */
  public Verifier withConfig(Path config) {
    this.config = Objects.requireNonNull(config, "config");
    return this;
  }

  /** Declared components, as for {@link JanusOptions#withComponents}. */
  @SuppressWarnings("unchecked")
  public Verifier withComponents(List<? extends Map<String, ?>> components) {
    this.components = (List<Map<String, Object>>) Json.value(components, "components");
    return this;
  }

  /** Reaches the engine through {@code embedding} instead of the default subprocess. */
  public Verifier withEmbedding(Embedding embedding) {
    this.embedding = Objects.requireNonNull(embedding, "embedding");
    return this;
  }

  /** Declares the handler for one provider state, with no teardown. */
  public Verifier state(String state, StateSetup setup) {
    return state(state, setup, null);
  }

  /** Declares the handler for one provider state. */
  public Verifier state(String state, StateSetup setup, StateTeardown teardown) {
    states.add(new State(Objects.requireNonNull(state, "state"), Objects.requireNonNull(setup, "setup"), teardown));
    return this;
  }

  /** Declares a request filter that changes the request's headers. */
  public Verifier requestFilter(RequestFilter filter) {
    return requestFilter(filter, List.of("headers"));
  }

  /** Declares a request filter that changes the request's {@code changes} slots. */
  public Verifier requestFilter(RequestFilter filter, List<String> changes) {
    filters.add(new Filter(Objects.requireNonNull(filter, "filter"), List.copyOf(changes)));
    return this;
  }

  /** Declares the producer of the message interaction with this description. */
  public Verifier message(String description, MessageProducer producer) {
    producers.add(new Producer(Objects.requireNonNull(description, "description"), Objects.requireNonNull(producer, "producer")));
    return this;
  }

  /** Runs the verification. Throws {@link VerificationFailedException} when the run failed. */
  public VerificationResult verify() {
    try (VerificationRun run = start()) {
      while (run.next() != null) {
        // Each result is collected by the run.
      }
      VerificationResult result = run.result();
      if ("failed".equals(result.status())) {
        throw new VerificationFailedException(result);
      }
      return result;
    }
  }

  /**
   * Starts the verification, as {@link #verify()} does, and returns the run, whose results are read
   * as the engine reports them. Close it, however it ends: that stops the handlers and the engine.
   */
  public VerificationRun start() {
    Map<String, Object> source = source();
    FramePipe pipe;
    try {
      pipe = (embedding == null ? Embedding.defaultEmbedding() : embedding).open();
    } catch (IOException e) {
      throw new JanusEmbeddingException(e.getMessage(), e);
    }
    EngineClient engine = new EngineClient(pipe);
    Loopback loopback = null;
    try {
      loopback = states.isEmpty() && filters.isEmpty() && producers.isEmpty() ? null : new Loopback();
      engine.hello(Janus.SDK_NAME, Janus.SDK_VERSION);
      Map<String, Object> loaded = null;
      if (config != null) {
        String path = Janus.hasRoot(config.toString())
            ? config.toString()
            : Path.of(System.getProperty("user.dir")).resolve(config).normalize().toString();
        loaded = map(engine.send(RequestFrameOp.CONFIG_LOAD, Map.of("path", path)).ok().get("config"));
      }
      Map<String, Object> target = new LinkedHashMap<>();
      Map<String, Object> http = new LinkedHashMap<>(transport == null ? Map.of() : transport);
      http.put("base-url", providerUrl);
      target.put("transports", List.of(Map.of("transport", "http", "options", http)));
      List<Object> declared = new ArrayList<>(loaded != null && loaded.get("components") instanceof List<?> l ? l : List.of());
      if (components != null) {
        components.stream().map(Janus::resolved).forEach(declared::add);
      }
      if (!declared.isEmpty()) {
        target.put("components", declared);
      }
      if (loaded != null || loopback != null) {
        target.put("hooks", hooks(loaded, loopback));
      }
      Map<String, Object> request = new LinkedHashMap<>();
      request.put("source", source);
      request.put("target", target);
      Map<String, Object> sent = new LinkedHashMap<>();
      sent.put("report", Map.of());
      if (options != null) {
        sent.putAll(options);
      }
      request.put("options", sent);
      String stream = (String) engine.send(RequestFrameOp.VERIFICATION_VERIFY, request).ok().get("stream");
      return new VerificationRun(engine, loopback, stream);
    } catch (IOException e) {
      close(loopback, engine);
      throw new JanusEmbeddingException("could not serve the verification's handlers: " + e.getMessage(), e);
    } catch (RuntimeException e) {
      close(loopback, engine);
      throw e;
    }
  }

  private static void close(Loopback loopback, EngineClient engine) {
    try {
      if (loopback != null) {
        loopback.close();
      }
    } finally {
      engine.close();
    }
  }

  /**
   * A started verification (ADR 0056 decision 3): one {@code verification/verify}, whose stream is
   * polled as results are asked for. Each {@code verification/interaction-result} is returned as the
   * engine sent it; the terminal event's summary is the {@link #result()}.
   */
  public static final class VerificationRun implements AutoCloseable {
    private final EngineClient engine;
    private final Loopback loopback;
    private final String stream;
    private final List<Map<String, Object>> results = new ArrayList<>();
    private final List<Map<String, Object>> warnings = new ArrayList<>();
    private boolean reported;
    private final java.util.ArrayDeque<Map<String, Object>> pending = new java.util.ArrayDeque<>();
    private VerificationResult result;
    private boolean closed;

    private VerificationRun(EngineClient engine, Loopback loopback, String stream) {
      this.engine = engine;
      this.loopback = loopback;
      this.stream = stream;
    }

    /** The next interaction result, polling for it when none has arrived; null once the run has ended. */
    public Map<String, Object> next() {
      while (pending.isEmpty() && result == null) {
        poll();
      }
      return pending.poll();
    }

    /** The run's result. Reads the rest of the stream first, when it has not ended. */
    public VerificationResult result() {
      while (result == null) {
        poll();
      }
      return result;
    }

    private void poll() {
      Map<String, Object> polled = engine.send(RequestFrameOp.EVENTS_POLL, Map.of("streams", List.of(stream), "wait-ms", POLL_WAIT_MS)).ok();
      for (Object item : (List<?>) polled.getOrDefault("events", List.of())) {
        Map<String, Object> event = map(item);
        if ("verification/interaction-result".equals(event.get("kind"))) {
          results.add(map(event.get("payload")));
          pending.add(map(event.get("payload")));
        }
        if ("verification/warning".equals(event.get("kind"))) {
          warnings.add(map(event.get("payload")));
        }
        if ("verification/report".equals(event.get("kind"))) {
          System.out.println(java.util.Objects.toString(map(event.get("payload")).get("text"), ""));
          reported = true;
        }
        if (Boolean.TRUE.equals(event.get("last"))) {
          if (reported) {
            System.out.println(); // sets the report apart from what the test runner prints next
          }
          Map<String, Object> summary = map(event.get("payload"));
          result = new VerificationResult(String.valueOf(summary.get("status")), summary, results, warnings);
        }
      }
    }

    /** Stops the handlers and ends the engine. */
    @Override
    public void close() {
      if (!closed) {
        closed = true;
        Verifier.close(loopback, engine);
      }
    }
  }

  private Map<String, Object> source() {
    long given = java.util.stream.Stream.of(pacts, broker, urls).filter(Objects::nonNull).count();
    if (given != 1) {
      throw new IllegalArgumentException("a Verifier needs exactly one of withPacts, withBroker and withUrls");
    }
    Map<String, Object> source = new LinkedHashMap<>();
    if (broker != null || urls != null) {
      source.put("kind", broker != null ? "broker" : "urls");
      source.putAll(broker != null ? broker : urls);
      return source;
    }
    List<Object> contracts = new ArrayList<>();
    for (Path pact : pacts) {
      try {
        contracts.add(Json.MAPPER.readValue(pact.toFile(), LinkedHashMap.class));
      } catch (IOException e) {
        throw new UncheckedIOException("could not read " + pact, e);
      }
    }
    source.put("kind", "inline");
    source.put("contracts", contracts);
    return source;
  }

  /** The configuration's entries at each point, then the handlers', each served by {@code loopback}. */
  private Map<String, Object> hooks(Map<String, Object> loaded, Loopback loopback) {
    Map<String, List<Object>> points = new LinkedHashMap<>();
    if (loaded != null && loaded.get("hooks") instanceof Map<?, ?> configured) {
      configured.forEach((point, entries) -> points.put((String) point, new ArrayList<>((List<?>) entries)));
    }
    for (int i = 0; i < states.size(); i++) {
      State state = states.get(i);
      String name = "state-" + (i + 1);
      add(points, loopback, "state-setup", entry(loopback, "state-setup", name, Map.of("state", state.state()), null),
          context -> answerState(() -> state.setup().setup(params(context))));
      if (state.teardown() != null) {
        add(points, loopback, "state-teardown", entry(loopback, "state-teardown", name, Map.of("state", state.state()), null),
            context -> answerState(() -> {
              state.teardown().teardown(params(context));
              return null;
            }));
      }
    }
    for (int i = 0; i < filters.size(); i++) {
      Filter filter = filters.get(i);
      List<String> declared = filter.changes().stream().map(slot -> "parts.request." + slot).toList();
      add(points, loopback, "before-request", entry(loopback, "before-request", "request-filter-" + (i + 1), null, declared),
          context -> answerFilter(filter, context));
    }
    for (int i = 0; i < producers.size(); i++) {
      Producer producer = producers.get(i);
      String name = "message-" + (i + 1);
      Map<String, Object> entry = entry(loopback, "produce-message", name, Map.of("interaction", producer.description()), List.of("parts"));
      add(points, loopback, "produce-message", entry, context -> answerMessage(producer.producer(), context));
    }
    Map<String, Object> hooks = new LinkedHashMap<>();
    hooks.put("version", loaded != null && loaded.get("version") != null ? loaded.get("version") : 1);
    hooks.put("hooks", points);
    return hooks;
  }

  private static void add(Map<String, List<Object>> points, Loopback loopback, String point, Map<String, Object> entry,
      java.util.function.Function<Map<String, Object>, Map<String, Object>> route) {
    points.computeIfAbsent(point, p -> new ArrayList<>()).add(entry);
    loopback.route("/" + point + "/" + entry.get("name"), route);
  }

  private static Map<String, Object> entry(Loopback loopback, String point, String name, Map<String, Object> when,
      List<String> changes) {
    Map<String, Object> entry = new LinkedHashMap<>();
    entry.put("name", name);
    if (when != null) {
      entry.put("when", when);
    }
    if (changes != null) {
      entry.put("changes", changes);
    }
    entry.put("run", Map.of("kind", "http", "url", loopback.url("/" + point + "/" + name),
        "headers", Map.of("authorization", loopback.authorization())));
    return entry;
  }

  private interface Call {
    Map<String, Object> call() throws Exception;
  }

  private static Map<String, Object> answerState(Call handler) {
    try {
      Map<String, Object> values = handler.call();
      return values == null ? Map.of("outcome", "ok") : Map.of("outcome", "ok", "values", values);
    } catch (StateUnsupportedException e) {
      return Map.of("outcome", "unsupported", "error", Map.of("code", "state-unsupported", "message", String.valueOf(e.getMessage())));
    } catch (Exception e) {
      return failed(e);
    }
  }

  private static Map<String, Object> answerFilter(Filter filter, Map<String, Object> context) {
    Map<String, Object> request = new LinkedHashMap<>();
    if (map(context.get("parts")).get("request") instanceof Map<?, ?> slots) {
      slots.forEach((slot, value) -> request.put((String) slot, map(value).get("content")));
    }
    try {
      Map<String, Object> returned = filter.filter().filter(request);
      if (returned == null) {
        return Map.of("outcome", "ok");
      }
      Map<String, Object> changes = new LinkedHashMap<>();
      for (String slot : filter.changes()) {
        if (returned.containsKey(slot)) {
          Map<String, Object> wrapped = new LinkedHashMap<>();
          wrapped.put("content", Json.value(returned.get(slot), slot));
          changes.put("parts.request." + slot, wrapped);
        }
      }
      return Map.of("outcome", "ok", "changes", changes);
    } catch (Exception e) {
      return failed(e);
    }
  }

  private static Map<String, Object> answerMessage(MessageProducer producer, Map<String, Object> context) {
    Map<String, Object> pactMessage = map(map(context.get("interaction")).get("pact-message"));
    try {
      Message message = producer.produce(map(pactMessage.get("request")));
      Map<String, Object> body = new LinkedHashMap<>();
      body.put("content", java.util.Base64.getEncoder().encodeToString(message.octets()));
      body.put("encoded", "base64");
      Map<String, Object> metadata = new LinkedHashMap<>();
      if (message.contentType() != null) {
        body.put("content-type", message.contentType());
        metadata.put("contentType", message.contentType());
      }
      metadata.putAll(message.metadata());
      return Map.of("outcome", "ok", "changes", Map.of("parts", Map.of("message",
          Map.of("metadata", Map.of("content", Json.value(metadata, "metadata")), "body", body))));
    } catch (Exception e) {
      return failed(e);
    }
  }

  private static Map<String, Object> failed(Exception e) {
    return Map.of("outcome", "failed", "error", Map.of("code", "handler-failed", "message", String.valueOf(e.getMessage())));
  }

  private static Map<String, Object> params(Map<String, Object> context) {
    return map(map(context.get("state")).get("params"));
  }

  @SuppressWarnings("unchecked")
  private static Map<String, Object> map(Object value) {
    return value instanceof Map<?, ?> m ? (Map<String, Object>) m : Map.of();
  }

  @SuppressWarnings("unchecked")
  private static Map<String, Object> json(Map<String, ?> value, String what) {
    return (Map<String, Object>) Json.value(Objects.requireNonNull(value, what), what);
  }
}
