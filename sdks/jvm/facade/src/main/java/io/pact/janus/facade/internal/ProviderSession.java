package io.pact.janus.facade.internal;

import io.pact.janus.bindings.protocol.v1.Vocabulary.RequestFrameOp;
import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.core.model.BasePact;
import io.pact.janus.facade.core.model.Interaction;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.messaging.Message;
import io.pact.janus.sdk.Engine;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * One provider's pact in one test, over one engine session (ADR 0070 decision 5): every interaction
 * added and its one variant armed, through an HTTP transport for an HTTP pact, or handed over as a
 * message for a message pact; then each variant's verdict reported and the contract merged into its
 * file. Not API.
 */
public final class ProviderSession implements MockServer {
  private record Armed(String handle, String variant, String description) {}

  private final Engine engine;
  private final BasePact pact;
  private final PactSpecVersion spec;
  private final String session;
  private final List<Armed> armed = new ArrayList<>();
  private final List<Message> messages = new ArrayList<>();
  private String url;
  private int port;

  /** Adds {@code pact}'s interactions and arms them; an HTTP pact's mock listens on {@code host}:{@code port}. */
  public ProviderSession(Engine engine, BasePact pact, PactSpecVersion spec, boolean http, String host, int port) {
    this.engine = engine;
    this.pact = pact;
    this.spec = spec;
    Map<String, Object> config = Map.of(
        "consumer", Map.of("name", pact.getConsumer().getName()),
        "provider", Map.of("name", pact.getProvider().getName()));
    session = (String) engine.send(RequestFrameOp.CONSUMER_SESSION_CREATE, Map.of("config", config)).get("session");
    if (http) {
      Map<String, Object> start = new LinkedHashMap<>();
      start.put("session", session);
      start.put("transport", "http");
      if (host != null || port > 0) {
        start.put("options", Map.of("host", host == null ? "127.0.0.1" : host, "port", port));
      }
      @SuppressWarnings("unchecked")
      Map<String, Object> endpoint = (Map<String, Object>) engine.send(RequestFrameOp.CONSUMER_SESSION_START_TRANSPORT,
          start).get("endpoint");
      this.port = ((Number) endpoint.get("port")).intValue();
      this.url = String.valueOf(endpoint.get("base-url"));
    }
    for (Interaction interaction : pact.getInteractions()) {
      arm(interaction);
    }
  }

  @SuppressWarnings("unchecked")
  private void arm(Interaction interaction) {
    String name = "'" + interaction.getDescription() + "'";
    Map<String, Object> steps = Map.of("specification", spec.forEngine(), "steps", interaction.calls(spec));
    Map<String, Object> added = engine.send(RequestFrameOp.CONSUMER_SESSION_ADD_PACT_INTERACTION,
        Map.of("session", session, "pact", steps));
    String handle = (String) added.get("handle");
    for (Map<String, Object> finding : (List<Map<String, Object>>) added.getOrDefault("findings", List.of())) {
      if ("lossy".equals(finding.get("kind"))) {
        System.err.println("pact-janus: " + name + " " + finding.get("path") + ": " + finding.get("message"));
      }
    }
    // A converted interaction's one variant is its example, `base`, as an upgraded pact's is (ADR 0066 decision 4).
    List<Map<String, Object>> variants = (List<Map<String, Object>>) engine.send(RequestFrameOp.CONSUMER_SESSION_VARIANTS,
        Map.of("session", session, "handle", handle)).get("variants");
    String variant = variants.isEmpty() ? "base" : (String) variants.get(0).get("id");
    Map<String, Object> served = engine.send(RequestFrameOp.CONSUMER_SESSION_SERVE_VARIANT,
        Map.of("session", session, "handle", handle, "variant", variant));
    if (interaction instanceof Message message) {
      if (served.get("parts") == null) {
        throw new IllegalStateException(name + ": the engine handed over no message, so it is older than ADR 0067's");
      }
      // pact-jvm hands a test the message as its builder built it, with its examples and its values'
      // own types, so the facade does too; serving it is what records it exercised (ADR 0070 decision 5).
      messages.add(message);
    }
    armed.add(new Armed(handle, variant, interaction.getDescription()));
  }

  public BasePact getPact() {
    return pact;
  }

  public List<Message> getMessages() {
    return messages;
  }

  @Override
  public String getUrl() {
    return url;
  }

  @Override
  public int getPort() {
    return port;
  }

  /**
   * Reports the test's verdict for every armed variant and merges the contract into {@code directory}.
   * Answers what the mock found wanting, as pact-jvm says it, or nothing when the provider's part passed.
   */
  @SuppressWarnings("unchecked")
  public List<String> finish(Throwable failure, Path directory) {
    List<String> notReceived = new ArrayList<>();
    List<String> mismatched = new ArrayList<>();
    for (Armed one : armed) {
      Map<String, Object> report = new LinkedHashMap<>();
      report.put("session", session);
      report.put("handle", one.handle());
      report.put("variant", one.variant());
      report.put("status", failure == null ? "passed" : "failed");
      if (failure != null) {
        report.put("reason", String.valueOf(failure.getMessage()));
      }
      Map<String, Object> result = (Map<String, Object>) engine.send(RequestFrameOp.CONSUMER_SESSION_REPORT_VARIANT,
          report).get("result");
      if ("not-exercised".equals(result.get("status"))) {
        notReceived.add(one.description());
      }
      for (Map<String, Object> mismatch : (List<Map<String, Object>>) result.getOrDefault("mismatches", List.of())) {
        Object path = mismatch.get("path");
        mismatched.add(one.description() + ": " + (path == null ? "" : path + ": ") + mismatch.get("message"));
      }
    }
    Path file = directory.resolve(pact.getConsumer().getName() + "-" + pact.getProvider().getName() + ".janus.json");
    Map<String, Object> finalised = engine.send(RequestFrameOp.CONSUMER_SESSION_FINALISE,
        Map.of("session", session, "write", Map.of("path", file.toAbsolutePath().toString(), "mode", "merge")));
    List<String> problems = new ArrayList<>();
    if (!mismatched.isEmpty()) {
      problems.add("The following mismatched requests occurred:\n" + String.join("\n", mismatched));
    }
    if (!notReceived.isEmpty()) {
      problems.add("The following requests were not received:\n" + String.join("\n", notReceived));
    }
    if (problems.isEmpty() && failure == null && finalised.get("written") == null) {
      problems.add("The engine withheld the contract with " + pact.getProvider().getName() + ": "
          + Json.write(finalised.getOrDefault("results", List.of())));
    }
    return problems;
  }
}
