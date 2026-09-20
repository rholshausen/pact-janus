package io.pact.janus.sdk.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import io.pact.janus.sdk.engine.Embedding;
import io.pact.janus.sdk.engine.FramePipe;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

/**
 * The engine a {@code lifecycle} or {@code variants} case runs against: a frame pipe that records
 * every operation the SDK sends and answers from the case's script (README §3). What these cases
 * check is what the SDK sends and what it does with what comes back — not what an engine would
 * decide.
 */
final class ScriptedPipe implements Embedding, FramePipe {
  private final JsonNode script;
  private final List<String> ops = new ArrayList<>();
  private final List<JsonNode> bodies = new ArrayList<>();
  private int handles;
  private boolean closed;

  ScriptedPipe(JsonNode script) {
    this.script = script == null ? Suite.MAPPER.createObjectNode() : script;
  }

  @Override
  public FramePipe open() {
    return this;
  }

  List<String> ops() {
    return List.copyOf(ops);
  }

  /** The first frame body sent for {@code op}. */
  JsonNode bodyOf(String op) {
    int at = ops.indexOf(op);
    return at < 0 ? null : bodies.get(at);
  }

  boolean closed() {
    return closed;
  }

  @Override
  public byte[] call(byte[] requestFrame) throws IOException {
    JsonNode frame = Suite.MAPPER.readTree(requestFrame);
    String op = frame.path("op").asText();
    ops.add(op);
    bodies.add(frame.path("body"));

    ObjectNode response = Suite.MAPPER.createObjectNode();
    response.put("type", "response");
    response.put("id", frame.path("id").asText());
    JsonNode error = script.path("errors").path(op);
    if (error.isObject()) {
      ObjectNode document = response.putObject("error");
      document.put("code", error.path("code").asText());
      document.put("message", error.path("message").asText(error.path("code").asText()));
      document.put("category", "request");
      ObjectNode details = document.putObject("details");
      error.path("details").properties().forEach(member -> details.set(member.getKey(), member.getValue()));
      if (error.has("problems")) {
        details.set("problems", error.get("problems"));
      }
    } else {
      response.set("ok", reply(op));
    }
    return (response + "\n").getBytes(StandardCharsets.UTF_8);
  }

  @Override
  public void close() {
    closed = true;
  }

  private ObjectNode reply(String op) {
    ObjectNode ok = Suite.MAPPER.createObjectNode();
    switch (op) {
      case "engine/hello" -> {
        ok.put("protocol-version", 1);
        ok.putObject("engine").put("name", "scripted").put("version", "0.0.0");
        ok.putObject("capabilities");
      }
      case "consumer-session/create" -> ok.put("session", "s-1");
      case "consumer-session/add-interaction" -> ok.put("handle", "i-" + ++handles);
      case "consumer-session/start-transport" ->
          ok.putObject("endpoint").put("kind", "http").put("base-url", "http://127.0.0.1:1");
      case "consumer-session/variants" -> {
        ArrayNode variants = ok.putArray("variants");
        if (script.path("variants").isArray()) {
          script.path("variants").forEach(variants::add);
        } else {
          variants.addObject().put("id", "base").put("label", "base");
        }
      }
      case "consumer-session/finalise" -> {
        if (script.path("results").isArray()) {
          ok.set("results", script.get("results"));
        } else {
          ArrayNode results = ok.putArray("results");
          for (int i = 1; i <= handles; i++) {
            results.addObject().put("handle", "i-" + i).put("status", "verified");
          }
        }
        if (!script.path("withhold-contract").asBoolean(false)) {
          ok.set("contract", script.has("contract") ? script.get("contract") : defaultContract());
        }
      }
      default -> {
        // Every other operation is answered with an empty result.
      }
    }
    return ok;
  }

  private JsonNode defaultContract() {
    try {
      return Suite.MAPPER.readTree("{\"$format\":\"janus-contract/1\",\"consumer\":{\"name\":\"web-app\"},"
          + "\"provider\":{\"name\":\"orders-api\"},\"interactions\":[]}");
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }
}
