package io.pact.janus.sdk;

import static io.pact.janus.sdk.Shapes.map;
import static io.pact.janus.sdk.Shapes.oneOf;
import static io.pact.janus.sdk.Shapes.regex;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestFactory;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;
import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.RegisterExtension;
import org.junit.jupiter.api.io.TempDir;
import org.junit.platform.engine.TestExecutionResult;

/**
 * End to end against the real {@code janus-engine} (the build compiles it first and names it in
 * {@code JANUS_ENGINE}). Each test names the behavioural-spec conformance ids it covers.
 */
class EndToEndTest {
  private static final ObjectMapper MAPPER = new ObjectMapper();
  // HTTP/1.1: see the FINDING test below.
  private static final HttpClient HTTP = HttpClient.newBuilder().version(HttpClient.Version.HTTP_1_1).build();
  @TempDir Path contracts;

  private Janus janus() {
    return Janus.of("web-app", "orders-api", JanusOptions.defaults().withContractDirectory(contracts));
  }

  private static JanusExtension extension(Janus janus) {
    return JanusExtension.of(janus);
  }

  @Test
  @DisplayName("the RFC example runs every variant the engine selects and writes a contract recording exactly those [session.execute.closure-per-selected-variant, session.finalise.writes-contract-when-verified, shape.optional.presence-dimension, shape.any-of.value-dimension, shape.one-of.alternative-dimension, shape.each-like.cardinality-dimension, shape.any-of.literal-containment, shape.regex.unanchored]")
  void rfcExample() throws Exception {
    Janus janus = janus();
    Interaction getOrder = OrderConsumerTest.getOrder(extension(janus));
    List<Variant> ran = new ArrayList<>();
    Set<String> statuses = new LinkedHashSet<>();
    Set<String> paymentTypes = new LinkedHashSet<>();
    int[] shippedAtAbsent = {0};
    janus.execute(getOrder, (mock, variant) -> {
      OrderClient.Order order = OrderClient.at(mock.url()).getOrder("42");
      assertTrue(order.lineCount() > 0);
      ran.add(variant);
      statuses.add(order.status());
      paymentTypes.add(order.payment().type());
      // the mock serves what the armed variant says
      boolean absent = "absent".equals(variant.point("response.body.shippedAt#presence"));
      assertEquals(absent, order.shippedAt().isEmpty(), variant.displayName());
      if (absent) {
        shippedAtAbsent[0]++;
      }
      assertEquals(variant.point("response.body.items#cardinality").equals("min") ? 1 : 2, order.lineCount());
    });
    Path file = janus.finalise().orElseThrow();

    // the engine's dimensions, derived from the nodes the SDK emitted
    Set<String> dimensions = new LinkedHashSet<>();
    ran.forEach(v -> v.assignment().forEach(p -> dimensions.add(p.dimension())));
    assertEquals(Set.of("response.body.items#cardinality", "response.body.payment#alternative",
        "response.body.shippedAt#presence", "response.body.status#value"), dimensions);
    assertEquals("base", ran.get(0).id());
    assertTrue(shippedAtAbsent[0] > 0);
    assertEquals(Set.of("PENDING", "SHIPPED", "DELIVERED"), statuses);
    assertEquals(Set.of("card", "invoice"), paymentTypes);

    assertEquals(contracts.resolve("web-app-orders-api.janus.json"), file);
    byte[] bytes = Files.readAllBytes(file);
    String text = new String(bytes, StandardCharsets.UTF_8);
    assertTrue(text.startsWith("{\"$format\":"), text);
    assertTrue(text.endsWith("}\n") && !text.endsWith("\n\n"), "one trailing LF");
    assertEquals(1, text.chars().filter(c -> c == '\n').count(), "compact: the only newline is the trailing one");
    JsonNode contract = MAPPER.readTree(bytes);
    JsonNode interaction = contract.at("/interactions/0");
    assertEquals("get an order", interaction.get("description").asText());
    List<String> recorded = new ArrayList<>();
    interaction.at("/selection/variants").forEach(v -> recorded.add(v.get("id").asText()));
    assertEquals(ran.stream().map(Variant::id).toList(), recorded);
    // the shape recorded is the shape submitted
    assertEquals(MAPPER.readTree(MAPPER.writeValueAsString(getOrder.toDocument().get("parts"))), interaction.get("parts"));
  }

