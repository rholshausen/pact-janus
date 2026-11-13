package io.pact.janus.sdk.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import io.pact.janus.sdk.JanusEngineException;
import io.pact.janus.sdk.ShapeRecorder;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Base64;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Runs a recorder case (README §3): a shape recorder built from the case's data, each answer observed
 * in order and the recorder finished, against the case's scripted engine or, for a live case, the
 * real engine. What this class knows is how Java spells the primitives; what they must do is the
 * case's.
 */
final class RecorderRunner {
  private static final String PROVIDER = "orders-api";

  private RecorderRunner() {}

  @SuppressWarnings("unchecked")
  static void run(JsonNode testCase, List<String> failures) throws IOException {
    boolean live = "live".equals(testCase.path("category").asText());
    ScriptedPipe pipe = live ? null : new ScriptedPipe(testCase.get("engine"));
    JsonNode script = testCase.path("recorder");
    JsonNode expect = testCase.path("expect");
    Path dir = Files.createTempDirectory("janus-recorder-");
    Path shapeDir = dir.resolve("shapes");
    Path evidenceDir = dir.resolve("evidence");
    ShapeRecorder recorder = ShapeRecorder.forProvider(PROVIDER).withShapeDir(shapeDir).withEvidenceDir(evidenceDir);
    if (script.has("source")) {
      recorder.withSource(script.get("source").asText());
    }
    if (script.has("policy")) {
      recorder.withPolicy(Suite.MAPPER.convertValue(script.get("policy"), Map.class));
    }
    if (pipe != null) {
      recorder.withEmbedding(pipe);
    }

    ArrayNode observed = Suite.MAPPER.createArrayNode();
    for (JsonNode observation : script.path("observations")) {
      if (observation.has("message")) {
        observed.add(observeMessage(recorder, observation, pipe));
        continue;
      }
      Map<String, Object> operation = new LinkedHashMap<>();
      observation.path("operation").properties().forEach(slot -> operation.put(slot.getKey(), Dsl.template(slot.getValue())));
      JsonNode response = observation.path("response");
      Map<String, List<String>> headers = new LinkedHashMap<>();
      response.path("headers").properties().forEach(header -> {
        List<String> lines = new ArrayList<>();
        if (header.getValue().isArray()) {
          header.getValue().forEach(line -> lines.add(line.asText()));
        } else {
          lines.add(header.getValue().asText());
        }
        headers.put(header.getKey(), lines);
      });
      JsonNode body = response.path("body");
      byte[] octets = body.has("bytes") ? Base64.getDecoder().decode(body.get("bytes").asText())
          : body.has("text") ? body.get("text").asText().getBytes(StandardCharsets.UTF_8) : null;
      ShapeRecorder.Options options = ShapeRecorder.Options.none();
      if (observation.has("description")) {
        options = options.withDescription(observation.get("description").asText());
      }
      if (observation.has("states")) {
        List<String> states = new ArrayList<>();
        observation.get("states").forEach(state -> states.add(state.asText()));
        options = options.withStates(states);
      }
      if (observation.has("request")) {
        options = options.withRequest(observation.path("request").path("method").asText(),
            observation.path("request").path("path").asText());
      }
      int sent = pipe == null ? 0 : pipe.ops().size();
      try {
        recorder.observe(operation, new ShapeRecorder.Response(response.path("status").asInt(), headers, octets), options);
        observed.add("ok");
      } catch (JanusEngineException e) {
        observed.addObject().put("error", "engine-error").put("code", e.code());
      } catch (IllegalArgumentException e) {
        if (pipe != null && pipe.ops().size() != sent) {
          throw e;
        }
        observed.addObject().put("error", "refused");
      }
    }
    Path written = script.path("finish").asBoolean(true) ? recorder.finish() : null;

    if (expect.has("observed") && !Suite.equal(expect.get("observed"), observed)) {
      failures.add("observe ended differently\n  expected: " + Suite.show(expect.get("observed")) + "\n  actual:   " + Suite.show(observed));
    }
    if (expect.has("ops") && pipe != null && !Suite.equal(expect.get("ops"), Suite.MAPPER.valueToTree(pipe.ops()))) {
      failures.add("the operations sent differ\n  expected: " + Suite.show(expect.get("ops")) + "\n  actual:   " + pipe.ops());
    }
    expect.path("frames").properties().forEach(frame -> {
      JsonNode actual = pipe == null ? null : pipe.bodyOf(frame.getKey());
      if (!Suite.subset(frame.getValue(), actual)) {
        failures.add("the '" + frame.getKey() + "' frame body\n  expected (a subset of): " + Suite.show(frame.getValue())
            + "\n  actual: " + Suite.show(actual));
      }
    });
    if (expect.has("engine-closed") && pipe != null && pipe.closed() != expect.get("engine-closed").asBoolean()) {
      failures.add("the engine was " + (pipe.closed() ? "" : "not ") + "ended, and the case expects otherwise");
    }
    Path shapeFile = shapeDir.resolve(PROVIDER + ".provider-shape.json");
    if (expect.has("written")) {
      boolean wanted = expect.get("written").asBoolean();
      if (wanted != (written != null)) {
        failures.add("finish answered " + written + ", and the case expects " + (wanted ? "a path" : "none"));
      } else if (wanted && (!written.equals(shapeFile) || !Files.exists(shapeFile))) {
        failures.add("finish answered " + written + ", and the case expects the file " + shapeFile);
      }
      JsonNode finalise = pipe == null ? null : pipe.bodyOf("provider-shape-session/finalise");
      JsonNode evidence = finalise == null ? null : finalise.path("write").path("evidence");
      String wantedEvidence = evidenceDir.resolve(PROVIDER + ".json").toString();
      if (wanted && pipe != null && (evidence == null || !wantedEvidence.equals(evidence.asText()))) {
        failures.add("finalise named the evidence " + Suite.show(evidence) + ", and the case expects " + wantedEvidence);
      }
    }
    if (expect.has("shape")) {
      JsonNode shape = Files.exists(shapeFile) ? Suite.MAPPER.readTree(Files.readString(shapeFile)) : null;
      if (!Suite.equal(expect.get("shape"), shape)) {
        failures.add("the shape written differs\n  expected: " + Suite.show(expect.get("shape")) + "\n  actual:   " + Suite.show(shape));
      }
    }
  }

