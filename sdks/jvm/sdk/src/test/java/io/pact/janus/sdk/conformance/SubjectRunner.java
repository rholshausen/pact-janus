package io.pact.janus.sdk.conformance;

import static org.junit.platform.engine.discovery.DiscoverySelectors.selectClass;

import ch.qos.logback.classic.Logger;
import ch.qos.logback.classic.spi.ILoggingEvent;
import ch.qos.logback.core.read.ListAppender;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.PrintStream;
import java.io.StringWriter;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Stream;
import javax.tools.JavaCompiler;
import javax.tools.ToolProvider;
import org.junit.platform.engine.TestExecutionResult;
import org.junit.platform.engine.support.descriptor.MethodSource;
import org.junit.platform.launcher.Launcher;
import org.junit.platform.launcher.LauncherDiscoveryRequest;
import org.junit.platform.launcher.TestExecutionListener;
import org.junit.platform.launcher.TestIdentifier;
import org.junit.platform.launcher.core.LauncherDiscoveryRequestBuilder;
import org.junit.platform.launcher.core.LauncherFactory;
import org.slf4j.LoggerFactory;

/**
 * Runs a case's subject (ADR 0076 decision 1): a JUnit Jupiter test class written in Java, beside the
 * case, for a spelling data cannot carry — pact-jvm's, through the facade, or a refusal of one of the
 * JVM's own types. It is compiled against the test class path and run through the JUnit Platform, as
 * a build runs it, with {@code pact.rootDir} naming a scratch contract directory, where a pact-jvm
 * user names theirs. What came of it is checked against the case's data: how each test ended, the
 * contracts written, what was printed, and the engine's log. It decides nothing a case means.
 */
final class SubjectRunner {
  /** The logger the engine's events go to (ADR 0061). */
  private static final String ENGINE_LOGGER = "io.pact.janus.engine";

  private SubjectRunner() {}

  /** One test or container the run finished, by display name. */
  record Outcome(String name, boolean container, String error) {}

  static void run(JsonNode testCase, List<String> failures) throws Exception {
    String file = testCase.path("subjects").path(Suite.LANGUAGE).asText(null);
    if (file == null) {
      // ADR 0076 decision 5: a case with subjects runs in every language it is for, and this one has none.
      failures.add("the case names no JVM subject, and it is a case for this language: "
          + Suite.show(testCase.path("subjects")));
      return;
    }
    Path source = Suite.root().resolve("cases").resolve(testCase.path("category").asText()).resolve(file);
    Path work = Files.createTempDirectory("janus-subject-");
    Path classes = Files.createDirectories(work.resolve("classes"));
    Path contracts = Files.createDirectories(work.resolve("contracts"));
    String problems = compile(source, classes);
    if (problems != null) {
      failures.add("the subject did not compile:\n" + problems);
      return;
    }

    ByteArrayOutputStream stdout = new ByteArrayOutputStream();
    ByteArrayOutputStream stderr = new ByteArrayOutputStream();
    Logger engine = (Logger) LoggerFactory.getLogger(ENGINE_LOGGER);
    ListAppender<ILoggingEvent> logged = new ListAppender<>();
    List<Outcome> outcomes;
    PrintStream out = System.out;
    PrintStream err = System.err;
    String rootDir = System.getProperty("pact.rootDir");
    try (URLClassLoader loader = new URLClassLoader(new URL[] {classes.toUri().toURL()},
        SubjectRunner.class.getClassLoader())) {
      Class<?> subject = loader.loadClass(file.substring(0, file.length() - ".java".length()));
      logged.start();
      engine.addAppender(logged);
      System.setOut(new PrintStream(stdout, true, StandardCharsets.UTF_8));
      System.setErr(new PrintStream(stderr, true, StandardCharsets.UTF_8));
      System.setProperty("pact.rootDir", contracts.toString());
      outcomes = launch(subject);
    } finally {
      System.setOut(out);
      System.setErr(err);
      engine.detachAppender(logged);
      if (rootDir == null) {
        System.clearProperty("pact.rootDir");
      } else {
        System.setProperty("pact.rootDir", rootDir);
      }
    }

    JsonNode expect = testCase.path("expect");
    checkTests(expect, outcomes, failures);
    checkContracts(testCase, contracts, failures);
    for (String stream : List.of("stdout", "stderr")) {
      String text = (stream.equals("stdout") ? stdout : stderr).toString(StandardCharsets.UTF_8);
      for (JsonNode part : expect.path("output").path(stream)) {
        if (!text.contains(part.asText())) {
          failures.add("the run's " + stream + " does not hold " + Suite.show(part) + ":\n" + text);
        }
      }
    }
    String log = String.join("\n", logged.list.stream()
        .map(event -> event.getLevel() + " " + event.getFormattedMessage()).toList());
    for (JsonNode part : expect.path("engine-log")) {
      if (!log.contains(part.asText())) {
        failures.add("the engine's log does not hold " + Suite.show(part) + ":\n" + log);
      }
    }
  }

