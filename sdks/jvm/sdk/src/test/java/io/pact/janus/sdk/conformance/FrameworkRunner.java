package io.pact.janus.sdk.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.concurrent.TimeUnit;
import java.util.stream.Stream;
import org.codehaus.groovy.control.CompilationFailedException;
import org.codehaus.groovy.control.CompilationUnit;
import org.codehaus.groovy.control.CompilerConfiguration;

/**
 * Runs a framework case (ADR 0078). Each of its subjects in this language is a JUnit Jupiter class or
 * a Spock specification. It is compiled against the test class path, by {@code javac} or by Groovy's
 * compiler, and run in a JVM of its own, forked with the project directory as its working directory.
 * There the JUnit Platform's launcher runs it ({@link FrameworkLauncher}), as Gradle's test worker
 * runs a project's tests. So the extension's {@code afterAll}, Spock's cleanup and the default contract
 * directory are the framework's and the project's, not this driver's. Each run is held to the case's
 * data, as an in-process subject's is ({@link SubjectRunner}). Every run of one case writes the same
 * contract, and its digest goes in the report. Nothing here decides what a case means.
 */
final class FrameworkRunner {
  /** How long one subject's JVM may run before it is ended and the case fails. */
  private static final long DEADLINE_SECONDS = 120;

  private FrameworkRunner() {}

  /** Runs every subject the case has in this language; the digest of the contract they wrote, or null. */
  static String run(JsonNode testCase, List<String> failures) throws Exception {
    Map<String, String> subjects = Suite.frameworkSubjects(testCase);
    if (subjects.isEmpty()) {
      failures.add("the case has no subject for a framework of this language");
      return null;
    }
    Map<String, String> digests = new TreeMap<>();
    for (var subject : subjects.entrySet()) {
      List<String> own = new ArrayList<>();
      Path project = runOne(testCase, subject.getValue(), own);
      if (project != null) {
        Path contracts = project.resolve("contracts");
        SubjectRunner.checkContracts(testCase, contracts, own);
        digests.putAll(digests(subject.getKey(), contracts));
      }
      own.forEach(line -> failures.add("[" + subject.getKey() + "] " + line));
    }
    // ADR 0078 decision 4: one case's subjects write one contract, whichever framework ran them.
    List<String> distinct = digests.values().stream().distinct().toList();
    if (distinct.size() > 1) {
      failures.add("the frameworks wrote different contracts: " + digests);
    }
    return distinct.size() == 1 ? distinct.get(0) : null;
  }

  /** Compiles the subject, sets its project up and runs it; the project directory, or null if it never ran. */
  private static Path runOne(JsonNode testCase, String file, List<String> failures) throws Exception {
    Path source = Suite.root().resolve("cases").resolve(testCase.path("category").asText()).resolve(file);
    Path work = Files.createTempDirectory("janus-framework-");
    Path classes = Files.createDirectories(work.resolve("classes"));
    Path project = Files.createDirectories(work.resolve("project"));
    String problems = file.endsWith(".groovy") ? compileGroovy(source, classes) : SubjectRunner.compile(source, classes);
    if (problems != null) {
      failures.add("the subject did not compile:\n" + problems);
      return null;
    }
    for (var entry : testCase.path("project").properties()) {
      Path target = project.resolve(entry.getKey());
      Files.createDirectories(target.getParent());
      JsonNode content = entry.getValue();
      Files.writeString(target, content.isTextual() ? content.asText()
          : Suite.MAPPER.writerWithDefaultPrettyPrinter().writeValueAsString(content) + "\n");
    }

    // The class path in an argument file: a Windows command line cannot hold a test class path.
    String classPath = classes + File.pathSeparator + System.getProperty("java.class.path");
    Path arguments = work.resolve("java.args");
    Files.writeString(arguments, "-cp \"" + classPath.replace("\\", "\\\\") + "\"\n");
    Path outcomes = work.resolve("outcomes.json");
    Path stdout = work.resolve("stdout");
    Path stderr = work.resolve("stderr");
    String className = file.substring(0, file.lastIndexOf('.'));
    Process process = new ProcessBuilder(
        Path.of(System.getProperty("java.home"), "bin", "java").toString(), "@" + arguments,
        FrameworkLauncher.class.getName(), className, outcomes.toString())
        .directory(project.toFile())
        .redirectOutput(stdout.toFile())
        .redirectError(stderr.toFile())
        .start();
    if (!process.waitFor(DEADLINE_SECONDS, TimeUnit.SECONDS)) {
      process.destroyForcibly().waitFor();
      failures.add("the test JVM did not finish in " + DEADLINE_SECONDS + "s:\n" + read(stdout) + "\n" + read(stderr));
      return project;
    }
    if (!Files.exists(outcomes)) {
      failures.add("the test JVM wrote no outcomes (exit " + process.exitValue() + "):\n" + read(stdout) + "\n" + read(stderr));
      return project;
    }

    List<SubjectRunner.Outcome> ran = new ArrayList<>();
    for (JsonNode outcome : Suite.MAPPER.readTree(Files.readString(outcomes))) {
      ran.add(new SubjectRunner.Outcome(outcome.path("name").asText(), outcome.path("container").asBoolean(),
          outcome.path("error").isNull() ? null : outcome.path("error").asText()));
    }
    JsonNode expect = testCase.path("expect");
    SubjectRunner.checkTests(expect, ran, failures);
    for (String stream : List.of("stdout", "stderr")) {
      String text = read(stream.equals("stdout") ? stdout : stderr);
      for (JsonNode part : expect.path("output").path(stream)) {
        if (!text.contains(part.asText())) {
          failures.add("the run's " + stream + " does not hold " + Suite.show(part) + ":\n" + text);
        }
      }
    }
    return project;
  }

  /** Compiles a Spock specification into {@code classes}, Spock's transforms applied; the compiler's errors when it does not. */
  private static String compileGroovy(Path source, Path classes) {
    CompilerConfiguration configuration = new CompilerConfiguration();
    configuration.setTargetDirectory(classes.toFile());
    configuration.setSourceEncoding("UTF-8");
    configuration.setClasspathList(List.of(System.getProperty("java.class.path").split(File.pathSeparator)));
    CompilationUnit unit = new CompilationUnit(configuration);
    unit.addSource(source.toFile());
    try {
      unit.compile();
      return null;
    } catch (CompilationFailedException e) {
      return e.getMessage();
    }
  }

  /** Each contract file the run wrote, keyed by framework and name, as the SHA-256 of its bytes. */
  private static Map<String, String> digests(String framework, Path contracts) throws IOException {
    Map<String, String> digests = new TreeMap<>();
    if (!Files.isDirectory(contracts)) {
      return digests;
    }
    try (Stream<Path> files = Files.list(contracts)) {
      for (Path file : files.filter(p -> p.toString().endsWith(".json")).sorted().toList()) {
        digests.put(framework + " " + file.getFileName(), sha256(Files.readAllBytes(file)));
      }
    }
    return digests;
  }

  private static String sha256(byte[] bytes) {
    try {
      return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
    } catch (NoSuchAlgorithmException e) {
      throw new IllegalStateException("every JVM has SHA-256", e);
    }
  }

  private static String read(Path file) throws IOException {
    return Files.exists(file) ? Files.readString(file, StandardCharsets.UTF_8) : "";
  }
}
