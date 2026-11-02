package io.pact.janus.sdk;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.EnumSource;
import org.junit.platform.engine.TestExecutionResult.Status;

/**
 * The test-framework integrations (STYLE.md, "Test-framework integration"; staged plan S2.4b): the
 * suites in {@code io.pact.janus.sdk.frameworks}, which use only the SDK's public API, run under JUnit 5
 * and under Spock as a build runs them, against the real engine. A variant that fails, fails the test
 * that ran it; a withheld contract fails the suite, and is not written; a provider test passes.
 */
class FrameworksTest {
  private static final Path CONTRACTS = Path.of(System.getProperty("janus.test.contracts"));
  private static final String PACKAGE = "io.pact.janus.sdk.frameworks.";

  /** Each framework's three suites, the names it gives their tests, and its consumers' names. */
  enum Framework {
    JUNIT5("junit5", "JUnitConsumerTest", "JUnitWithheldTest", "JUnitProviderTest",
        "getsAnOrder()", "readsWhenAnOrderShipped()", "deletesAnOrder()",
        List.of("UserConsumer: a GET request for all users", "UserConsumer: a GET request for user 1")),
    SPOCK("spock", "SpockConsumerSpec", "SpockWithheldSpec", "SpockProviderSpec",
        "gets an order", "reads when an order shipped", "deletes an order",
        List.of("satisfies UserConsumer's pact"));

    final String consumer;
    final String consumerSuite;
    final String withheldSuite;
    final String providerSuite;
    final String getsAnOrder;
    final String readsShipped;
    final String deletes;
    final List<String> providerTests;

    Framework(String consumer, String consumerSuite, String withheldSuite, String providerSuite, String getsAnOrder,
        String readsShipped, String deletes, List<String> providerTests) {
      this.consumer = consumer;
      this.consumerSuite = consumerSuite;
      this.withheldSuite = withheldSuite;
      this.providerSuite = providerSuite;
      this.getsAnOrder = getsAnOrder;
      this.readsShipped = readsShipped;
      this.deletes = deletes;
      this.providerTests = providerTests;
    }
  }

  private static final Map<Framework, Map<String, List<Harness.Outcome>>> RUNS = new LinkedHashMap<>();

  private static Path contract(String consumer) {
    return CONTRACTS.resolve(consumer + "-orders-api.janus.json");
  }

  @BeforeAll
  static void runEverySuite() throws IOException {
    for (Framework framework : Framework.values()) {
      Files.deleteIfExists(contract(framework.consumer));
      Files.deleteIfExists(contract(framework.consumer + "-careless"));
      Map<String, List<Harness.Outcome>> runs = new LinkedHashMap<>();
      for (String suite : List.of(framework.consumerSuite, framework.withheldSuite, framework.providerSuite)) {
        runs.put(suite, Harness.run(PACKAGE + suite));
      }
      RUNS.put(framework, runs);
    }
  }

  private static Status status(List<Harness.Outcome> outcomes, String name) {
    return Harness.named(outcomes, name).status();
  }

  @ParameterizedTest
  @EnumSource(Framework.class)
  void aConsumerSuitePassesAndWritesItsContract(Framework framework) throws IOException {
    List<Harness.Outcome> run = RUNS.get(framework).get(framework.consumerSuite);
    assertEquals(Status.SUCCESSFUL, status(run, framework.getsAnOrder), run::toString);
    assertEquals(Status.SUCCESSFUL, status(run, framework.consumerSuite), run::toString);
    assertTrue(Files.readString(contract(framework.consumer)).contains("\"get an order\""));
  }

  @ParameterizedTest
  @EnumSource(Framework.class)
  void aFailingVariantFailsTheTestThatRanItAndOnlyThatOne(Framework framework) {
    List<Harness.Outcome> run = RUNS.get(framework).get(framework.withheldSuite);
    Harness.Outcome failed = Harness.named(run, framework.readsShipped);
    assertEquals(Status.FAILED, failed.status(), run::toString);
    // Spock reports an exception from a call in an `expect:` block as the condition's, caused by it.
    Throwable error = failed.error();
    while (!(error instanceof ExecuteFailedException) && error.getCause() != null) {
      error = error.getCause();
    }
    assertInstanceOf(ExecuteFailedException.class, error, failed.error()::toString);
    assertTrue(error.getMessage().contains("shippedAt=absent"), error::getMessage);
    assertEquals(Status.SUCCESSFUL, status(run, framework.deletes), run::toString);
  }