  /** Compiles the subject into {@code classes}; the compiler's diagnostics when it does not compile. */
  static String compile(Path source, Path classes) {
    JavaCompiler compiler = ToolProvider.getSystemJavaCompiler();
    if (compiler == null) {
      return "no Java compiler: the JVM suite runs on a JDK (ADR 0076)";
    }
    StringWriter diagnostics = new StringWriter();
    var files = compiler.getStandardFileManager(null, null, StandardCharsets.UTF_8);
    var task = compiler.getTask(diagnostics, files, null,
        List.of("-d", classes.toString(), "-classpath", System.getProperty("java.class.path"), "-parameters",
            "-encoding", "UTF-8", "-proc:none"),
        null, files.getJavaFileObjects(source.toFile()));
    return Boolean.TRUE.equals(task.call()) ? null : diagnostics.toString();
  }

  /** Runs the class as a build would, recording what happened to every test and container. */
  private static List<Outcome> launch(Class<?> subject) {
    List<Outcome> outcomes = Collections.synchronizedList(new ArrayList<>());
    LauncherDiscoveryRequest request = LauncherDiscoveryRequestBuilder.request()
        .selectors(selectClass(subject))
        .build();
    Launcher launcher = LauncherFactory.create();
    launcher.execute(request, new TestExecutionListener() {
      @Override
      public void executionFinished(TestIdentifier id, TestExecutionResult result) {
        String error = result.getStatus() == TestExecutionResult.Status.SUCCESSFUL ? null
            : result.getThrowable().map(SubjectRunner::message).orElse(result.getStatus().toString());
        outcomes.add(new Outcome(name(id), id.isContainer(), error));
      }

      @Override
      public void executionSkipped(TestIdentifier id, String reason) {
        outcomes.add(new Outcome(name(id), id.isContainer(), "skipped: " + reason));
      }
    });
    return outcomes;
  }

  /** A test's name: a method's own, without the parameter types JUnit displays; a dynamic test's display name. */
  static String name(TestIdentifier id) {
    return id.getSource().filter(MethodSource.class::isInstance).map(source -> ((MethodSource) source).getMethodName())
        .filter(method -> id.getDisplayName().startsWith(method + "("))
        .orElse(id.getDisplayName());
  }

  /** A failure's message, with each cause's beneath it: a JUnit extension wraps what it reports. */
  static String message(Throwable error) {
    StringBuilder text = new StringBuilder(String.valueOf(error));
    for (Throwable cause = error.getCause(); cause != null && cause != error; cause = cause.getCause()) {
      text.append("\ncaused by: ").append(cause);
    }
    return text.toString();
  }

