package io.pact.janus.sdk.conformance;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.NullNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.net.httpserver.HttpServer;
import io.pact.janus.sdk.JanusEngineException;
import io.pact.janus.sdk.Message;
import io.pact.janus.sdk.StateUnsupportedException;
import io.pact.janus.sdk.VerificationFailedException;
import io.pact.janus.sdk.VerificationResult;
import io.pact.janus.sdk.Verifier;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;

/**
 * Runs a provider case (README §3): a verifier built from the case's data, its handlers recording
 * each call, {@code verify} against the case's scripted engine — or, for a live case, the real engine
 * and a provider this driver serves. What this class knows is how Java spells the primitives; what
 * they must do is the case's.
 */
final class ProviderRunner {
  private static final TypeReference<Map<String, Object>> MAP = new TypeReference<>() {};
  private static final Pattern TOKEN = Pattern.compile("^Bearer [0-9a-fA-F]{32,}$");

  private final JsonNode testCase;
  private final List<String> failures;
  private final ArrayNode calls = Suite.MAPPER.createArrayNode();

  private ProviderRunner(JsonNode testCase, List<String> failures) {
    this.testCase = testCase;
    this.failures = failures;
  }

  static void run(JsonNode testCase, List<String> failures) throws IOException {
    new ProviderRunner(testCase, failures).run();
  }

