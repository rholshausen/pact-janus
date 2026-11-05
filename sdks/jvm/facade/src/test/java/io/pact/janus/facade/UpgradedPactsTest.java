package io.pact.janus.facade;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.pact.janus.sdk.Engine;
import io.pact.janus.sdk.JanusOptions;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.Tag;
import org.junit.jupiter.api.TestFactory;

/**
 * S2.5b's exit (ADR 0070): every contract pact-jvm's ported consumer tests wrote is what
 * {@code janus upgrade} writes from the pact pact-jvm 4.7.8 wrote running the same tests, interaction
 * by interaction. Runs after those tests, as the {@code compareContracts} task.
 */
@Tag("contracts")
class UpgradedPactsTest {
  private static final ObjectMapper JSON = new ObjectMapper();

  /** Contracts that cannot be compared yet, each with why. */
  private static final Map<String, String> KNOWN = Map.of(
      "test_consumer-QueryWithSquareBrackets.janus.json",
      "pact-reference's pact_models, which reads every v1-v4 pact the engine reads, as today's Rust verifier does, "
          + "cannot read pact-jvm's V2 rule keyed $.query['identifier[other]']",
      "consumer-provider2.janus.json",
      "pact-jvm's Groovy MultiProviderTest (S2.5d) writes an interaction of the same name to this pact, and "
          + "pact-jvm's merge kept its");

  @TestFactory
  Stream<DynamicTest> eachContractIsWhatUpgradeWritesFromPactJvmsPact() throws IOException {
    Path contracts = Path.of(System.getProperty("pact.rootDir"));
    Path pacts = Path.of(System.getProperty("janus.facade.pacts"));
    List<Path> written;
    try (Stream<Path> files = Files.walk(contracts)) {
      written = files.filter(f -> f.toString().endsWith(".janus.json")).sorted().toList();
    }
    assertFalse(written.isEmpty(), "the ported tests wrote no contracts to " + contracts);
    return written.stream().map(contract -> {
      Path relative = contracts.relativize(contract);
      Path pact = pacts.resolve(relative.toString().replace(".janus.json", ".json"));
      String known = KNOWN.get(relative.toString().replace('\\', '/'));
      if (known != null) {
        // A known exception must still fail, so a fix shows up as one to delete.
        return DynamicTest.dynamicTest(relative + " (" + known + ")",
            () -> org.junit.jupiter.api.Assertions.assertThrows(Throwable.class, () -> compare(contract, pact), known));
      }
      return DynamicTest.dynamicTest(relative.toString(), () -> compare(contract, pact));
    });
  }

  private static void compare(Path contract, Path pact) throws IOException {
    JsonNode recorded = JSON.readTree(contract.toFile());
    JsonNode upgraded;
    try (Engine engine = Engine.start(JanusOptions.defaults())) {
      Map<String, Object> answer = engine.send("upgrade/pact", Map.of("pact", JSON.readValue(pact.toFile(), Map.class)));
      // The SDK declares `contract-text`, so the engine answers the contract as its bytes (ADR 0063).
      Object text = answer.get("contract-text");
      upgraded = text != null ? JSON.readTree((String) text) : JSON.valueToTree(answer.get("contract"));
    }
    Map<String, JsonNode> expected = byIdentity(upgraded);
    for (Map.Entry<String, JsonNode> interaction : byIdentity(recorded).entrySet()) {
      JsonNode wanted = expected.get(interaction.getKey());
      if (wanted != null) {
        wanted = withDrawnExamples(wanted, interaction.getValue());
      }
      JsonNode got = documents(interaction.getValue());
      if (wanted == null || !documents(wanted).equals(got)) {
        assertEquals(wanted == null ? null : wanted.toPrettyString(), interaction.getValue().toPrettyString(),
            interaction.getKey());
      }
    }
  }

  /**
   * {@code upgraded} with each example pact-jvm drew at random from a pattern replaced by the one the
   * facade drew from it (ADR 0070 decision 3): where both have a {@code regex} shape whose examples
   * differ and each matches the pattern, the facade's is put everywhere pact-jvm's appears.
   */
  private static JsonNode withDrawnExamples(JsonNode upgraded, JsonNode recorded) throws IOException {
    Map<String, String> drawn = new LinkedHashMap<>();
    collectDrawn(upgraded, recorded, drawn);
    String text = upgraded.toString();
    for (Map.Entry<String, String> example : drawn.entrySet()) {
      text = text.replace(JSON.writeValueAsString(example.getKey()), JSON.writeValueAsString(example.getValue()));
    }
    return JSON.readTree(text);
  }

  private static void collectDrawn(JsonNode upgraded, JsonNode recorded, Map<String, String> drawn) {
    if (upgraded == null || recorded == null) {
      return;
    }
    if (upgraded.isObject() && "regex".equals(upgraded.path("shape").asText())
        && "regex".equals(recorded.path("shape").asText())
        && upgraded.path("pattern").equals(recorded.path("pattern"))
        && upgraded.path("example").isTextual() && recorded.path("example").isTextual()) {
      String pattern = upgraded.path("pattern").asText();
      String theirs = upgraded.path("example").asText();
      String ours = recorded.path("example").asText();
      if (!theirs.equals(ours) && theirs.matches(pattern) && ours.matches(pattern)) {
        drawn.put(theirs, ours);
      }
    }
    if (upgraded.isObject()) {
      upgraded.fieldNames().forEachRemaining(name -> collectDrawn(upgraded.get(name), recorded.get(name), drawn));
    } else if (upgraded.isArray()) {
      for (int i = 0; i < upgraded.size(); i++) {
        collectDrawn(upgraded.get(i), recorded.get(i), drawn);
      }
    }
  }

  /**
   * An interaction with each recorded JSON body's text read as its document. Member order decides nothing
   * (ADR 0037): pact-jvm's pact writer sorts members, and a test's own text keeps its order. Objects compare
   * whatever their members' order.
   */
  private static JsonNode documents(JsonNode interaction) {
    JsonNode copy = interaction.deepCopy();
    copy.path("selection").path("variants").forEach(variant -> variant.path("parts").forEach(part -> {
      JsonNode body = part.get("body");
      if (body != null && "text".equals(body.path("encoded").asText())
          && body.path("content-type").asText().contains("json")) {
        try {
          ((com.fasterxml.jackson.databind.node.ObjectNode) body).set("content", JSON.readTree(body.get("content").asText()));
        } catch (IOException e) {
          // Not JSON after all: compared as its text.
        }
      }
    }));
    return copy;
  }

  /** A contract's interactions by description and states, which is how today's pacts tell them apart. */
  private static Map<String, JsonNode> byIdentity(JsonNode contract) {
    Map<String, JsonNode> interactions = new LinkedHashMap<>();
    contract.get("interactions").forEach(interaction -> interactions.put(
        interaction.get("description").asText() + " " + interaction.path("states"), interaction));
    return interactions;
  }
}
