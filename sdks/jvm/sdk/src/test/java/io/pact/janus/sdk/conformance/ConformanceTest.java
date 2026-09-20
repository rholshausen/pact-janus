package io.pact.janus.sdk.conformance;

import static org.junit.jupiter.api.Assertions.fail;
import static org.junit.jupiter.api.DynamicTest.dynamicTest;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.stream.Stream;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.TestFactory;

/**
 * The SDK conformance suite (plan task 6.4, SDK spec §7, ADR 0017), run on the JVM: every case
 * under {@code conformance/cases}, one dynamic test each, against this SDK and — for the
 * {@code live} cases — the engine {@code JANUS_ENGINE} names. The cases are the same files the
 * TypeScript driver runs; what is the JVM's here is only how the DSL is spelled ({@link Dsl}).
 *
 * <p>The run writes a report the suite checker reads, so "this SDK is conformant" is a claim the
 * build makes, not one this class asserts in prose.
 */
class ConformanceTest {
  private static final List<Suite.Result> RESULTS = new ArrayList<>();

  @TestFactory
  @DisplayName("the SDK conformance suite")
  Stream<DynamicTest> suite() {
    return Suite.cases().stream().map(testCase -> dynamicTest(displayName(testCase), () -> {
      Suite.Result result = CaseRunner.run(testCase);
      RESULTS.add(result);
      if (!"passed".equals(result.status())) {
        fail(testCase.path("id").asText() + " — " + testCase.path("title").asText() + "\n" + result.detail());
      }
    }));
  }

  @AfterAll
  static void writeReport() throws IOException {
    // Every case's outcome, including the ones that failed: a report missing a case is itself a
    // finding, and the checker says so.
    System.out.println("conformance report: " + Suite.writeReport(RESULTS));
  }

  private static String displayName(JsonNode testCase) {
    List<String> covers = new ArrayList<>();
    testCase.path("covers").forEach(id -> covers.add(id.asText()));
    return testCase.path("id").asText() + ": " + testCase.path("title").asText() + " [" + String.join(", ", covers) + "]";
  }
}