  private void run() throws IOException {
    boolean live = "live".equals(testCase.path("category").asText());
    ScriptedPipe pipe = live ? null : new ScriptedPipe(testCase.get("engine"));
    JsonNode script = testCase.path("verifier");
    JsonNode expect = testCase.path("expect");
    Path dir = Files.createTempDirectory("janus-provider-");
    HttpServer provider = live ? serve(testCase.path("provider")) : null;
    try {
      String url = script.has("provider-url")
          ? script.get("provider-url").asText()
          : "http://127.0.0.1:" + (provider == null ? 0 : provider.getAddress().getPort());
      Verifier verifier = Verifier.forProvider(url);
      if (script.has("pacts")) {
        List<Path> pacts = new ArrayList<>();
        int i = 0;
        for (JsonNode document : script.get("pacts")) {
          Path file = dir.resolve("pact-" + ++i + ".json");
          Files.writeString(file, document.toString());
          pacts.add(file);
        }
        verifier.withPacts(pacts);
      }
      if (script.has("broker")) {
        verifier.withBroker(Suite.MAPPER.convertValue(script.get("broker"), MAP));
      }
      if (script.has("urls")) {
        verifier.withUrls(Suite.MAPPER.convertValue(script.get("urls"), MAP));
      }
      if (script.has("options")) {
        verifier.withOptions(Suite.MAPPER.convertValue(script.get("options"), MAP));
      }
      if (script.has("transport")) {
        verifier.withTransport(Suite.MAPPER.convertValue(script.get("transport"), MAP));
      }
      Path configFile = dir.resolve("verifier.janus.yaml").toAbsolutePath();
      if (script.has("config")) {
        Files.writeString(configFile, script.get("config").asText());
        // Relative to the working directory, as a test would write it (`config-loaded`).
        verifier.withConfig(relative(configFile));
      }
      if (script.has("components")) {
        verifier.withComponents(Suite.MAPPER.convertValue(script.get("components"), new TypeReference<List<Map<String, Object>>>() {}));
      }
      if (pipe != null) {
        verifier.withEmbedding(pipe);
      }
      int n = 0;
      for (JsonNode handler : script.path("state-handlers")) {
        String name = "state-" + ++n;
        JsonNode teardown = handler.get("teardown");
        verifier.state(handler.path("state").asText(),
            params -> act(handler.path("setup"), name, "setup", params),
            teardown == null ? null : params -> act(teardown, name, "teardown", params));
      }
      n = 0;
      for (JsonNode filter : script.path("request-filters")) {
        String name = "request-filter-" + ++n;
        Verifier.RequestFilter spelled = request -> {
          ObjectNode call = calls.addObject().put("handler", name);
          call.set("request", Suite.MAPPER.valueToTree(request));
          if (filter.has("throws")) {
            throw new IllegalStateException(filter.get("throws").asText());
          }
          Map<String, Object> returned = new LinkedHashMap<>(request);
          returned.putAll(Suite.MAPPER.convertValue(filter.path("set").isObject() ? filter.get("set") : Suite.MAPPER.createObjectNode(), MAP));
          return returned;
        };
        if (filter.has("changes")) {
          verifier.requestFilter(spelled, Suite.MAPPER.convertValue(filter.get("changes"), new TypeReference<List<String>>() {}));
        } else {
          verifier.requestFilter(spelled);
        }
      }
      n = 0;
      for (JsonNode producer : script.path("message-producers")) {
        String name = "message-" + ++n;
        verifier.message(producer.path("description").asText(), request -> {
          ObjectNode call = calls.addObject().put("handler", name);
          call.set("request", Suite.MAPPER.valueToTree(request));
          if (producer.has("throws")) {
            throw new IllegalStateException(producer.get("throws").asText());
          }
          return message(producer.path("returns"));
        });
      }

      if (testCase.path("verify").asBoolean(true)) {
        JsonNode outcome;
        VerificationResult result = null;
        java.io.PrintStream out = System.out;
        java.io.ByteArrayOutputStream printed = new java.io.ByteArrayOutputStream();
        System.setOut(new java.io.PrintStream(printed, true, java.nio.charset.StandardCharsets.UTF_8));
        try {
          result = verifier.verify();
          outcome = Suite.MAPPER.getNodeFactory().textNode("ok");
        } catch (VerificationFailedException e) {
          result = e.result();
          ObjectNode failed = Suite.MAPPER.createObjectNode().put("error", "verification-failed");
          ArrayNode named = failed.putArray("failed");
          e.failures().forEach(f -> named.addObject()
              .<ObjectNode>set("interaction", Suite.MAPPER.valueToTree(f.get("interaction")))
              .put("variant", String.valueOf(f.get("variant"))));
          outcome = failed;
        } catch (JanusEngineException e) {
          outcome = Suite.MAPPER.createObjectNode().put("error", "engine-error").put("code", e.code());
        } catch (IllegalArgumentException e) {
          if (pipe != null && !pipe.ops().isEmpty()) {
            throw e;
          }
          outcome = Suite.MAPPER.createObjectNode().put("error", "refused");
        } finally {
          System.setOut(out);
        }
        // What verify printed: the run's report, line by line (ADR 0059).
        List<String> lines = printed.toString(java.nio.charset.StandardCharsets.UTF_8).lines().toList();
        if (expect.has("printed") && !Suite.equal(expect.get("printed"), Suite.MAPPER.valueToTree(lines))) {
          failures.add("what verify printed differs\n  expected: " + Suite.show(expect.get("printed")) + "\n  actual:   " + lines);
        }
        if (expect.has("outcome") && !Suite.subset(expect.get("outcome"), outcome)) {
          failures.add("the outcome differs\n  expected: " + Suite.show(expect.get("outcome")) + "\n  actual:   " + Suite.show(outcome));
        }
        JsonNode expectedResult = expect.path("result");
        if (expectedResult.has("status") && (result == null || !expectedResult.get("status").asText().equals(result.status()))) {
          failures.add("the result's status is " + (result == null ? "(none)" : result.status())
              + ", and the case expects " + expectedResult.get("status"));
        }
        if (expectedResult.has("results")) {
          JsonNode actual = result == null ? null : Suite.MAPPER.valueToTree(result.results());
          if (!Suite.equal(expectedResult.get("results"), actual)) {
            failures.add("the results differ\n  expected: " + Suite.show(expectedResult.get("results")) + "\n  actual:   " + Suite.show(actual));
          }
        }
        if (expectedResult.has("warnings")) {
          JsonNode actual = result == null ? null : Suite.MAPPER.valueToTree(result.warnings());
          if (!Suite.equal(expectedResult.get("warnings"), actual)) {
            failures.add("the warnings differ\n  expected: " + Suite.show(expectedResult.get("warnings")) + "\n  actual:   " + Suite.show(actual));
          }
        }
      }

      JsonNode verify = pipe == null ? null : pipe.bodyOf("verification/verify");
      JsonNode hooks = verify == null ? null : verify.path("target").get("hooks");
      if (expect.has("ops") && pipe != null && !Suite.equal(expect.get("ops"), Suite.MAPPER.valueToTree(pipe.ops()))) {
        failures.add("the operations sent differ\n  expected: " + Suite.show(expect.get("ops")) + "\n  actual:   " + pipe.ops());
      }
      expect.path("frames").properties().forEach(frame -> {
        JsonNode sent = pipe == null ? null : pipe.bodyOf(frame.getKey());
        if (!Suite.subset(frame.getValue(), sent)) {
          failures.add("the '" + frame.getKey() + "' frame body\n  expected (a subset of): " + Suite.show(frame.getValue())
              + "\n  actual: " + Suite.show(sent));
        }
      });
      if (expect.has("engine-closed") && pipe != null && pipe.closed() != expect.get("engine-closed").asBoolean()) {
        failures.add("the engine was " + (pipe.closed() ? "" : "not ") + "ended, and the case expects otherwise");
      }
      if (expect.has("hooks")) {
        JsonNode actual = hooks == null ? NullNode.getInstance() : normalised(hooks);
        if (!Suite.equal(expect.get("hooks"), actual)) {
          failures.add("target.hooks differs\n  expected: " + Suite.show(expect.get("hooks")) + "\n  actual:   " + Suite.show(actual));
        }
      }
      if (expect.path("config-loaded").asBoolean(false)) {
        JsonNode path = pipe == null ? null : pipe.bodyOf("config/load").path("path");
        if (path == null || !path.asText().equals(configFile.toString())) {
          failures.add("config/load carried " + Suite.show(path) + ", and the case expects the file's absolute path " + configFile);
        }
      }
      if (expect.has("invocations") && pipe != null && !Suite.equal(expect.get("invocations"), pipe.invocations())) {
        failures.add("the endpoint's answers differ\n  expected: " + Suite.show(expect.get("invocations"))
            + "\n  actual:   " + Suite.show(pipe.invocations()));
      }
      if (expect.has("handler-calls")) {
        JsonNode wanted = expect.get("handler-calls");
        boolean same = wanted.size() == calls.size();
        for (int i = 0; same && i < wanted.size(); i++) {
          same = Suite.subset(wanted.get(i), calls.get(i));
        }
        if (!same) {
          failures.add("the handlers were called differently\n  expected: " + Suite.show(wanted) + "\n  actual:   " + Suite.show(calls));
        }
      }
      if (expect.path("endpoint-closed").asBoolean(false)) {
        String endpoint = loopbackUrl(hooks);
        if (endpoint == null) {
          failures.add("the verify frame names no loopback endpoint to check");
        } else if (answers(endpoint)) {
          failures.add("the endpoint at " + endpoint + " still answers after verify ended");
        }
      }
    } finally {
      if (provider != null) {
        provider.stop(0);
      }
    }
  }

