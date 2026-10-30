package io.pact.janus.sdk;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Function;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestFactory;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;
import org.junit.jupiter.api.extension.RegisterExtension;
import org.junit.platform.engine.TestExecutionResult.Status;

/**
 * pact-jvm's JUnit 5 provider tests (provider/junit5/src/test at 88477ee), ported to
 * {@link JanusProviderExtension} (ADR 0056 decisions 3 and 5). Each nested class is the original
 * with its annotations' Janus spellings: {@code @TestTemplate} with {@code context.verifyInteraction()}
 * is a {@code @TestFactory} returning {@code janus.interactions(this)}, {@code @PactFolder} is the
 * pacts it named, {@code @PactVerifyProvider} is {@link ProvidesMessage}, and a WireMock stub is a
 * provider on the JDK's HTTP server answering as the stub did.
 *
 * <p>Every pact is a fixture of S1.1's corpus, where today's verifier and {@code janus verify} reach
 * one verdict per interaction. Each class is run here as a build would run it, and each interaction's
 * test must reach that verdict.
 */
class PactJvmExamplesTest {

  // --- the ported tests ---------------------------------------------------------------------------

  /** ContractTest: three pacts, states returning the ticket id a generator reads, a request filter. */
  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class ContractTest {
    static Stub provider;

    @RegisterExtension
    static final JanusProviderExtension janus = JanusProviderExtension.of(() -> Verifier
        .forProvider(provider.url())
        .withPacts(List.of(pact("examples/pact-jvm/contract-test/contract.json"),
            pact("examples/pact-jvm/contract-test/contract2.json"),
            pact("examples/pact-jvm/contract-test/contract-v3.json")))
        // pact-jvm's testTemplate added this header to the request: a request filter.
        .requestFilter(request -> with(request, "X-ContractTest", "true")));

    @BeforeAll
    static void setUpService() {
      provider = Stub.start(exchange -> "GET".equals(exchange.method()) && "/data".equals(exchange.path())
          && "true".equals(exchange.header("X-ContractTest"))
          && exchange.query().matches("ticketId=(0000|1234|99987)")
          ? new Stub.Reply(204, Map.of("Location", provider.url() + "/ticket/1234", "X-Ticket-ID", "1234"), null)
          : null);
    }

    @AfterAll
    static void stop() {
      provider.close();
    }

    @TestFactory
    Stream<DynamicTest> testTemplate() {
      return janus.interactions(this);
    }

    @State("default")
    public Map<String, Object> toDefaultState() {
      return Map.of("ticketId", "1234");
    }

    @State("state 2")
    public Map<String, Object> toSecondState(Map<String, Object> params) {
      return Map.of("ticketId", "99987");
    }

    @State(value = "default", action = State.Action.TEARDOWN)
    public void toDefaultStateAfter() {}

    @State(value = "state 2", action = State.Action.TEARDOWN)
    public void toSecondStateAfter(Map<String, Object> params) {}
  }

  /**
   * ProviderStateInjectedTest: the state's values generate the request body and the expected
   * LOCATION header. pact-jvm generates the expected header too and passes; today's verifier, and
   * Janus, compare the recorded one and fail it (the fixture records both).
   */
  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class ProviderStateInjectedTest {
    static Stub provider;
    static final List<Object> valueB = new ArrayList<>();

    @RegisterExtension
    static final JanusProviderExtension janus = JanusProviderExtension.of(() -> Verifier
        .forProvider(provider.url())
        .withPacts(List.of(pact("examples/pact-jvm/provider-state-injected/pact.json"))));

    @BeforeAll
    static void start() {
      provider = Stub.start(exchange -> "POST".equals(exchange.method()) && "/values".equals(exchange.path())
          ? new Stub.Reply(200, Map.of("Location", "http://server/users/666554433", "content-type", "application/json"),
              "{\"userId\": 666554433,\"userName\": \"Test\"}")
          : null);
    }

    @AfterAll
    static void stop() {
      provider.close();
    }

    @TestFactory
    Stream<DynamicTest> testTemplate() {
      return janus.interactions(this);
    }

    @State("a provider state with injectable values")
    public Map<String, Object> defaultState(Map<String, Object> params) {
      valueB.add(params.get("valueB"));
      return Map.of("userId", 666554433);
    }
  }

