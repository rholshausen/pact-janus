package io.pact.janus.sdk;

import static io.pact.janus.sdk.Shapes.map;
import static io.pact.janus.sdk.Shapes.optional;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;
import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.RegisterExtension;
import org.junit.jupiter.api.io.TempDir;
import org.junit.platform.engine.TestExecutionResult;

/**
 * SDK spec §7 category 2 — session lifecycle and call sequence, against a scripted in-memory
 * engine. Each test names the behavioural-spec conformance ids it covers.
 */
class SessionLifecycleTest {
  private static final ObjectMapper MAPPER = new ObjectMapper();
  private final ScriptedEngine engine = new ScriptedEngine();
  @TempDir Path contracts;

  private Janus janus() {
    return Janus.of("web-app", "orders-api",
        JanusOptions.defaults().withEmbedding(engine).withContractDirectory(contracts));
  }

  private static Interaction order(Janus janus) {
    return janus.interaction("get an order")
        .given("an order exists", map("id", "42"))
        .request(r -> r.method("GET").path("/orders/42"))
        .response(r -> r.status(200).body(map("id", 42, "shippedAt", optional("x"))));
  }

  @Test
  @DisplayName("configuring, and building an interaction, make no protocol call [session.janus.no-call-until-execute, session.interaction.no-call-until-execute]")
  void noCallUntilExecute() {
    Janus janus = janus();
    order(janus).toSpec();
    assertEquals(0, engine.opened);
    assertTrue(engine.calls.isEmpty());
  }

  @Test
  @DisplayName("the first execute starts the engine and offers protocol version 1 [session.janus.hello-offers-v1]")
  void helloOffersV1() throws Exception {
    Janus janus = janus();
    janus.execute(order(janus), (mock, variant) -> {});
    assertEquals(1, engine.opened);
    assertEquals("engine/hello", engine.calls.get(0).op());
    assertEquals(MAPPER.readTree("[1]"), engine.body("engine/hello").get("protocol-versions"));
    assertEquals("janus-jvm", engine.body("engine/hello").at("/host/name").asText());
  }

  @Test
  @DisplayName("a hello the engine refuses fails the first execute with details.supported verbatim, and is not retried [janus errors: protocol-version-unsupported]")
  void helloRefused() {
    engine.error("engine/hello", "{\"code\":\"protocol-version-unsupported\",\"category\":\"protocol\","
        + "\"message\":\"no common version\",\"details\":{\"supported\":[2,3]}}");
    Janus janus = janus();
    JanusEngineException error = assertThrows(JanusEngineException.class,
        () -> janus.execute(order(janus), (mock, variant) -> {}));
    assertEquals("protocol-version-unsupported", error.code());
    assertEquals(List.of(2, 3), error.supported());
    assertEquals(List.of("engine/hello"), engine.ops());
    assertEquals(1, engine.closed);
    assertEquals(Optional.empty(), janus.finalise());
    assertEquals(1, engine.opened);
  }

  @Test
  @DisplayName("execute makes the full call sequence, and finalise ends it [session.execute.full-call-sequence]")
  void fullCallSequence() throws Exception {
    engine.variants("base", "shippedAt-absent");
    Janus janus = janus();
    Interaction interaction = order(janus);
    janus.execute(interaction, (mock, variant) -> {});
    janus.finalise();

    assertEquals(List.of("engine/hello", "consumer-session/create", "consumer-session/add-interaction",
        "consumer-session/start-transport", "consumer-session/variants", "consumer-session/serve-variant",
        "consumer-session/report-variant", "consumer-session/serve-variant", "consumer-session/report-variant",
        "consumer-session/finalise"), engine.ops());
    // `events`: this test has an SLF4J provider, so what the mock does has somewhere to be logged.
    assertEquals(MAPPER.readTree("{\"events\":true,\"config\":{\"consumer\":{\"name\":\"web-app\"},\"provider\":{\"name\":\"orders-api\"}}}"),
        engine.body("consumer-session/create"));
    assertEquals(MAPPER.readTree(MAPPER.writeValueAsString(interaction.toDocument())),
        engine.body("consumer-session/add-interaction").get("interaction"));
    assertEquals("cs-1", engine.body("consumer-session/add-interaction").get("session").asText());
    assertEquals("http", engine.body("consumer-session/start-transport").get("transport").asText());
    assertEquals(MAPPER.readTree("{\"session\":\"cs-1\",\"handle\":\"i-1\"}"), engine.body("consumer-session/variants"));
    assertEquals(List.of("base", "shippedAt-absent"), engine.served());
    assertEquals(MAPPER.readTree("{\"session\":\"cs-1\",\"handle\":\"i-1\",\"variant\":\"base\",\"status\":\"passed\"}"),
        engine.body("consumer-session/report-variant"));
    assertEquals(MAPPER.readTree("{\"session\":\"cs-1\",\"write\":{\"path\":"
        + MAPPER.writeValueAsString(contracts.resolve("web-app-orders-api.janus.json").toAbsolutePath().toString())
        + ",\"mode\":\"merge\"}}"), engine.body("consumer-session/finalise"));
    assertEquals(1, engine.closed);
  }