  static void checkTests(JsonNode expect, List<Outcome> outcomes, List<String> failures) {
    Map<String, Outcome> tests = new TreeMap<>();
    for (Outcome outcome : outcomes) {
      if (!outcome.container()) {
        tests.put(outcome.name(), outcome);
      }
    }
    List<String> named = new ArrayList<>();
    for (JsonNode wanted : expect.path("tests")) {
      String name = wanted.path("test").asText();
      named.add(name);
      Outcome outcome = tests.get(name);
      if (outcome == null) {
        failures.add("the subject ran no test '" + name + "'; it ran " + tests.keySet());
        continue;
      }
      JsonNode fails = wanted.get("fails");
      if (fails == null && outcome.error() != null) {
        failures.add("'" + name + "' failed, and the case expects it to pass: " + outcome.error());
      } else if (fails != null && outcome.error() == null) {
        failures.add("'" + name + "' passed, and the case expects it to fail with " + Suite.show(fails));
      } else if (fails != null) {
        for (JsonNode part : fails) {
          if (!outcome.error().contains(part.asText())) {
            failures.add("'" + name + "' failed without saying " + Suite.show(part) + ": " + outcome.error());
          }
        }
      }
    }
    for (String name : tests.keySet()) {
      if (!named.contains(name)) {
        failures.add("the subject ran a test '" + name + "' the case does not name");
      }
    }
    // The class's container: its own failure, beyond its tests' (a JUnit extension's afterAll).
    JsonNode suite = expect.path("suite").get("fails");
    List<String> errors = outcomes.stream().filter(o -> o.container() && o.error() != null)
        .map(Outcome::error).toList();
    if (suite == null) {
      errors.forEach(error -> failures.add("the subject failed, beyond its tests: " + error));
    } else if (errors.isEmpty()) {
      failures.add("the subject did not fail, and the case expects it to, with " + Suite.show(suite));
    } else {
      for (JsonNode part : suite) {
        if (errors.stream().noneMatch(error -> error.contains(part.asText()))) {
          failures.add("the subject failed without saying " + Suite.show(part) + ": " + errors);
        }
      }
    }
  }

  /** The contracts the run wrote, each file against the case's {@code contracts}; one it does not name fails it. */
  static void checkContracts(JsonNode testCase, Path directory, List<String> failures) throws IOException {
    Map<String, JsonNode> written = new TreeMap<>();
    // A framework's run writes into the project's contracts, which a run that wrote nothing has not made.
    try (Stream<Path> files = Files.isDirectory(directory) ? Files.list(directory) : Stream.empty()) {
      for (Path file : files.filter(p -> p.toString().endsWith(".json")).toList()) {
        written.put(file.getFileName().toString(), Suite.MAPPER.readTree(Files.readString(file)));
      }
    }
    record(testCase, written);
    JsonNode expected = testCase.path("expect").get("contracts");
    if (expected == null) {
      return;
    }
    for (String name : written.keySet()) {
      if (!expected.has(name)) {
        failures.add("the run wrote '" + name + "', which the case does not name");
      }
    }
    for (var contract : expected.properties()) {
      JsonNode document = written.get(contract.getKey());
      boolean wanted = contract.getValue().path("written").asBoolean(true);
      if (wanted != (document != null)) {
        failures.add("'" + contract.getKey() + "' was " + (document != null ? "" : "not ") + "written, and the case expects it "
            + (wanted ? "" : "not ") + "to be");
      }
      if (contract.getValue().has("content") && document != null) {
        Suite.checkContent(contract.getValue().get("content"), document, "'" + contract.getKey() + "'", failures);
      }
    }
  }

  /**
   * Record mode (conformance README §6): with {@code JANUS_CONFORMANCE_RECORD} naming a directory,
   * writes each contract's content in the form a case's {@code contracts} expects, to be read and
   * reviewed against the row before it is checked in.
   */
  private static void record(JsonNode testCase, Map<String, JsonNode> written) throws IOException {
    String directory = System.getenv("JANUS_CONFORMANCE_RECORD");
    if (directory == null || written.isEmpty()) {
      return;
    }
    ObjectNode contracts = Suite.MAPPER.createObjectNode();
    written.forEach((name, document) -> contracts.putObject(name).set("content", Suite.contentOf(document)));
    Path file = Files.createDirectories(Path.of(directory))
        .resolve(testCase.path("id").asText().replace('/', '-') + ".json");
    Files.writeString(file, Suite.MAPPER.writerWithDefaultPrettyPrinter().writeValueAsString(contracts) + "\n");
  }
}