  /** ProviderStateParametersInjectedTest: the request path is generated from the state's parameter. */
  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class ProviderStateParametersInjectedTest {
    static Stub provider;

    @RegisterExtension
    static final JanusProviderExtension janus = JanusProviderExtension.of(() -> Verifier
        .forProvider(provider.url())
        .withPacts(List.of(pact("examples/pact-jvm/provider-state-parameters/pact.json"))));

    @BeforeAll
    static void start() {
      provider = Stub.start(exchange -> "/api/hello/John".equals(exchange.path())
          ? new Stub.Reply(200, Map.of("content-type", "application/json"), "{\"name\": \"John\"}")
          : null);
    }

    @AfterAll
    static void stop() {
      provider.close();
    }

    @TestFactory
    Stream<DynamicTest> testTemplate() {
      return janus.interactions(this);
    }

    @State("User exists")
    public Map<String, Object> defaultState(Map<String, Object> params) {
      return Map.of();
    }
  }

  /** StateInterface1 and StateInterface2: @State default methods the test class inherits. */
  interface StateInterface1 {
    @State("state1")
    default void toState1() {
      server().serve("/data");
    }

    Stub server();
  }

  interface StateInterface2 {
    @State("state2")
    default void toState2() {
      server().serve("/moreData");
    }

    Stub server();
  }

  /** StateAnnotationsOnInterfaceTest: each state, set up by an interface's method, stubs a path. */
  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class StateAnnotationsOnInterfaceTest implements StateInterface1, StateInterface2 {
    static Stub provider;

    @RegisterExtension
    static final JanusProviderExtension janus = JanusProviderExtension.of(() -> Verifier
        .forProvider(provider.url())
        .withPacts(List.of(pact("examples/pact-jvm/state-annotations-on-interface/pact.json"))));

    @BeforeAll
    static void start() {
      provider = Stub.start(exchange -> provider.served(exchange.path()) ? new Stub.Reply(204, Map.of(), null) : null);
    }

    @AfterAll
    static void stop() {
      provider.close();
    }

    @TestFactory
    Stream<DynamicTest> testTemplate() {
      return janus.interactions(this);
    }

    @Override
    public Stub server() {
      return provider;
    }
  }

  /** AmqpContractTest: v3 and v4 asynchronous messages, produced by methods returning JSON text. */
  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class AmqpContractTest {
    @RegisterExtension
    static final JanusProviderExtension janus = JanusProviderExtension.of(() -> Verifier
        // pact-jvm's MessageTestTarget: there is no HTTP provider to reach.
        .forProvider("http://127.0.0.1:1")
        .withPacts(List.of(pact("messages/pact-jvm-v3/pact.json"), pact("examples/pact-jvm/amqp-v4/pact.json"))));

    @TestFactory
    Stream<DynamicTest> testTemplate() {
      return janus.interactions(this);
    }

    @State("SomeProviderState")
    public void someProviderState() {}

    @ProvidesMessage("a V3 test message")
    public String verifyMessageForOrder() {
      return "{\"testParam1\": \"value1\",\"testParam2\": \"value2\"}";
    }

    @ProvidesMessage("a V4 test message")
    public String verifyV4MessageForOrder() {
      return "{\"testParam1\": \"value1\",\"testParam2\": \"value2\"}";
    }
  }

