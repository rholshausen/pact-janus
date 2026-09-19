package io.pact.janus.sdk;

import static org.junit.platform.engine.discovery.DiscoverySelectors.selectClass;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Supplier;
import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.platform.engine.TestExecutionResult;
import org.junit.platform.launcher.Launcher;
import org.junit.platform.launcher.LauncherDiscoveryRequest;
import org.junit.platform.launcher.TestExecutionListener;
import org.junit.platform.launcher.TestIdentifier;
import org.junit.platform.launcher.core.LauncherDiscoveryRequestBuilder;
import org.junit.platform.launcher.core.LauncherFactory;

/**
 * Runs a JUnit test class the way a build would, and records what happened to every test and
 * container — so the JUnit integration itself can be tested: that {@code finalise} runs after a
 * failing test, that a withheld contract fails the class, that each variant is its own test.
 *
 * <p>Classes it runs carry {@code @EnabledIfSystemProperty(named = Harness.PROPERTY, ...)}, so the
 * build's own test run skips them.
 */
final class Harness {
  static final String PROPERTY = "janus.harness";

  /** The fixture the class being run picks up when it initialises. */
  static volatile Janus janus;

  record Outcome(String name, boolean container, TestExecutionResult.Status status, Throwable error) {}

  static List<Outcome> run(Class<?> testClass) {
    List<Outcome> outcomes = new ArrayList<>();
    LauncherDiscoveryRequest request = LauncherDiscoveryRequestBuilder.request()
        .selectors(selectClass(testClass))
        .configurationParameter("junit.jupiter.conditions.deactivate", "")
        .build();
    Launcher launcher = LauncherFactory.create();
    System.setProperty(PROPERTY, "true");
    try {
      launcher.execute(request, new TestExecutionListener() {
        @Override
        public void executionFinished(TestIdentifier id, TestExecutionResult result) {
          outcomes.add(new Outcome(id.getDisplayName(), id.isContainer(), result.getStatus(),
              result.getThrowable().orElse(null)));
        }
      });
    } finally {
      System.clearProperty(PROPERTY);
    }
    return outcomes;
  }

  /**
   * The extension a harnessed class registers: {@link JanusExtension#afterAll}, on an extension the
   * class builds in {@code @BeforeAll} — a static initialiser would run while the build merely
   * discovers the class, long before this harness has a fixture for it.
   */
  static AfterAllCallback finaliser(Supplier<JanusExtension> extension) {
    return context -> {
      JanusExtension janus = extension.get();
      if (janus != null) {
        janus.afterAll(context);
      }
    };
  }

  static Outcome named(List<Outcome> outcomes, String name) {
    return outcomes.stream().filter(o -> o.name().equals(name) || o.name().endsWith("$" + name)).findFirst()
        .orElseThrow(() -> new AssertionError("no outcome named '" + name + "' in " + outcomes));
  }

  static List<Outcome> tests(List<Outcome> outcomes) {
    return outcomes.stream().filter(o -> !o.container()).toList();
  }

  private Harness() {}
}
