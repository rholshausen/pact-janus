package io.pact.janus.sdk;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import java.nio.file.Path;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

/**
 * Where the verifier finds janus-plugin-bridge (behavioural spec 'verify', ADR 0112 decision 5): its own
 * option, else {@code JANUS_PLUGIN_BRIDGE}. The conformance cases cover the option, which every SDK runs;
 * the variable is read here, since a JVM cannot set its own environment.
 */
class VerifierPluginBridgeTest {
  private static Map<String, Object> options(String named, Map<String, String> environment) {
    Map<String, Object> options = new LinkedHashMap<>();
    Verifier.addPluginBridge(options, named, environment);
    return options;
  }

  @Test
  void isTheVariablesWhenNoOptionNamesOneResolvedAgainstTheWorkingDirectory() {
    Map<String, Object> options = options(null, Map.of("JANUS_PLUGIN_BRIDGE", "tools/janus-plugin-bridge"));
    assertEquals(
        Map.of(
            "source", Map.of("kind", "subprocess", "reference",
                Path.of(System.getProperty("user.dir")).resolve("tools/janus-plugin-bridge").toString()),
            "grants", Map.of("env", List.of("PACT_PLUGIN_DIR", "HOME", "USERPROFILE", "PACT_DO_NOT_TRACK", "RUST_LOG")),
            "limits", Map.of("deadline-ms", 30_000)),
        options.get("plugin-bridge"));
  }

  @Test
  void isTheOptionsBeforeTheVariables() {
    String named = Path.of("from-the-option").toAbsolutePath().toString();
    Map<String, Object> options = options(named, Map.of("JANUS_PLUGIN_BRIDGE", "/from/the/environment"));
    @SuppressWarnings("unchecked")
    Map<String, Object> source = (Map<String, Object>) ((Map<String, Object>) options.get("plugin-bridge")).get("source");
    assertEquals(named, source.get("reference"));
  }

  @Test
  void isNotDeclaredWhenNothingNamesOneOrTheVariableIsEmpty() {
    assertFalse(options(null, Map.of()).containsKey("plugin-bridge"));
    assertFalse(options(null, Map.of("JANUS_PLUGIN_BRIDGE", "")).containsKey("plugin-bridge"));
  }

  @Test
  void isTheRunsOwnWhenItsOptionsGiveOne() {
    Map<String, Object> given = Map.of("source", Map.of("kind", "subprocess", "reference", "/srv/bridge"));
    Map<String, Object> options = new LinkedHashMap<>(Map.of("plugin-bridge", given));
    Verifier.addPluginBridge(options, "/opt/janus-plugin-bridge", Map.of());
    assertEquals(given, options.get("plugin-bridge"));
  }
}
