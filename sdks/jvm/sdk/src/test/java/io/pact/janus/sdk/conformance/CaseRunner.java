package io.pact.janus.sdk.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import io.pact.janus.sdk.ContractWithheldException;
import io.pact.janus.sdk.ExecuteFailedException;
import io.pact.janus.sdk.Interaction;
import io.pact.janus.sdk.Janus;
import io.pact.janus.sdk.JanusEngineException;
import io.pact.janus.sdk.JanusOptions;
import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;

/**
 * Runs one conformance case against this SDK and says what it found. Every check a case can ask for
 * lives here; a case's own JSON says which ones it asks for. A failure is a list of lines, so a
 * report can carry it and a test can throw it.
 */
final class CaseRunner {
  private static final String CONSUMER = "web-app";
  private static final String PROVIDER = "orders-api";

  private final JsonNode testCase;
  private final List<String> failures = new ArrayList<>();

  private CaseRunner(JsonNode testCase) {
    this.testCase = testCase;
  }

  static Suite.Result run(JsonNode testCase) {
    CaseRunner runner = new CaseRunner(testCase);
    try {
      if ("translation".equals(testCase.path("category").asText())) {
        runner.translation();
      } else {
        runner.session();
      }
    } catch (Exception | AssertionError problem) {
      runner.failures.add("the case did not run: " + problem);
    }
    List<String> covers = new ArrayList<>();
    testCase.path("covers").forEach(id -> covers.add(id.asText()));
    return new Suite.Result(
        testCase.path("id").asText(),
        runner.failures.isEmpty() ? "passed" : "failed",
        covers,
        runner.failures.isEmpty() ? null : String.join("\n", runner.failures));
  }

  /** Category 1: what a DSL chain builds, with no engine anywhere. */
  private void translation() {
    Janus janus = Janus.of(CONSUMER, PROVIDER);
    JsonNode expect = testCase.path("expect");
    if (expect.path("refused").asBoolean(false)) {
      // The chain must not become a document. How it refuses is this language's business; that it
      // refuses is the case's (ADR 0019).
      try {
        JsonNode document = Suite.MAPPER.valueToTree(Dsl.interaction(janus, testCase.path("interaction")).toDocument());
        failures.add("the chain was accepted, and the case expects it refused: " + Suite.show(document));
      } catch (RuntimeException refused) {
        // Refused, as the case requires.
      }
      return;
    }
    JsonNode built = Suite.MAPPER.valueToTree(Dsl.interaction(janus, testCase.path("interaction")).toDocument());
    if (expect.has("spec") && !Suite.equal(expect.get("spec"), built)) {
      failures.add("the built interaction-spec document differs\n  expected: " + Suite.show(expect.get("spec"))
          + "\n  actual:   " + Suite.show(built));
    }
    expect.path("at").properties().forEach(at -> {
      JsonNode actual = built.at(at.getKey());
      if (!Suite.equal(at.getValue(), actual)) {
        failures.add("at '" + at.getKey() + "'\n  expected: " + Suite.show(at.getValue())
            + "\n  actual:   " + Suite.show(actual));
      }
    });
  }

