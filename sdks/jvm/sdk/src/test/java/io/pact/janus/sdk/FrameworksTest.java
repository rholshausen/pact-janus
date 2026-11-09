package io.pact.janus.sdk;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.junit.platform.engine.TestExecutionResult.Status;

/**
 * What the conformance suite's framework cases (ADR 0078) cannot show, since it needs more than one
 * test class: classes recording one consumer and provider, run at once as a build runs them, merge into
 * one contract, and a second class recording an interaction differently fails (S2.11, ADR 0065). The
 * classes are {@code io.pact.janus.sdk.frameworks}, which use only the SDK's public API.
 */
class FrameworksTest {
  private static final Path CONTRACTS = Path.of(System.getProperty("janus.test.contracts"));
  private static final String PACKAGE = "io.pact.janus.sdk.frameworks.";

  private static Path contract(String consumer) {
    return CONTRACTS.resolve(consumer + "-orders-api.janus.json");
  }

  private static Status status(List<Harness.Outcome> outcomes, String name) {
    return Harness.named(outcomes, name).status();
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
}