  /** MessageWithMetadataTest: a message's bytes and its metadata (pact-jvm's MessageAndMetadata). */
  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class MessageWithMetadataTest {
    @RegisterExtension
    static final JanusProviderExtension janus = JanusProviderExtension.of(() -> Verifier
        .forProvider("http://127.0.0.1:1")
        .withPacts(List.of(pact("messages/pact-jvm-v4-metadata/pact.json"))));

    @TestFactory
    Stream<DynamicTest> testTemplate() {
      return janus.interactions(this);
    }

    @ProvidesMessage("A message with metadata")
    public Message verifyV4MessageWithMetadataForOrder() {
      return Message.of("{\"someField\": \"someValue\"}".getBytes(StandardCharsets.UTF_8))
          .withMetadata(Map.of("someKey", "different string pact but same type"));
    }
  }

  /** SynchronousMessageContractTest: a synchronous message, whose producer receives the request. */
  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class SynchronousMessageContractTest {
    static final List<Object> requests = new ArrayList<>();

    @RegisterExtension
    static final JanusProviderExtension janus = JanusProviderExtension.of(() -> Verifier
        .forProvider("http://127.0.0.1:1")
        .withPacts(List.of(pact("messages/pact-jvm-synchronous/pact.json"))));

    @TestFactory
    Stream<DynamicTest> testTemplate() {
      return janus.interactions(this);
    }

    @ProvidesMessage("a test message")
    public String verifyMessageForOrder(Map<String, Object> request) {
      requests.add(request.get("request"));
      return "{\"name\": \"Fred\", \"testParam1\": \"value1\",\"testParam2\": \"value2\"}";
    }
  }

  // --- each ported class, run as a build runs it, against S1.1's verdicts ---------------------------

  @Test
  void contractTestVerifiesEachPactThroughItsStatesAndFilter() {
    assertEquals(Map.of(
        "anotherService: Get data", Status.SUCCESSFUL,
        "anotherService2: Get data 2", Status.SUCCESSFUL,
        "anotherService3: Get data", Status.SUCCESSFUL), tests(ContractTest.class));
  }

  @Test
  void providerStateInjectedTestFailsOnTheRecordedLocationAsTodaysVerifierDoes() {
    ProviderStateInjectedTest.valueB.clear();
    List<Harness.Outcome> outcomes = Harness.run(ProviderStateInjectedTest.class);
    Harness.Outcome test = Harness.named(outcomes, "V3Consumer: a request");
    assertEquals(Status.FAILED, test.status());
    assertTrue(test.error().getMessage().contains("$.response.headers.location"), test.error().getMessage());
    assertEquals(List.of(100), ProviderStateInjectedTest.valueB, "the state's parameters reached the method");
  }

  @Test
  void providerStateParametersInjectedTestGeneratesThePathFromTheParameter() {
    assertEquals(Map.of("SomeConsumer: Hello John", Status.SUCCESSFUL), tests(ProviderStateParametersInjectedTest.class));
  }

  @Test
  void stateAnnotationsOnInterfacesSetUpEachState() {
    assertEquals(Map.of(
        "consumerWithMultipleInteractions: Get data", Status.SUCCESSFUL,
        "consumerWithMultipleInteractions: GET moreData", Status.SUCCESSFUL), tests(StateAnnotationsOnInterfaceTest.class));
  }

  @Test
  void amqpContractTestProducesEachMessage() {
    assertEquals(Map.of(
        "test_consumer: a V3 test message", Status.SUCCESSFUL,
        "test_consumer: a V4 test message", Status.SUCCESSFUL), tests(AmqpContractTest.class));
  }

  @Test
  void messageWithMetadataTestComparesTheMetadata() {
    assertEquals(Map.of("test_consumer: A message with metadata", Status.SUCCESSFUL), tests(MessageWithMetadataTest.class));
  }

  @Test
  void synchronousMessageContractTestReceivesTheRequest() {
    SynchronousMessageContractTest.requests.clear();
    assertEquals(Map.of("KafkaRequestReplyConsumer: a test message", Status.SUCCESSFUL),
        tests(SynchronousMessageContractTest.class));
    assertEquals(1, SynchronousMessageContractTest.requests.size());
    assertTrue(String.valueOf(SynchronousMessageContractTest.requests.get(0)).contains("abcd123"),
        String.valueOf(SynchronousMessageContractTest.requests.get(0)));
  }

