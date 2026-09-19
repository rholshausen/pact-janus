package io.pact.janus.sdk;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
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
 * SDK spec §7 category 3 — variant iteration, against a scripted in-memory engine. Each test names
 * the behavioural-spec conformance ids it covers.
 */
class VariantIterationTest {
  private final ScriptedEngine engine = new ScriptedEngine();
  @TempDir java.nio.file.Path contracts;

  private Janus janus() {
    return Janus.of("web-app", "orders-api",
        JanusOptions.defaults().withEmbedding(engine).withContractDirectory(contracts));
  }

  private static Interaction ping(Janus janus) {
    return janus.interaction("ping").request(r -> r.method("GET").path("/ping")).response(r -> r.status(204));
  }

  @Test
  @DisplayName("the closure runs once per selected variant, in the engine's order, with that variant armed and its descriptor as sent [session.execute.closure-per-selected-variant]")
  void oncePerSelectedVariant() {
    engine.variants("base", "b", "a");
    Janus janus = janus();
    List<String> seen = new ArrayList<>();
    janus.execute(ping(janus), (mock, variant) -> {
      // the variant under test is the one armed last
      assertEquals(variant.id(), engine.served().get(engine.served().size() - 1));
      assertEquals("http://127.0.0.1:1", mock.url());
      assertEquals(1, mock.endpoint().get("port"));
      seen.add(variant.id() + "|" + variant.label() + "|" + variant.origin());
    });
    assertEquals(List.of("base|v:base|base", "b|v:b|covering", "a|v:a|covering"), seen);
  }

  @Test
  @DisplayName("a one-variant selection still runs the closure once [session.execute.single-variant-still-runs]")
  void singleVariantStillRuns() {
    engine.variants("base");
    Janus janus = janus();
    int[] runs = {0};
    janus.execute(ping(janus), (mock, variant) -> runs[0]++);
    assertEquals(1, runs[0]);
    assertEquals(List.of("base"), engine.served());
  }

  @Test
  @DisplayName("a failing variant fails execute, naming it by label and id with its cause [session.execute.failing-variant-fails-build]")
  void failingVariantFailsBuild() {
    engine.variants("base", "shippedAt=absent");
    Janus janus = janus();
    IOException cause = new IOException("could not parse the order");
    ExecuteFailedException error = assertThrows(ExecuteFailedException.class, () -> janus.execute(ping(janus), (mock, variant) -> {
      if (!variant.id().equals("base")) {
        throw cause;
      }
    }));
    assertEquals(1, error.failures().size());
    assertEquals("shippedAt=absent", error.failures().get(0).variant().id());
    assertSame(cause, error.getCause());
    assertTrue(error.getMessage().contains("'ping' failed on 1 of 2 variants"), error.getMessage());
    assertTrue(error.getMessage().contains("v:shippedAt=absent [shippedAt=absent]: java.io.IOException: could not parse the order"),
        error.getMessage());
  }

  @Test
  @DisplayName("every remaining variant still runs after a failure, and execute fails once naming them all [session.execute.every-variant-runs-after-a-failure]")
  void everyVariantRunsAfterAFailure() {
    engine.variants("base", "one", "two", "three");
    Janus janus = janus();
    List<String> ran = new ArrayList<>();
    ExecuteFailedException error = assertThrows(ExecuteFailedException.class, () -> janus.execute(ping(janus), (mock, variant) -> {
      ran.add(variant.id());
      if (variant.id().equals("base") || variant.id().equals("two")) {
        throw new AssertionError("failed on " + variant.id());
      }
    }));
    assertEquals(List.of("base", "one", "two", "three"), ran);
    assertEquals(List.of("base", "one", "two", "three"), engine.served());
    assertEquals(List.of("base", "two"), error.failures().stream().map(f -> f.variant().id()).toList());
    assertEquals(1, error.getSuppressed().length);
  }

  // ---------------------------------------------------------------------------------------------
  // The @TestFactory form: one test per selected variant.

  @EnabledIfSystemProperty(named = Harness.PROPERTY, matches = "true")
  static class PerVariantSuite {
    static JanusExtension janus;
    @RegisterExtension static final AfterAllCallback finalise = Harness.finaliser(() -> janus);

    @BeforeAll
    static void configure() {
      janus = JanusExtension.of(Harness.janus);
    }

    @TestFactory
    Stream<DynamicTest> ping() {
      return janus.variants(VariantIterationTest.ping(janus.janus()), (mock, variant) -> {
        if (variant.id().equals("items=min+1")) {
          throw new AssertionError("only ever reads the first item");
        }
      });
    }
  }

  @Test
  @DisplayName("as a @TestFactory each selected variant is its own named test, a failure is reported on its own test, and still withholds the contract [session.execute.closure-per-selected-variant, session.execute.failing-variant-fails-build, session.finalise.failed-test-withholds-contract]")
  void perVariantTests() {
    engine.variants("base", "items=min+1", "shippedAt=absent");
    Harness.janus = janus();
    List<Harness.Outcome> outcomes = Harness.run(PerVariantSuite.class);

    List<Harness.Outcome> tests = Harness.tests(outcomes);
    assertEquals(List.of("v:base [base]", "v:items=min+1 [items=min+1]", "v:shippedAt=absent [shippedAt=absent]"),
        tests.stream().map(Harness.Outcome::name).toList());
    assertEquals(List.of(TestExecutionResult.Status.SUCCESSFUL, TestExecutionResult.Status.FAILED,
        TestExecutionResult.Status.SUCCESSFUL), tests.stream().map(Harness.Outcome::status).toList());
    assertEquals("only ever reads the first item", tests.get(1).error().getMessage());
    assertInstanceOf(ContractWithheldException.class, Harness.named(outcomes, "PerVariantSuite").error());
  }
}