  @Test
  @DisplayName("one session per suite: create and start-transport once, add-interaction per execute [session.execute.one-session-per-suite]")
  void oneSessionPerSuite() {
    Janus janus = janus();
    janus.execute(order(janus), (mock, variant) -> {});
    janus.execute(janus.interaction("list orders").request(r -> r.method("GET").path("/orders")), (mock, variant) -> {});
    janus.finalise();
    assertEquals(List.of("engine/hello", "consumer-session/create", "consumer-session/add-interaction",
        "consumer-session/start-transport", "consumer-session/variants", "consumer-session/serve-variant",
        "consumer-session/report-variant", "consumer-session/add-interaction", "consumer-session/variants",
        "consumer-session/serve-variant", "consumer-session/report-variant", "consumer-session/finalise"), engine.ops());
    assertEquals(1, engine.opened);
  }

  @Test
  @DisplayName("interaction-invalid is thrown before any variant runs, problems verbatim [request/response/one-of errors: interaction-invalid, shape.one-of.engine-validates]")
  void interactionInvalid() {
    engine.error("consumer-session/add-interaction", "{\"code\":\"interaction-invalid\",\"category\":\"document\","
        + "\"message\":\"interaction specification is not valid\",\"details\":{\"problems\":["
        + "{\"pointer\":\"/parts/response/body/alternatives\",\"message\":\"alternatives 'a' and 'b' do not have disjoint discriminator values\"}]}}");
    Janus janus = janus();
    List<Variant> ran = new ArrayList<>();
    JanusEngineException error = assertThrows(JanusEngineException.class,
        () -> janus.execute(order(janus), (mock, variant) -> ran.add(variant)));
    assertEquals("interaction-invalid", error.code());
    assertEquals("document", error.category());
    assertEquals(List.of(new JanusEngineException.Problem("/parts/response/body/alternatives",
        "alternatives 'a' and 'b' do not have disjoint discriminator values")), error.problems());
    assertTrue(error.getMessage().contains("/parts/response/body/alternatives: alternatives 'a' and 'b'"));
    assertTrue(ran.isEmpty());
    assertFalse(engine.ops().contains("consumer-session/variants"));
  }

  @Test
  @DisplayName("variant-budget-exceeded is thrown from variants before any variant runs [execute errors: variant-budget-exceeded]")
  void variantBudgetExceeded() {
    engine.error("consumer-session/variants", "{\"code\":\"variant-budget-exceeded\",\"category\":\"document\","
        + "\"message\":\"too many\",\"details\":{\"max-variants\":50}}");
    Janus janus = janus();
    List<Variant> ran = new ArrayList<>();
    JanusEngineException error = assertThrows(JanusEngineException.class,
        () -> janus.execute(order(janus), (mock, variant) -> ran.add(variant)));
    assertEquals("variant-budget-exceeded", error.code());
    assertEquals(50, error.details().get("max-variants"));
    assertTrue(ran.isEmpty());
    assertFalse(engine.ops().contains("consumer-session/serve-variant"));
  }

  @Test
  @DisplayName("finalise names the contract file for the engine to write, merging into it, and writes none itself [session.finalise.writes-contract-when-verified, session.finalise.names-contract-file]")
  void namesTheContractFile() throws Exception {
    engine.on("consumer-session/finalise", body -> "{\"ok\":{\"results\":[{\"handle\":\"i-1\",\"status\":\"verified\"}],"
        + "\"written\":" + body.path("write").path("path") + "}}");
    Janus janus = janus();
    janus.execute(order(janus), (mock, variant) -> {});
    Optional<Path> written = janus.finalise();

    Path expected = contracts.resolve("web-app-orders-api.janus.json").toAbsolutePath();
    assertEquals(MAPPER.readTree("{\"path\":" + MAPPER.writeValueAsString(expected.toString()) + ",\"mode\":\"merge\"}"),
        engine.body("consumer-session/finalise").get("write"));
    assertEquals(Optional.of(expected), written);
    // The engine's host writes the file (ADR 0065); this engine wrote none, and nor did the SDK.
    assertFalse(Files.exists(expected));
  }

