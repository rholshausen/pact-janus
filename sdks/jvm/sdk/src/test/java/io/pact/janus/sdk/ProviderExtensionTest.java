package io.pact.janus.sdk;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestFactory;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;
import org.junit.jupiter.api.extension.RegisterExtension;
import org.junit.platform.engine.TestExecutionResult.Status;

/**
 * {@link JanusProviderExtension}'s report (ADR 0056 decision 3): one test per interaction result,
 * a pending failure aborted, and a last test when the run's status says what no result did. The
 * engine is scripted; the ported pact-jvm examples run the real one.
 */
class ProviderExtensionTest {
  static volatile ScriptedEngine engine;

  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class Suite {
    @RegisterExtension
    static final JanusProviderExtension janus = JanusProviderExtension.of(() -> Verifier
        .forProvider("http://127.0.0.1:8080")
        .withUrls(Map.of("urls", List.of("https://broker.example/pacts/1")))
        .withEmbedding(engine));

    @TestFactory
    Stream<DynamicTest> pacts() {
      return janus.interactions(this);
    }
  }

  private static String result(String description, String variant, String status, boolean pending) {
    return "{\"kind\":\"verification/interaction-result\",\"payload\":{\"interaction\":{\"contract\":"
        + "{\"consumer\":\"web-app\",\"provider\":\"orders-api\"},\"index\":0,\"description\":\"" + description + "\"},"
        + "\"variant\":\"" + variant + "\",\"status\":\"" + status + "\",\"pending\":" + pending
        + ",\"mismatches\":[{\"path\":\"$.response.status\",\"message\":\"Expected 200 but was 500\"}]}}";
  }

  private static ScriptedEngine scripted(String summaryStatus, String... results) {
    ScriptedEngine scripted = new ScriptedEngine();
    scripted.ok("verification/verify", "{\"session\":\"vs-1\",\"stream\":\"s-1\"}");
    String finished = "{\"kind\":\"verification/finished\",\"last\":true,\"payload\":{\"status\":\"" + summaryStatus + "\"}}";
    scripted.ok("events/poll", "{\"events\":[" + String.join(",", results) + (results.length > 0 ? "," : "") + finished + "]}");
    return scripted;
  }

  private static Map<String, Status> run() {
    List<Harness.Outcome> outcomes = Harness.run(Suite.class);
    return Harness.tests(outcomes).stream()
        .collect(java.util.stream.Collectors.toMap(Harness.Outcome::name, Harness.Outcome::status, (a, b) -> a,
            java.util.LinkedHashMap::new));
  }

  @Test
  void eachResultIsATestAndAPendingFailureIsAborted() {
    engine = scripted("verified",
        result("get an order", "base", "verified", false),
        result("get an order", "status=404", "failed", true));
    assertEquals(Map.of(
        "web-app: get an order", Status.SUCCESSFUL,
        "web-app: get an order [status=404]", Status.ABORTED), run());
    assertEquals(1, engine.closed, "the run's engine is ended when the stream closes");
  }

  @Test
  void aFailedResultFailsItsTestWithItsMismatches() {
    engine = scripted("failed", result("get an order", "base", "failed", false));
    List<Harness.Outcome> outcomes = Harness.run(Suite.class);
    Harness.Outcome test = Harness.named(outcomes, "web-app: get an order");
    assertEquals(Status.FAILED, test.status());
    assertTrue(test.error().getMessage().contains("$.response.status Expected 200 but was 500"), test.error().getMessage());
    assertEquals(1, Harness.tests(outcomes).size(), "no last test: a result already shows the failure");
  }

  @Test
  void aFailedRunWithNoFailedResultEndsWithAFailedTest() {
    engine = scripted("failed");
    assertEquals(Map.of("verification", Status.FAILED), run());
  }

  @Test
  void aFilteredRunEndsWithAnAbortedTest() {
    engine = scripted("filtered", result("get an order", "base", "verified", false));
    assertEquals(Map.of(
        "web-app: get an order", Status.SUCCESSFUL,
        "verification", Status.ABORTED), run());
  }
}