  @ParameterizedTest
  @EnumSource(Framework.class)
  void aWithheldContractFailsTheSuiteAndIsNotWritten(Framework framework) {
    List<Harness.Outcome> run = RUNS.get(framework).get(framework.withheldSuite);
    Harness.Outcome suite = Harness.named(run, framework.withheldSuite);
    assertEquals(Status.FAILED, suite.status(), run::toString);
    assertInstanceOf(ContractWithheldException.class, suite.error());
    assertFalse(Files.exists(contract(framework.consumer + "-careless")));
  }

  @ParameterizedTest
  @EnumSource(Framework.class)
  void aProviderTestPasses(Framework framework) {
    List<Harness.Outcome> run = RUNS.get(framework).get(framework.providerSuite);
    assertEquals(framework.providerTests, Harness.tests(run).stream().map(Harness.Outcome::name).toList());
    for (Harness.Outcome test : Harness.tests(run)) {
      assertEquals(Status.SUCCESSFUL, test.status(), run::toString);
    }
    assertEquals(Status.SUCCESSFUL, status(run, framework.providerSuite), run::toString);
  }

  // Staged plan S2.11, ADR 0065: test classes run at once, each with an engine of its own, merge
  // into one contract file.
  @Test
  void twoClassesRunAtOnceMergeIntoOneContractTheBytesOneClassRecordingBothWrites() throws IOException {
    Path merged = contract("junit5-merged");
    Path together = CONTRACTS.resolve("together").resolve("junit5-merged-orders-api.janus.json");
    Files.deleteIfExists(merged);
    Files.deleteIfExists(together);
    List<Harness.Outcome> run = Harness.runAtOnce(PACKAGE + "JUnitMergedOrdersTest", PACKAGE + "JUnitMergedDeletesTest");
    assertEquals(Status.SUCCESSFUL, status(run, "JUnitMergedOrdersTest"), run::toString);
    assertEquals(Status.SUCCESSFUL, status(run, "JUnitMergedDeletesTest"), run::toString);
    List<Harness.Outcome> one = Harness.run(PACKAGE + "JUnitMergedTogetherTest");
    assertEquals(Status.SUCCESSFUL, status(one, "JUnitMergedTogetherTest"), one::toString);

    String text = Files.readString(merged);
    assertEquals(Files.readString(together), text);
    assertTrue(text.indexOf("\"delete an order\"") < text.indexOf("\"get an order\""), text);
  }

  @Test
  void theSecondClassToRecordAnInteractionDifferentlyFailsNamingIt() throws IOException {
    Files.deleteIfExists(contract("junit5-conflicting"));
    List<Harness.Outcome> run = Harness.runAtOnce(PACKAGE + "JUnitConflictOkTest", PACKAGE + "JUnitConflictAcceptedTest");
    List<Harness.Outcome> classes = List.of(Harness.named(run, "JUnitConflictOkTest"),
        Harness.named(run, "JUnitConflictAcceptedTest"));
    assertEquals(List.of(Status.SUCCESSFUL, Status.FAILED),
        classes.stream().map(Harness.Outcome::status).sorted().toList(), run::toString);
    Harness.Outcome failed = classes.stream().filter(c -> c.status() == Status.FAILED).findFirst().orElseThrow();
    JanusEngineException error = assertInstanceOf(JanusEngineException.class, failed.error());
    assertEquals("contract-conflict", error.code());
    assertTrue(error.getMessage().contains("'create an order'"), error::getMessage);
  }

  @Test
  void bothFrameworksWriteTheSameContract() throws IOException {
    String junit = Files.readString(contract("junit5")).replace("\"junit5\"", "\"web-app\"");
    String spock = Files.readString(contract("spock")).replace("\"spock\"", "\"web-app\"");
    assertEquals(junit, spock);
  }
}
