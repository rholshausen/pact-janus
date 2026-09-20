package io.pact.janus.sdk.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.MissingNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;

/**
 * The conformance suite as this SDK reads it (plan task 6.4): the cases under
 * {@code conformance/cases}, loaded as data, and the report a run writes. Nothing here is the JVM's
 * opinion about a case — these are the same files the TypeScript driver reads.
 */
final class Suite {
  static final ObjectMapper MAPPER = new ObjectMapper();

  /** One case's outcome, as the report records it. */
  record Result(String id, String status, List<String> covers, String detail) {}

  private Suite() {}

  /** Where the suite lives: {@code janus.conformance.suite}, or the repository's own copy. */
  static Path root() {
    String configured = System.getProperty("janus.conformance.suite");
    return configured != null ? Path.of(configured) : Path.of("../../../conformance").toAbsolutePath().normalize();
  }

  /** Every case in the corpus, in id order — the order a report lists them in. */
  static List<JsonNode> cases() {
    Path cases = root().resolve("cases");
    List<JsonNode> loaded = new ArrayList<>();
    try (Stream<Path> categories = Files.list(cases)) {
      for (Path category : categories.filter(Files::isDirectory).sorted().toList()) {
        try (Stream<Path> files = Files.list(category)) {
          for (Path file : files.filter(p -> p.toString().endsWith(".json")).sorted().toList()) {
            loaded.add(MAPPER.readTree(Files.readString(file)));
          }
        }
      }
    } catch (IOException e) {
      throw new UncheckedIOException("reading the conformance suite at " + cases, e);
    }
    loaded.sort(Comparator.comparing(node -> node.path("id").asText()));
    return loaded;
  }

  /**
   * The report the suite checker reads ({@code cargo run -p pact_janus_conformance -- check}),
   * which is what makes "this SDK is conformant" a claim the build makes (ADR 0017).
   */
  static Path writeReport(List<Result> results) throws IOException {
    ObjectNode report = MAPPER.createObjectNode();
    report.put("$format", "janus-conformance-report/1");
    ObjectNode sdk = report.putObject("sdk");
    sdk.put("name", "pact-janus-jvm");
    sdk.put("version", "0.0.0");
    sdk.put("language", "jvm");
    ArrayNode cases = report.putArray("cases");
    results.stream().sorted(Comparator.comparing(Result::id)).forEach(result -> {
      ObjectNode entry = cases.addObject();
      entry.put("id", result.id());
      entry.put("status", result.status());
      result.covers().forEach(entry.putArray("covers")::add);
      if (result.detail() != null) {
        entry.put("detail", result.detail());
      }
    });
    String configured = System.getProperty("janus.conformance.report");
    Path file = configured != null ? Path.of(configured) : Path.of("build/conformance/jvm.json").toAbsolutePath();
    Files.createDirectories(file.getParent());
    Files.writeString(file, MAPPER.writerWithDefaultPrettyPrinter().writeValueAsString(report) + "\n");
    return file;
  }

  // -----------------------------------------------------------------------------------------
  // How a case's expectation is compared with what the SDK did. Documents are compared by value:
  // object members are a set, because ADR 0017 compares content and a language cannot always
  // preserve the order its author wrote (6.3's report §2.1); arrays are compared in order, and
  // numbers by value, so 42 written as a long and 42 written as an int are the same number.

  static boolean equal(JsonNode expected, JsonNode actual) {
    JsonNode a = expected == null ? MissingNode.getInstance() : expected;
    JsonNode b = actual == null ? MissingNode.getInstance() : actual;
    if (a.isNumber() && b.isNumber()) {
      return a.decimalValue().compareTo(b.decimalValue()) == 0;
    }
    if (a.isArray() && b.isArray()) {
      if (a.size() != b.size()) {
        return false;
      }
      for (int i = 0; i < a.size(); i++) {
        if (!equal(a.get(i), b.get(i))) {
          return false;
        }
      }
      return true;
    }
    if (a.isObject() && b.isObject()) {
      if (a.size() != b.size()) {
        return false;
      }
      for (Map.Entry<String, JsonNode> member : a.properties()) {
        if (!b.has(member.getKey()) || !equal(member.getValue(), b.get(member.getKey()))) {
          return false;
        }
      }
      return true;
    }
    return a.equals(b);
  }

  /** Every member {@code expected} names is there and equal; members it does not name are not checked. */
  static boolean subset(JsonNode expected, JsonNode actual) {
    if (expected != null && expected.isObject() && actual != null && actual.isObject()) {
      for (Map.Entry<String, JsonNode> member : expected.properties()) {
        if (!actual.has(member.getKey()) || !subset(member.getValue(), actual.get(member.getKey()))) {
          return false;
        }
      }
      return true;
    }
    return equal(expected, actual);
  }

  static String show(JsonNode node) {
    return node == null ? "(absent)" : node.toString();
  }
}