  @Test
  @DisplayName("a consumer that mishandles shippedAt being absent fails naming those variants, and gets no contract [session.execute.failing-variant-fails-build, session.execute.every-variant-runs-after-a-failure, session.finalise.failed-test-withholds-contract]")
  void carelessConsumer() {
    Janus janus = janus();
    Interaction getOrder = OrderConsumerTest.getOrder(extension(janus));
    List<Variant> ran = new ArrayList<>();
    ExecuteFailedException failed = assertThrows(ExecuteFailedException.class, () -> janus.execute(getOrder, (mock, variant) -> {
      ran.add(variant);
      OrderClient.careless(mock.url()).getOrder("42");
    }));

    List<Variant> expectedFailures = ran.stream()
        .filter(v -> "absent".equals(v.point("response.body.shippedAt#presence"))).toList();
    assertFalse(expectedFailures.isEmpty());
    assertEquals(expectedFailures, failed.failures().stream().map(VariantFailure::variant).toList());
    failed.failures().forEach(f -> assertInstanceOf(NullPointerException.class, f.cause()));
    assertEquals(ran.size(), failed.variantCount());
    assertTrue(failed.getMessage().contains("shippedAt=absent [response.body.shippedAt#presence=absent]"), failed.getMessage());

    ContractWithheldException withheld = assertThrows(ContractWithheldException.class, janus::finalise);
    assertFalse(withheld.engineWithheld(), "every exchange verified; it is the consumer's own test that failed");
    assertTrue(withheld.getMessage().contains("shippedAt=absent"), withheld.getMessage());
    assertFalse(Files.exists(contracts.resolve("web-app-orders-api.janus.json")));
  }

  @Test
  @DisplayName("the engine rejects a one-of whose alternatives share a discriminator literal, and the problems arrive verbatim [shape.one-of.engine-validates, session.response.bare-value-is-equality]")
  void engineValidatesOneOf() {
    Janus janus = janus();
    Interaction ambiguous = janus.interaction("ambiguous payment")
        .request(r -> r.method("GET").path("/payments/1"))
        .response(r -> r.status(200).body(oneOf("type", map(
            "card", map("type", "card"),
            "voucher", map("type", "card")))));
    try {
      JanusEngineException error = assertThrows(JanusEngineException.class,
          () -> janus.execute(ambiguous, (mock, variant) -> {}));
      assertEquals("interaction-invalid", error.code());
      assertEquals("/parts/response/body/alternatives", error.problems().get(0).pointer());
      assertTrue(error.problems().get(0).message().contains("disjoint"), error.problems().toString());
    } finally {
      assertThrows(ContractWithheldException.class, janus::finalise);
    }
  }

  @Test
  @DisplayName("a request the armed variant does not match is the engine's verdict, reported by finalise with its mismatches [session.finalise.withheld-contract-fails-build]")
  void engineWithholdsOnMismatch() {
    Janus janus = janus();
    Interaction ping = janus.interaction("ping").request(r -> r.method("GET").path("/ping")).response(r -> r.status(204));
    // the closure swallows the mock's failure, so only the engine knows
    janus.execute(ping, (mock, variant) -> send(mock.uri("/pong"), "GET", null));
    ContractWithheldException withheld = assertThrows(ContractWithheldException.class, janus::finalise);
    assertTrue(withheld.engineWithheld());
    assertEquals("failed", withheld.results().get(0).getStatus());
    assertTrue(withheld.getMessage().contains("'ping': failed"), withheld.getMessage());
    assertTrue(withheld.getMessage().contains("/pong"), withheld.getMessage());
    assertFalse(Files.exists(contracts.resolve("web-app-orders-api.janus.json")));
  }