  @Test
  @DisplayName("told to, finalise asks the engine to overwrite the contract file [session.finalise.names-contract-file]")
  void asksToOverwrite() {
    Janus janus = Janus.of("web-app", "orders-api",
        JanusOptions.defaults().withEmbedding(engine).withContractDirectory(contracts).withContractOverwrite(true));
    janus.execute(order(janus), (mock, variant) -> {});
    janus.finalise();
    assertEquals("overwrite", engine.body("consumer-session/finalise").path("write").path("mode").asText());
  }

  @Test
  @DisplayName("an engine that answers the contract instead of writing it fails finalise, saying it is too old, and writes nothing")
  void contractFromAnOldEngineFails() {
    engine.ok("consumer-session/finalise", "{\"results\":[{\"handle\":\"i-1\",\"status\":\"verified\"}],"
        + "\"contract-text\":\"{}\\n\"}");
    Janus janus = janus();
    janus.execute(order(janus), (mock, variant) -> {});
    JanusEmbeddingException error = assertThrows(JanusEmbeddingException.class, janus::finalise);
    assertTrue(error.getMessage().contains("predates 'contract-writing'"), error.getMessage());
    assertFalse(Files.exists(contracts.resolve("web-app-orders-api.janus.json")));
  }

  @Test
  @DisplayName("a withheld contract fails finalise, naming each unverified interaction and variant with the engine's mismatches, and writes nothing [session.finalise.withheld-contract-fails-build]")
  void withheldContractFails() {
    engine.variants("base", "shippedAt-absent", "items-min+1");
    engine.ok("consumer-session/finalise", "{\"results\":[{\"handle\":\"i-1\",\"status\":\"failed\",\"variants\":["
        + "{\"variant\":\"base\",\"status\":\"verified\"},"
        + "{\"variant\":\"shippedAt-absent\",\"status\":\"failed\",\"mismatches\":[{\"path\":\"$.request.path\",\"message\":\"Expected '/orders/42' but got '/orders/43'\"}]},"
        + "{\"variant\":\"items-min+1\",\"status\":\"not-exercised\"}]}]}");
    Janus janus = janus();
    janus.execute(order(janus), (mock, variant) -> {});
    ContractWithheldException error = assertThrows(ContractWithheldException.class, janus::finalise);

    assertEquals(List.of("get an order"), error.interactions());
    String message = error.getMessage();
    assertTrue(message.contains("'get an order': failed"), message);
    assertTrue(message.contains("v:shippedAt-absent [shippedAt-absent]: failed"), message);
    assertTrue(message.contains("$.request.path: Expected '/orders/42' but got '/orders/43'"), message);
    assertTrue(message.contains("v:items-min+1 [items-min+1]: not-exercised"), message);
    assertFalse(message.contains("v:base [base]"), message);
    assertFalse(Files.exists(contracts.resolve("web-app-orders-api.janus.json")));
    assertEquals(1, engine.closed);
  }

  @Test
  @DisplayName("a failed test is reported, and the engine withholds the contract [session.finalise.failed-test-withholds-contract, session.execute.reports-each-variant]")
  void failedTestWithholdsContract() {
    engine.variants("base", "shippedAt-absent");
    Janus janus = janus();
    assertThrows(ExecuteFailedException.class, () -> janus.execute(order(janus), (mock, variant) -> {
      if (variant.id().equals("shippedAt-absent")) {
        throw new IllegalStateException("shippedAt missing");
      }
    }));
    List<JsonNode> reports = engine.calls.stream().filter(c -> c.op().equals("consumer-session/report-variant"))
        .map(ScriptedEngine.Call::body).toList();
    assertEquals("passed", reports.get(0).path("status").asText());
    assertEquals("failed", reports.get(1).path("status").asText());
    assertEquals("java.lang.IllegalStateException: shippedAt missing", reports.get(1).path("reason").asText());

    ContractWithheldException error = assertThrows(ContractWithheldException.class, janus::finalise);
    assertEquals(List.of("get an order"), error.interactions());
    assertFalse(Files.exists(contracts.resolve("web-app-orders-api.janus.json")));
    assertTrue(engine.ops().contains("consumer-session/finalise"));
  }

  @Test
  @DisplayName("a variant the engine did not verify fails execute though its test passed [session.execute.engine-verdict-fails-the-variant]")
  void engineVerdictFailsTheVariant() {
    engine.variants("base", "shippedAt-absent");
    engine.on("consumer-session/report-variant", body -> body.path("variant").asText().equals("base")
        ? "{\"ok\":{\"result\":{\"variant\":\"base\",\"status\":\"failed\",\"mismatches\":[{\"path\":\"$.request.path\",\"message\":\"Expected '/orders/42' but got '/orders/7'\"}]}}}"
        : "{\"ok\":{\"result\":{\"variant\":\"shippedAt-absent\",\"status\":\"not-exercised\"}}}");
    Janus janus = janus();
    ExecuteFailedException error = assertThrows(ExecuteFailedException.class,
        () -> janus.execute(order(janus), (mock, variant) -> {}));
    assertEquals(List.of("base", "shippedAt-absent"), error.failures().stream().map(f -> f.variant().id()).toList());
    assertTrue(error.getMessage().contains("$.request.path: Expected '/orders/42' but got '/orders/7'"), error.getMessage());
    assertTrue(error.getMessage().contains("nothing exercised the mock under this variant"), error.getMessage());
  }

