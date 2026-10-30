package io.pact.janus.sdk.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import io.pact.janus.sdk.engine.Embedding;
import io.pact.janus.sdk.engine.FramePipe;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
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
  private static final HttpClient HTTP = HttpClient.newHttpClient();
  private final JsonNode script;
  private final List<String> ops = new ArrayList<>();
  private final List<JsonNode> bodies = new ArrayList<>();
  private int handles;
  private boolean closed;
  private boolean polled;
  /** What the SDK's endpoint answered each of the script's {@code invoke} calls (provider cases). */
  private final ArrayNode invocations = Suite.MAPPER.createArrayNode();

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

  ArrayNode invocations() {
    return invocations;
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
    // `after` answers that many calls the default way first: the engine that dies mid-loop.
    long before = ops.stream().filter(op::equals).count() - 1;
    if (error.isObject() && before >= error.path("after").asLong(0)) {
      ObjectNode document = response.putObject("error");
      document.put("code", error.path("code").asText());
      document.put("message", error.path("message").asText(error.path("code").asText()));
      document.put("category", "request");
      ObjectNode details = document.putObject("details");
      error.path("details").properties().forEach(member -> details.set(member.getKey(), member.getValue()));
      if (error.has("problems")) {
        details.set("problems", error.get("problems"));
      }
    } else if (op.equals("events/poll")) {
      response.set("ok", poll());
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
        if (script.has("capabilities")) {
          ok.set("capabilities", script.get("capabilities"));
        } else {
          ok.putObject("capabilities").putObject("config-loading");
        }
      }
      case "config/load" -> ok.set("config", script.has("config")
          ? script.get("config")
          : Suite.MAPPER.createObjectNode().put("version", 1).set("hooks", Suite.MAPPER.createObjectNode()));
      case "verification/verify" -> ok.put("session", "vs-1").put("stream", "s-1");
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

  /**
   * The run's stream, all at once: the script's hooks are called first, as the engine would call
   * them during the run, then its events and the terminal one (README §3).
   */
  private ObjectNode poll() throws IOException {
    ObjectNode ok = Suite.MAPPER.createObjectNode();
    ArrayNode events = ok.putArray("events");
    if (polled) {
      return ok;
    }
    polled = true;
    for (JsonNode invoke : script.path("invoke")) {
      invocations.add(invoke(invoke));
    }
    ArrayNode scripted = Suite.MAPPER.createArrayNode();
    if (script.path("events").isArray()) {
      scripted.addAll((ArrayNode) script.get("events"));
    } else {
      ObjectNode payload = scripted.addObject().put("kind", "verification/interaction-result").putObject("payload");
      ObjectNode ref = payload.putObject("interaction");
      ref.putObject("contract").put("consumer", "web-app").put("provider", "orders-api");
      ref.put("index", 0).put("description", "get an order");
      payload.put("variant", "base").put("status", "verified");
    }
    ObjectNode finished = scripted.addObject().put("kind", "verification/finished");
    if (script.has("summary")) {
      finished.set("payload", script.get("summary"));
    } else {
      finished.putObject("payload").put("status", "verified").putObject("variants").put("verified", 1).put("failed", 0);
    }
    for (int i = 0; i < scripted.size(); i++) {
      ObjectNode event = events.addObject().put("stream", "s-1").put("seq", i + 1);
      event.setAll((ObjectNode) scripted.get(i));
      if (i == scripted.size() - 1) {
        event.put("last", true);
      }
    }
    return ok;
  }

  /** POSTs the context to the hook entry the {@code verify} frame names, with its headers. */
  private ObjectNode invoke(JsonNode invoke) throws IOException {
    String point = invoke.path("point").asText();
    String name = invoke.path("hook").asText();
    JsonNode entry = null;
    for (JsonNode candidate : bodyOf("verification/verify").path("target").path("hooks").path("hooks").path(point)) {
      if (candidate.path("name").asText().equals(name)) {
        entry = candidate;
      }
    }
    if (entry == null) {
      throw new IllegalStateException("the verify frame has no '" + name + "' hook at '" + point + "'");
    }
    HttpRequest.Builder request = HttpRequest.newBuilder(URI.create(entry.path("run").path("url").asText()))
        .header("content-type", "application/json")
        .POST(HttpRequest.BodyPublishers.ofString(invoke.path("context").toString()));
    if (invoke.path("authorised").asBoolean(true)) {
      entry.path("run").path("headers").properties().forEach(h -> request.header(h.getKey(), h.getValue().asText()));
    }
    HttpResponse<String> response;
    try {
      response = HTTP.send(request.build(), HttpResponse.BodyHandlers.ofString());
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new IOException(e);
    }
    ObjectNode answered = Suite.MAPPER.createObjectNode().put("status", response.statusCode());
    if (!response.body().isEmpty()) {
      answered.set("result", Suite.MAPPER.readTree(response.body()));
    }
    return answered;
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