  // --- support ------------------------------------------------------------------------------------

  /** Each test's name and how it ended, failing on a container that did not succeed. */
  private static Map<String, Status> tests(Class<?> testClass) {
    List<Harness.Outcome> outcomes = Harness.run(testClass);
    outcomes.stream().filter(Harness.Outcome::container).forEach(container ->
        assertEquals(Status.SUCCESSFUL, container.status(), container.name() + ": " + container.error()));
    Map<String, Status> tests = Harness.tests(outcomes).stream()
        .collect(Collectors.toMap(Harness.Outcome::name, Harness.Outcome::status, (a, b) -> a, LinkedHashMap::new));
    Harness.tests(outcomes).stream().filter(o -> o.status() != Status.SUCCESSFUL)
        .forEach(o -> System.err.println(o.name() + ": " + o.error()));
    return tests;
  }

  /** A pact in S1.1's corpus, by its path below corpora/verifier. */
  static Path pact(String path) {
    return Path.of(System.getProperty("janus.conformance.suite")).getParent().resolve("corpora/verifier").resolve(path);
  }

  /** The request with one more header. */
  @SuppressWarnings("unchecked")
  static Map<String, Object> with(Map<String, Object> request, String name, String value) {
    Map<String, Object> headers = new LinkedHashMap<>(request.get("headers") instanceof Map<?, ?> h ? (Map<String, Object>) h : Map.of());
    headers.put(name, List.of(value));
    return Map.of("headers", headers);
  }

  /** A provider on the JDK's HTTP server, in place of the WireMock stubs pact-jvm's tests use. */
  static final class Stub implements AutoCloseable {
    record Exchange(String method, String path, String query, Map<String, List<String>> headers) {
      String header(String name) {
        return headers.entrySet().stream().filter(e -> e.getKey().equalsIgnoreCase(name))
            .map(e -> e.getValue().get(0)).findFirst().orElse(null);
      }
    }

    record Reply(int status, Map<String, String> headers, String body) {}

    private final HttpServer server;
    private final Set<String> served = ConcurrentHashMap.newKeySet();

    private Stub(HttpServer server) {
      this.server = server;
    }

    static Stub start(Function<Exchange, Reply> answer) {
      try {
        HttpServer server = HttpServer.create(new InetSocketAddress(InetAddress.getByName("127.0.0.1"), 0), 0);
        server.createContext("/", http -> reply(http, answer.apply(new Exchange(http.getRequestMethod(),
            http.getRequestURI().getPath(), String.valueOf(http.getRequestURI().getRawQuery()), http.getRequestHeaders()))));
        server.start();
        return new Stub(server);
      } catch (IOException e) {
        throw new java.io.UncheckedIOException(e);
      }
    }

    private static void reply(HttpExchange http, Reply reply) throws IOException {
      http.getRequestBody().readAllBytes();
      Reply answered = reply == null ? new Reply(404, Map.of(), null) : reply;
      answered.headers().forEach((name, value) -> http.getResponseHeaders().add(name, value));
      byte[] body = answered.body() == null ? new byte[0] : answered.body().getBytes(StandardCharsets.UTF_8);
      http.sendResponseHeaders(answered.status(), body.length == 0 ? -1 : body.length);
      try (OutputStream out = http.getResponseBody()) {
        out.write(body);
      }
    }

    String url() {
      return "http://127.0.0.1:" + server.getAddress().getPort();
    }

    /** Answers {@code path} from now on, as a WireMock stub registered by a state would. */
    void serve(String path) {
      served.add(path);
    }

    boolean served(String path) {
      return served.contains(path);
    }

    @Override
    public void close() {
      server.stop(0);
    }
  }
}
