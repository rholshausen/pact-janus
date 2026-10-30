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
 * {@code request-filter}, {@code verify}; ADR 0055). A verification is one protocol run: this class
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

  private record State(String state, StateSetup setup, StateTeardown teardown) {}

  private record Filter(RequestFilter filter, List<String> changes) {}

  private final String providerUrl;
  private List<Path> pacts;
  private Map<String, Object> broker;
  private Map<String, Object> urls;
  private Map<String, Object> options;
  private Path config;
  private List<Map<String, Object>> components;
  private Embedding embedding;
  private final List<State> states = new ArrayList<>();
  private final List<Filter> filters = new ArrayList<>();

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

  /** The run's options, as {@code verification/verify} spells them: publish, filter, variants, … */
  public Verifier withOptions(Map<String, ?> options) {
    this.options = json(options, "options");
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

  /** Runs the verification. Throws {@link VerificationFailedException} when the run failed. */
  public VerificationResult verify() {
    Map<String, Object> source = source();
    FramePipe pipe;
    try {
      pipe = (embedding == null ? Embedding.defaultEmbedding() : embedding).open();
    } catch (IOException e) {
      throw new JanusEmbeddingException(e.getMessage(), e);
    }
    try (EngineClient engine = new EngineClient(pipe);
        Loopback loopback = states.isEmpty() && filters.isEmpty() ? null : new Loopback()) {
      engine.hello(Janus.SDK_NAME, Janus.SDK_VERSION);
      Map<String, Object> loaded = null;
      if (config != null) {
        String path = Janus.hasRoot(config.toString())
            ? config.toString()
            : Path.of(System.getProperty("user.dir")).resolve(config).normalize().toString();
        loaded = map(engine.send(RequestFrameOp.CONFIG_LOAD, Map.of("path", path)).ok().get("config"));
      }
      Map<String, Object> target = new LinkedHashMap<>();
      target.put("transports", List.of(Map.of("transport", "http", "options", Map.of("base-url", providerUrl))));
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
      if (options != null) {
        request.put("options", options);
      }
      String stream = (String) engine.send(RequestFrameOp.VERIFICATION_VERIFY, request).ok().get("stream");
      VerificationResult result = poll(engine, stream);
      if ("failed".equals(result.status())) {
        throw new VerificationFailedException(result);
      }
      return result;
    } catch (IOException e) {
      throw new JanusEmbeddingException("could not serve the verification's handlers: " + e.getMessage(), e);
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

  /** Polls the run's stream to its terminal event, collecting every interaction result. */
  private static VerificationResult poll(EngineClient engine, String stream) {
    List<Map<String, Object>> results = new ArrayList<>();
    for (;;) {
      Map<String, Object> polled = engine.send(RequestFrameOp.EVENTS_POLL, Map.of("streams", List.of(stream), "wait-ms", POLL_WAIT_MS)).ok();
      for (Object item : (List<?>) polled.getOrDefault("events", List.of())) {
        Map<String, Object> event = map(item);
        if ("verification/interaction-result".equals(event.get("kind"))) {
          results.add(map(event.get("payload")));
        }
        if (Boolean.TRUE.equals(event.get("last"))) {
          Map<String, Object> summary = map(event.get("payload"));
          return new VerificationResult(String.valueOf(summary.get("status")), summary, results);
        }
      }
    }
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