  @Test
  @DisplayName("lower-cased header names and as-written query names are what the real transport matches [session.request.header-names-lower-cased, session.request.query-names-as-written, session.request.multi-value-slots, session.given.multiple-states]")
  void headersAndQuery() throws Exception {
    Janus janus = janus();
    Interaction search = janus.interaction("search orders")
        .given("orders exist")
        .given("the caller is authorised", map("key", "k-1"))
        .request(r -> r.method("GET").path("/orders")
            .query("Page", "2")
            .query("tag", List.of("red", "blue"))
            .header("X-Api-Key", "k-1"))
        .response(r -> r.status(200).header("Content-Type", "application/json").body(map("orders", List.of())));
    janus.execute(search, (mock, variant) -> {
      HttpResponse<String> response = send(mock.uri("/orders?Page=2&tag=red&tag=blue"), "GET", "k-1");
      assertEquals(200, response.statusCode(), response.body());
    });
    Path file = janus.finalise().orElseThrow();
    JsonNode states = MAPPER.readTree(file.toFile()).at("/interactions/0/states");
    assertEquals(2, states.size());
    assertFalse(states.get(0).has("params"));
  }

  @Test
  @DisplayName("a shape helper as a header value means exactly one value, so a client sending the header once gets a contract [session.request.shape-value-is-exactly-one]")
  void helperHeaderValueIsExactlyOneValue() {
    Janus janus = janus();
    Interaction traced = janus.interaction("traced ping")
        .request(r -> r.method("GET").path("/ping").header("X-Trace", regex("^[0-9a-f]+$", "abc123")))
        .response(r -> r.status(204));
    List<String> ids = new ArrayList<>();
    janus.execute(traced, (mock, variant) -> {
      ids.add(variant.id());
      // a client that sends the header once, as nearly every client would
      send(mock.uri("/ping"), "GET", null, "X-Trace", "abc123");
    });
    // Bounded at one, the each-like's cardinality has a single point: no request-side variant.
    assertEquals(List.of("base"), ids);
    assertTrue(janus.finalise().isPresent());
  }

  @Test
  @DisplayName("the engine records an equality over null with its example [shape.literal.scalar-is-equality]")
  void nullEquality() throws Exception {
    Janus janus = janus();
    Interaction nothing = janus.interaction("a null note")
        .request(r -> r.method("GET").path("/note"))
        .response(r -> r.status(200).body(map("note", null)));
    janus.execute(nothing, (mock, variant) -> {
      HttpResponse<String> response = send(mock.uri("/note"), "GET", null);
      assertEquals(MAPPER.readTree("{\"note\":null}"), MAPPER.readTree(response.body()));
    });
    JsonNode contract = MAPPER.readTree(janus.finalise().orElseThrow().toFile());
    JsonNode note = contract.at("/interactions/0/parts/response/body/members/note");
    assertTrue(note.has("example") && note.get("example").isNull(), note.toString());
  }

  @Test
  @DisplayName("FINDING: the mock serves none of the response headers the interaction declares, and labels no JSON body application/json")
  void declaredResponseHeadersAreNotServed() {
    Janus janus = janus();
    Interaction labelled = janus.interaction("labelled")
        .request(r -> r.method("GET").path("/labelled"))
        .response(r -> r.status(200).header("Content-Type", "application/json").header("X-Served", "yes")
            .body(map("a", 1)));
    janus.execute(labelled, (mock, variant) -> {
      HttpResponse<String> response = send(mock.uri("/labelled"), "GET", null);
      assertEquals(200, response.statusCode());
      assertTrue(response.headers().firstValue("content-type").isEmpty(), response.headers().map().toString());
      assertTrue(response.headers().firstValue("x-served").isEmpty(), response.headers().map().toString());
    });
    assertTrue(janus.finalise().isPresent());
  }