  /** A handler's behaviour, after recording the call. */
  private Map<String, Object> act(JsonNode behaviour, String handler, String action, Map<String, Object> params) {
    calls.addObject().put("handler", handler).put("action", action).set("params", Suite.MAPPER.valueToTree(params));
    if (behaviour.has("throws")) {
      throw new IllegalStateException(behaviour.get("throws").asText());
    }
    if (behaviour.has("unsupported")) {
      throw new StateUnsupportedException(behaviour.get("unsupported").asText());
    }
    return behaviour.has("returns") ? Suite.MAPPER.convertValue(behaviour.get("returns"), MAP) : null;
  }

  private static Path relative(Path file) {
    Path cwd = Path.of(System.getProperty("user.dir")).toAbsolutePath();
    try {
      return cwd.relativize(file);
    } catch (IllegalArgumentException differentRoot) {
      return file;
    }
  }

  /** target.hooks with each loopback url as {@code loopback:<path>} and each run token as {@code <token>} (README §4). */
  private static JsonNode normalised(JsonNode hooks) {
    JsonNode copy = hooks.deepCopy();
    copy.path("hooks").properties().forEach(point -> point.getValue().forEach(entry -> {
      if (entry.path("run") instanceof ObjectNode run) {
        String url = run.path("url").asText();
        if (url.startsWith("http://127.0.0.1:")) {
          run.put("url", "loopback:" + URI.create(url).getPath());
        }
        if (run.path("headers") instanceof ObjectNode headers && TOKEN.matcher(headers.path("authorization").asText()).matches()) {
          headers.put("authorization", "Bearer <token>");
        }
      }
    }));
    return copy;
  }

