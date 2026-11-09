package io.pact.janus.sdk.conformance;

import static org.junit.platform.engine.discovery.DiscoverySelectors.selectClass;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.platform.engine.TestExecutionResult;
import org.junit.platform.launcher.TestExecutionListener;
import org.junit.platform.launcher.TestIdentifier;
import org.junit.platform.launcher.core.LauncherDiscoveryRequestBuilder;
import org.junit.platform.launcher.core.LauncherFactory;

/**
 * The test JVM a framework case's subject runs in (ADR 0078 decision 3), as Gradle's test worker runs
 * a project's tests: the JUnit Platform's launcher, whose Jupiter and Spock engines discover the class,
 * in the project directory. It writes what happened to every test and container as JSON, for
 * {@link FrameworkRunner} to check, and exits, whatever the engine's threads are doing.
 *
 * <p>Usage: {@code FrameworkLauncher <class> <outcomes.json>}.
 */
public final class FrameworkLauncher {
  private FrameworkLauncher() {}

  public static void main(String[] args) throws Exception {
    ArrayNode outcomes = Suite.MAPPER.createArrayNode();
    LauncherFactory.create().execute(
        LauncherDiscoveryRequestBuilder.request().selectors(selectClass(args[0])).build(),
        new TestExecutionListener() {
          @Override
          public void executionFinished(TestIdentifier id, TestExecutionResult result) {
            record(id, result.getStatus() == TestExecutionResult.Status.SUCCESSFUL ? null
                : result.getThrowable().map(SubjectRunner::message).orElse(result.getStatus().toString()));
          }

          @Override
          public void executionSkipped(TestIdentifier id, String reason) {
            record(id, "skipped: " + reason);
          }

          private synchronized void record(TestIdentifier id, String error) {
            ObjectNode outcome = outcomes.addObject();
            outcome.put("name", SubjectRunner.name(id));
            outcome.put("container", id.isContainer());
            outcome.put("error", error);
          }
        });
    Files.writeString(Path.of(args[1]), Suite.MAPPER.writeValueAsString(outcomes));
    System.exit(0);
  }
}
