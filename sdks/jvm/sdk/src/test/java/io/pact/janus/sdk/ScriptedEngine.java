package io.pact.janus.sdk;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import io.pact.janus.sdk.engine.Embedding;
import io.pact.janus.sdk.engine.FramePipe;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Function;

/**
 * An in-memory engine for the session-lifecycle and variant-iteration tests: answers each operation
 * from a script, and records every request frame it was sent. Its default script is a well-behaved
 * engine; a test overrides the operations it cares about.
 */
final class ScriptedEngine implements Embedding {
  static final ObjectMapper MAPPER = new ObjectMapper();

  /** One request the engine received. */
  record Call(String op, JsonNode body) {}

  final List<Call> calls = new ArrayList<>();
  int opened;
  int closed;
  private final Map<String, Function<JsonNode, String>> script = new HashMap<>();
  private int handles;
  private final List<String> served = new ArrayList<>();

  ScriptedEngine() {
    ok("engine/hello", "{\"protocol-version\":1,\"engine\":{\"name\":\"scripted\",\"version\":\"0\"},\"capabilities\":{}}");
    ok("consumer-session/create", "{\"session\":\"cs-1\"}");
    on("consumer-session/add-interaction", body -> "{\"ok\":{\"handle\":\"i-" + ++handles + "\"}}");
    ok("consumer-session/start-transport",
        "{\"endpoint\":{\"kind\":\"http\",\"host\":\"127.0.0.1\",\"port\":1,\"base-url\":\"http://127.0.0.1:1\"}}");
    variants("base");
    on("consumer-session/serve-variant", body -> {
      served.add(body.path("variant").asText());
      return "{\"ok\":{}}";
    });
    on("consumer-session/finalise", body -> "{\"ok\":{\"results\":[],\"contract\":"
        + "{\"$format\":\"janus-contract/1\",\"consumer\":{\"name\":\"c\"},\"provider\":{\"name\":\"p\"},\"interactions\":[]}}}");
  }

  /** Answers {@code op} with {@code ok}. */
  ScriptedEngine ok(String op, String okJson) {
    return on(op, body -> "{\"ok\":" + okJson + "}");
  }

  /** Answers {@code op} with an error document. */
  ScriptedEngine error(String op, String errorJson) {
    return on(op, body -> "{\"error\":" + errorJson + "}");
  }

  /**
   * Answers {@code op} with the frame members {@code answer} returns ({@code "ok":…} or
   * {@code "error":…}, without braces-wrapped type/id, which are added here).
   */
  ScriptedEngine on(String op, Function<JsonNode, String> answer) {
    script.put(op, answer);
    return this;
  }

  /** Makes {@code consumer-session/variants} select variants with these ids, labelled {@code v:<id>}. */
  ScriptedEngine variants(String... ids) {
    StringBuilder json = new StringBuilder("{\"variants\":[");
    for (int i = 0; i < ids.length; i++) {
      json.append(i == 0 ? "" : ",").append("{\"id\":\"").append(ids[i]).append("\",\"label\":\"v:").append(ids[i])
          .append("\",\"origin\":\"").append(i == 0 ? "base" : "covering").append("\",\"assignment\":[]}");
    }
    return ok("consumer-session/variants", json.append("],\"report\":{}}").toString());
  }

  List<String> ops() {
    return calls.stream().map(Call::op).toList();
  }

  List<String> served() {
    return served;
  }

  JsonNode body(String op) {
    return calls.stream().filter(c -> c.op().equals(op)).map(Call::body).findFirst().orElseThrow();
  }

  @Override
  public FramePipe open() {
    opened++;
    return new FramePipe() {
      @Override
      public byte[] call(byte[] requestFrame) {
        try {
          JsonNode frame = MAPPER.readTree(requestFrame);
          String op = frame.path("op").asText();
          calls.add(new Call(op, frame.path("body")));
          Function<JsonNode, String> answer = script.get(op);
          String members = answer == null
              ? "\"error\":{\"code\":\"operation-unsupported\",\"category\":\"protocol\",\"message\":\"no script\",\"details\":{\"op\":\"" + op + "\"}}"
              : strip(answer.apply(frame.path("body")));
          String response = "{\"type\":\"response\",\"id\":\"" + frame.path("id").asText() + "\"," + members + "}";
          return response.getBytes(StandardCharsets.UTF_8);
        } catch (Exception e) {
          throw new IllegalStateException(e);
        }
      }

      @Override
      public void close() {
        closed++;
      }
    };
  }

  private static String strip(String braced) {
    String t = braced.trim();
    return t.substring(1, t.length() - 1);
  }

  static ObjectNode json(String text) {
    try {
      return (ObjectNode) MAPPER.readTree(text);
    } catch (Exception e) {
      throw new IllegalArgumentException(e);
    }
  }
}