  /** Categories 2–4: the SDK driving an engine — the case's scripted one, or the real one. */
  private void session() throws IOException {
    boolean live = "live".equals(testCase.path("category").asText());
    ScriptedPipe pipe = live ? null : new ScriptedPipe(testCase.get("engine"));
    Path contractDirectory = Files.createTempDirectory("janus-conformance-");
    JanusOptions options = JanusOptions.defaults().withContractDirectory(contractDirectory);
    Janus janus = Janus.of(CONSUMER, PROVIDER, pipe == null ? options : options.withEmbedding(pipe));
    Set<String> dimensions = new LinkedHashSet<>();
    List<Integer> variantCounts = new ArrayList<>();

    try {
      int index = 0;
      for (JsonNode step : testCase.path("steps")) {
        String where = "step " + ++index;
        if (step.has("build")) {
          Dsl.interaction(janus, step.get("build"));
          check(step.path("expect"), where, Outcome.ok(), List.of(), null);
        } else if (step.has("execute")) {
          List<String> calls = new ArrayList<>();
          JsonNode closure = step.path("execute").path("closure");
          Interaction interaction = Dsl.interaction(janus, step.path("execute").path("interaction"));
          Outcome outcome = capture(() -> janus.execute(interaction, (mock, variant) -> {
            calls.add(variant.id());
            variant.assignment().forEach(point -> dimensions.add(point.dimension()));
            if (fails(closure, variant.id())) {
              throw new AssertionError("the consumer cannot handle variant '" + variant.id() + "'");
            }
            if (closure.has("exchange")) {
              send(mock.url(), closure.get("exchange"));
            }
          }));
          variantCounts.add(calls.size());
          check(step.path("expect"), where, outcome, calls, null);
        } else if (step.has("finalise")) {
          Outcome outcome = capture(janus::finalise);
          check(step.path("expect"), where, outcome, List.of(), readContract(contractDirectory));
        }
      }

      JsonNode expect = testCase.path("expect");
      if (expect.has("ops") && pipe != null) {
        JsonNode sent = Suite.MAPPER.valueToTree(pipe.ops());
        if (!Suite.equal(expect.get("ops"), sent)) {
          failures.add("the operations sent differ\n  expected: " + Suite.show(expect.get("ops"))
              + "\n  actual:   " + Suite.show(sent));
        }
      }
      if (pipe != null) {
        expect.path("frames").properties().forEach(frame -> {
          JsonNode body = pipe.bodyOf(frame.getKey());
          if (!Suite.subset(frame.getValue(), body)) {
            failures.add("the '" + frame.getKey() + "' frame body\n  expected (a subset of): "
                + Suite.show(frame.getValue()) + "\n  actual: " + Suite.show(body));
          }
        });
        if (expect.has("engine-closed") && pipe.closed() != expect.path("engine-closed").asBoolean()) {
          failures.add("the engine was " + (pipe.closed() ? "" : "not ") + "ended, and the case expects it "
              + (expect.path("engine-closed").asBoolean() ? "" : "not ") + "to be");
        }
      }
      for (JsonNode dimension : expect.path("dimensions")) {
        if (!dimensions.contains(dimension.asText())) {
          failures.add("the engine assigned no '" + dimension.asText() + "' dimension; it assigned " + dimensions);
        }
      }
      if (expect.has("variant-count") && !variantCounts.contains(expect.path("variant-count").asInt())) {
        failures.add("the engine selected " + variantCounts + " variants, and the case expects "
            + expect.path("variant-count").asInt());
      }
    } finally {
      // Sessions are the only resource (engine-protocol spec §7.1): a case that did not finalise
      // still has an engine to end, and on a live case that is a subprocess.
      try {
        janus.finalise();
      } catch (RuntimeException | AssertionError ignored) {
        // Whatever it reports has already been checked, or the case never asked. This SDK's
        // failures are AssertionErrors, not exceptions (STYLE.md), so both are caught here.
      }
    }
  }

  // -------------------------------------------------------------------------------------------
  // What a step produced, in the case's own vocabulary (README §4).

  private record Outcome(String kind, String code, List<String> variants, List<String> interactions,
      Boolean engineWithheld) {
    static Outcome ok() {
      return new Outcome("ok", null, null, null, null);
    }
  }

  private interface Body {
    void run() throws Exception;
  }

  private Outcome capture(Body body) {
    try {
      body.run();
      return Outcome.ok();
    } catch (ExecuteFailedException failed) {
      return new Outcome("execute-failed", null,
          failed.failures().stream().map(failure -> failure.variant().id()).toList(), null, null);
    } catch (ContractWithheldException withheld) {
      return new Outcome("contract-withheld", null, null, withheld.failedInteractions(), withheld.engineWithheld());
    } catch (JanusEngineException engineError) {
      return new Outcome("engine-error", engineError.code(), null, null, null);
    } catch (Exception problem) {
      throw new IllegalStateException("the case's step failed in a way no outcome describes", problem);
    }
  }

  private void check(JsonNode expect, String where, Outcome outcome, List<String> calls, Contract contract) {
    if (!expect.isObject()) {
      return;
    }
    if (expect.has("outcome") && !matches(expect.get("outcome"), outcome)) {
      failures.add(where + ": the outcome differs\n  expected: " + Suite.show(expect.get("outcome"))
          + "\n  actual:   " + outcome);
    }
    if (expect.has("closure-calls")) {
      JsonNode actual = Suite.MAPPER.valueToTree(calls);
      if (!Suite.equal(expect.get("closure-calls"), actual)) {
        failures.add(where + ": the closure ran on different variants\n  expected: "
            + Suite.show(expect.get("closure-calls")) + "\n  actual:   " + Suite.show(actual));
      }
    }
    JsonNode wanted = expect.path("contract");
    if (wanted.isObject() && contract != null) {
      if (wanted.has("written") && wanted.path("written").asBoolean() != contract.written()) {
        failures.add(where + ": a contract was " + (contract.written() ? "" : "not ")
            + "written, and the case expects it " + (wanted.path("written").asBoolean() ? "" : "not ") + "to be");
      }
      if (wanted.has("text") && !wanted.path("text").asText().equals(contract.text())) {
        failures.add(where + ": the contract file's text differs\n  expected: "
            + Suite.MAPPER.valueToTree(wanted.path("text").asText())
            + "\n  actual:   " + Suite.MAPPER.valueToTree(contract.text()));
      }
      if (wanted.has("content")) {
        checkContent(wanted.get("content"), contract.document(), where);
      }
    }
  }