  @Test
  @DisplayName("an interaction the engine refused is named when it withholds the contract [session.finalise.refused-interaction-withholds-contract]")
  void refusedInteractionWithholdsContract() {
    engine.ok("consumer-session/finalise", "{\"results\":[{\"handle\":\"i-1\",\"status\":\"verified\"}],"
        + "\"refused\":[{\"description\":\"broken\",\"code\":\"interaction-invalid\",\"message\":\"bad\"}]}");
    Janus janus = janus();
    janus.execute(order(janus), (mock, variant) -> {});
    ContractWithheldException error = assertThrows(ContractWithheldException.class, janus::finalise);
    assertEquals(List.of("broken"), error.interactions());
    assertTrue(error.getMessage().contains("'broken': refused, bad [interaction-invalid]"), error.getMessage());
  }

  @Test
  @DisplayName("finalise with no open session ends the engine and does nothing else [finalise: no open session]")
  void finaliseWithoutSession() {
    engine.error("consumer-session/create", "{\"code\":\"internal\",\"category\":\"internal\",\"message\":\"boom\"}");
    Janus janus = janus();
    assertThrows(JanusEngineException.class, () -> janus.execute(order(janus), (mock, variant) -> {}));
    assertEquals(Optional.empty(), janus.finalise());
    assertFalse(engine.ops().contains("consumer-session/finalise"));
    assertEquals(1, engine.closed);
    // and never having started an engine, it starts none
    Janus idle = janus();
    assertEquals(Optional.empty(), idle.finalise());
    assertEquals(1, engine.opened);
  }

  // ---------------------------------------------------------------------------------------------
  // The JUnit integration runs finalise after the class's last test, whatever happened.

  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class AFailingSuite {
    static JanusExtension janus;
    @RegisterExtension static final AfterAllCallback finalise = Harness.finaliser(() -> janus);

    @BeforeAll
    static void configure() {
      janus = JanusExtension.of(Harness.janus);
    }

    @Test
    void passes() {
      janus.execute(order(janus.janus()), (mock, variant) -> {});
    }

    @Test
    void fails() {
      janus.execute(janus.interaction("list orders").request(r -> r.path("/orders")), (mock, variant) -> {
        throw new AssertionError("the client under test got it wrong");
      });
    }
  }

  @Test
  @DisplayName("the JUnit integration runs finalise after the last test even when one failed, and the withheld contract fails the class [session.finalise.always-runs]")
  void finaliseAlwaysRuns() {
    Harness.janus = janus();
    List<Harness.Outcome> outcomes = Harness.run(AFailingSuite.class);

    assertEquals(TestExecutionResult.Status.SUCCESSFUL, Harness.named(outcomes, "passes()").status());
    assertEquals(TestExecutionResult.Status.FAILED, Harness.named(outcomes, "fails()").status());
    assertInstanceOf(ExecuteFailedException.class, Harness.named(outcomes, "fails()").error());
    Harness.Outcome suite = Harness.named(outcomes, "AFailingSuite");
    assertEquals(TestExecutionResult.Status.FAILED, suite.status());
    assertInstanceOf(ContractWithheldException.class, suite.error());
    assertEquals("consumer-session/finalise", engine.ops().get(engine.ops().size() - 1));
    assertEquals(1, engine.closed);
  }

  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class APassingSuite {
    static JanusExtension janus;
    @RegisterExtension static final AfterAllCallback finalise = Harness.finaliser(() -> janus);

    @BeforeAll
    static void configure() {
      janus = JanusExtension.of(Harness.janus);
    }

    @Test
    void passes() {
      janus.execute(order(janus.janus()), (mock, variant) -> {});
    }
  }

  @Test
  @DisplayName("a passing suite's contract is written by the integration's finalise [session.finalise.always-runs, session.finalise.writes-contract-when-verified]")
  void passingSuiteWritesContract() {
    Harness.janus = janus();
    List<Harness.Outcome> outcomes = Harness.run(APassingSuite.class);
    assertEquals(TestExecutionResult.Status.SUCCESSFUL, Harness.named(outcomes, "APassingSuite").status());
    assertTrue(Files.exists(contracts.resolve("web-app-orders-api.janus.json")));
  }
}