  private static String loopbackUrl(JsonNode hooks) {
    if (hooks == null) {
      return null;
    }
    for (Map.Entry<String, JsonNode> point : hooks.path("hooks").properties()) {
      for (JsonNode entry : point.getValue()) {
        String url = entry.path("run").path("url").asText();
        if (url.startsWith("http://127.0.0.1:")) {
          return url;
        }
      }
    }
    return null;
  }

  private static boolean answers(String url) {
    try {
      HttpClient.newHttpClient().send(HttpRequest.newBuilder(URI.create(url)).POST(HttpRequest.BodyPublishers.ofString("{}")).build(),
          HttpResponse.BodyHandlers.discarding());
      return true;
    } catch (IOException e) {
      return false;
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      return false;
    }
  }

  /** The live case's provider, on the loopback interface. */
  private static HttpServer serve(JsonNode provider) throws IOException {
    HttpServer server = HttpServer.create(new InetSocketAddress(InetAddress.getByName("127.0.0.1"), 0), 0);
    server.createContext("/", exchange -> {
      try (exchange) {
        JsonNode route = null;
        for (JsonNode candidate : provider.path("routes")) {
          if (route == null && candidate.path("method").asText().equals(exchange.getRequestMethod())
              && candidate.path("path").asText().equals(exchange.getRequestURI().getPath())) {
            route = candidate;
          }
        }
        if (route == null) {
          exchange.sendResponseHeaders(404, -1);
          return;
        }
        for (Map.Entry<String, JsonNode> required : route.path("require-headers").properties()) {
          if (!required.getValue().asText().equals(exchange.getRequestHeaders().getFirst(required.getKey()))) {
            exchange.sendResponseHeaders(401, -1);
            return;
          }
        }
        route.path("headers").properties().forEach(h -> exchange.getResponseHeaders().set(h.getKey(), h.getValue().asText()));
        byte[] body = route.has("body") ? route.get("body").toString().getBytes(StandardCharsets.UTF_8) : new byte[0];
        exchange.sendResponseHeaders(route.path("status").asInt(), body.length == 0 ? -1 : body.length);
        if (body.length > 0) {
          try (OutputStream out = exchange.getResponseBody()) {
            out.write(body);
          }
        }
      }
    });
    server.start();
    return server;
  }

  /** The case's message as this SDK's: bytes as a byte[], a string as itself, JSON as a Java value. */
  private static Message message(JsonNode script) {
    Object body = script.has("bytes") ? java.util.Base64.getDecoder().decode(script.get("bytes").asText())
        : script.has("text") ? script.get("text").asText()
        : Suite.MAPPER.convertValue(script.path("json"), Object.class);
    Message message = Message.of(body);
    if (script.has("content-type")) {
      message = message.withContentType(script.get("content-type").asText());
    }
    if (script.has("metadata")) {
      message = message.withMetadata(Suite.MAPPER.convertValue(script.get("metadata"), MAP));
    }
    return message;
  }
}