  /**
   * Interaction content, and nothing else (ADR 0017): each interaction by description, compared on
   * the members the case names — {@code states}, {@code parts}, {@code selection}. {@code metadata}
   * and the parties are facts about the SDK that wrote the file, deliberately outside content
   * identity.
   */
  private void checkContent(JsonNode expected, JsonNode actual, String where) {
    JsonNode recorded = actual == null ? Suite.MAPPER.createArrayNode() : actual.path("interactions");
    JsonNode wanted = expected.path("interactions");
    if (recorded.size() != wanted.size()) {
      failures.add(where + ": the contract records " + recorded.size() + " interactions, and the case expects "
          + wanted.size());
    }
    for (JsonNode interaction : wanted) {
      String description = interaction.path("description").asText();
      JsonNode found = null;
      for (JsonNode candidate : recorded) {
        if (description.equals(candidate.path("description").asText())) {
          found = candidate;
          break;
        }
      }
      if (found == null) {
        failures.add(where + ": the contract records no interaction '" + description + "'");
        continue;
      }
      for (var member : interaction.properties()) {
        if (!Suite.equal(member.getValue(), found.get(member.getKey()))) {
          failures.add(where + ": '" + description + "' recorded a different '" + member.getKey() + "'\n  expected: "
              + Suite.show(member.getValue()) + "\n  actual:   " + Suite.show(found.get(member.getKey())));
        }
      }
    }
  }

  private boolean matches(JsonNode expected, Outcome actual) {
    if (expected.isTextual()) {
      return expected.asText().equals(actual.kind());
    }
    if (!expected.path("error").asText().equals(actual.kind())) {
      return false;
    }
    if (expected.has("code") && !expected.path("code").asText().equals(actual.code())) {
      return false;
    }
    if (expected.has("variants")
        && !Suite.equal(expected.get("variants"), Suite.MAPPER.valueToTree(actual.variants()))) {
      return false;
    }
    if (expected.has("interactions")
        && !Suite.equal(expected.get("interactions"), Suite.MAPPER.valueToTree(actual.interactions()))) {
      return false;
    }
    return !expected.has("engine-withheld")
        || expected.path("engine-withheld").asBoolean() == Boolean.TRUE.equals(actual.engineWithheld());
  }

  private static boolean fails(JsonNode closure, String variant) {
    for (JsonNode id : closure.path("fail-on")) {
      if ("*".equals(id.asText()) || id.asText().equals(variant)) {
        return true;
      }
    }
    return false;
  }

  // -------------------------------------------------------------------------------------------
  // The contract file, and the live cases' HTTP client.

  private record Contract(boolean written, String text, JsonNode document) {}

  private Contract readContract(Path directory) throws IOException {
    Path file = directory.resolve(CONSUMER + "-" + PROVIDER + ".janus.json");
    if (!Files.exists(file)) {
      return new Contract(false, null, null);
    }
    String text = new String(Files.readAllBytes(file), StandardCharsets.UTF_8);
    return new Contract(true, text, Suite.MAPPER.readTree(text));
  }

  /** A live case's consumer: the request the case says to send, and the status it expects back. */
  private void send(String baseUrl, JsonNode exchange) throws Exception {
    String query = exchange.has("query") ? "?" + exchange.path("query").asText() : "";
    URI uri = URI.create(baseUrl + exchange.path("path").asText() + query);
    HttpRequest.Builder request = HttpRequest.newBuilder(uri);
    exchange.path("headers").properties().forEach(header -> request.header(header.getKey(), header.getValue().asText()));
    if (exchange.has("body")) {
      request.header("Content-Type", "application/json");
      request.method(exchange.path("method").asText(),
          HttpRequest.BodyPublishers.ofString(exchange.get("body").toString()));
    } else {
      request.method(exchange.path("method").asText(), HttpRequest.BodyPublishers.noBody());
    }
    // HTTP/1.1: the JDK's default client offers an HTTP/2 upgrade the mock never answers
    // (Phase 9 findings; 6.3's report §3.1), so the version is pinned here as STYLE.md tells users to.
    HttpClient client = HttpClient.newBuilder().version(HttpClient.Version.HTTP_1_1).build();
    HttpResponse<String> response = client.send(request.build(), HttpResponse.BodyHandlers.ofString());
    if (exchange.has("expect-status") && response.statusCode() != exchange.path("expect-status").asInt()) {
      throw new AssertionError(exchange.path("method").asText() + " " + uri + ": the mock answered "
          + response.statusCode() + ", not " + exchange.path("expect-status").asInt() + ": " + response.body());
    }
  }
}