  @Test
  @DisplayName("FINDING: the engine's mock never answers the JDK HttpClient's default request, which offers an HTTP/2 cleartext upgrade; the engine still counts the variant verified")
  void defaultJdkHttpClientIsNeverAnswered() throws Exception {
    Janus janus = janus();
    Interaction ping = janus.interaction("ping").request(r -> r.method("GET").path("/ping")).response(r -> r.status(204));
    HttpClient jdkDefault = HttpClient.newHttpClient();
    List<Throwable> outcomes = new ArrayList<>();
    janus.execute(ping, (mock, variant) -> {
      try {
        jdkDefault.send(HttpRequest.newBuilder(mock.uri("/ping")).timeout(Duration.ofSeconds(2)).GET().build(),
            HttpResponse.BodyHandlers.ofString());
        outcomes.add(null);
      } catch (java.net.http.HttpTimeoutException e) {
        outcomes.add(e);
      }
    });
    assertEquals(1, outcomes.size());
    assertInstanceOf(java.net.http.HttpTimeoutException.class, outcomes.get(0));
    // the consumer never saw a response, yet the exchange verified and a contract is produced
    assertTrue(janus.finalise().isPresent());
  }

  // ---------------------------------------------------------------------------------------------
  // The @TestFactory form, against the real engine.

  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class OrderVariantsSuite {
    static JanusExtension janus;
    @RegisterExtension static final AfterAllCallback finalise = Harness.finaliser(() -> janus);

    @BeforeAll
    static void configure() {
      janus = JanusExtension.of(Harness.janus);
    }

    @TestFactory
    Stream<DynamicTest> getsAnOrder() {
      return janus.variants(OrderConsumerTest.getOrder(janus), (mock, variant) -> {
        OrderClient.Order order = OrderClient.careless(mock.url()).getOrder("42");
        assertTrue(order.lineCount() > 0);
      });
    }
  }

  @Test
  @DisplayName("as a @TestFactory every selected variant is a named test; the careless consumer's shippedAt=absent variants fail on their own and the class fails for the withheld contract [session.execute.closure-per-selected-variant, session.finalise.always-runs, session.finalise.failed-test-withholds-contract]")
  void perVariantTestsAgainstTheEngine() {
    Harness.janus = janus();
    List<Harness.Outcome> outcomes = Harness.run(OrderVariantsSuite.class);
    List<Harness.Outcome> tests = Harness.tests(outcomes);
    assertEquals(8, tests.size(), tests.toString());
    assertEquals("base", tests.get(0).name());
    for (Harness.Outcome test : tests) {
      boolean absent = test.name().contains("shippedAt=absent");
      assertEquals(absent ? TestExecutionResult.Status.FAILED : TestExecutionResult.Status.SUCCESSFUL, test.status(), test.name());
    }
    assertInstanceOf(ContractWithheldException.class, Harness.named(outcomes, "OrderVariantsSuite").error());
    assertFalse(Files.exists(contracts.resolve("web-app-orders-api.janus.json")));
  }

  // ---------------------------------------------------------------------------------------------

  private static HttpResponse<String> send(URI uri, String method, String apiKey, String... headers) {
    HttpRequest.Builder request = HttpRequest.newBuilder(uri).timeout(Duration.ofSeconds(10))
        .method(method, HttpRequest.BodyPublishers.noBody());
    if (apiKey != null) {
      request.header("X-Api-Key", apiKey);
    }
    for (int i = 0; i < headers.length; i += 2) {
      request.header(headers[i], headers[i + 1]);
    }
    try {
      return HTTP.send(request.build(), HttpResponse.BodyHandlers.ofString());
    } catch (Exception e) {
      throw new IllegalStateException(e);
    }
  }
}