  /** One message observation, as {@code observeMessage} spells it: the operation's metadata and body as templates. */
  private static JsonNode observeMessage(ShapeRecorder recorder, JsonNode observation, ScriptedPipe pipe) {
    Map<String, Object> operation = new LinkedHashMap<>();
    JsonNode written = observation.path("operation");
    if (written.has("metadata")) {
      Map<String, Object> metadata = new LinkedHashMap<>();
      written.get("metadata").properties().forEach(member -> metadata.put(member.getKey(), Dsl.template(member.getValue())));
      operation.put("metadata", metadata);
    }
    if (written.has("body")) {
      operation.put("body", Dsl.template(written.get("body")));
    }
    int sent = pipe == null ? 0 : pipe.ops().size();
    try {
      ShapeRecorder.MessageOptions options = null;
      if (observation.has("description")) {
        options = ShapeRecorder.MessageOptions.described(observation.get("description").asText());
        if (observation.has("states")) {
          List<String> states = new ArrayList<>();
          observation.get("states").forEach(state -> states.add(state.asText()));
          options = options.withStates(states);
        }
        if (observation.has("message-request")) {
          options = options.withRequest(ProviderRunner.message(observation.get("message-request")));
        }
      }
      recorder.observeMessage(operation, ProviderRunner.message(observation.get("message")), options);
      return Suite.MAPPER.getNodeFactory().textNode("ok");
    } catch (JanusEngineException e) {
      return Suite.MAPPER.createObjectNode().put("error", "engine-error").put("code", e.code());
    } catch (IllegalArgumentException e) {
      if (pipe != null && pipe.ops().size() != sent) {
        throw e;
      }
      return Suite.MAPPER.createObjectNode().put("error", "refused");
    }
  }
}
