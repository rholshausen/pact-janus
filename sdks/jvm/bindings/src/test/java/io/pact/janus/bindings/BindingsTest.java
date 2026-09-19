package io.pact.janus.bindings;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.pact.janus.bindings.contract.v1.InteractionSpec;
import io.pact.janus.bindings.contract.v1.ShapePart;
import io.pact.janus.bindings.contract.v1.State;
import io.pact.janus.bindings.protocol.v1.EngineError;
import io.pact.janus.bindings.protocol.v1.Event;
import io.pact.janus.bindings.protocol.v1.ResponseFrame;
import io.pact.janus.bindings.protocol.v1.Vocabulary;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

/**
 * The generated bindings (plan task 6.1) are only worth having if they are the protocol's own
 * types: these tests pin the properties spike 1.1's bindings round found and the SDK relies on.
 */
class BindingsTest {
  private static final ObjectMapper MAPPER = new ObjectMapper();
  private static final Path SPECS = Path.of(System.getProperty("janus.specs"));

  @Test
  void nameEveryOperationTheFrameSchemaKnows() throws Exception {
    JsonNode frame = MAPPER.readTree(SPECS.resolve("engine-protocol/schemas/v1/frame.schema.json").toFile());
    List<String> known = new ArrayList<>();
    frame.at("/$defs/RequestFrame/properties/op/x-known-values").forEach(v -> known.add(v.asText()));
    assertEquals(known, Vocabulary.RequestFrameOp.KNOWN);
    assertEquals("consumer-session/create", Vocabulary.RequestFrameOp.CONSUMER_SESSION_CREATE);
  }

  @Test
  void passADocumentThroughUnchangedIncludingMembersTheyDoNotKnow() throws Exception {
    // A newer engine's members (retryable, trace), and optional members left absent (Event's
    // 'last', whose schema default is false) must come back out exactly as they went in.
    String wire = """
        {"type":"response","id":"r-1","error":{"code":"interaction-invalid","message":"bad","retryable":false},"trace":"t-9"}""";
    ResponseFrame frame = MAPPER.readValue(wire, ResponseFrame.class);
    EngineError error = frame.getError();
    assertEquals(false, error.getAdditionalProperties().get("retryable"));
    assertEquals(MAPPER.readTree(wire), rewritten(frame));

    String event = """
        {"stream":"s-1","seq":1,"kind":"verification/started","payload":{}}""";
    assertEquals(MAPPER.readTree(event), rewritten(MAPPER.readValue(event, Event.class)));
  }

  /** What a binding writes, read back as a tree, so a Long and an int holding 1 compare equal. */
  private static JsonNode rewritten(Object binding) throws Exception {
    return MAPPER.readTree(MAPPER.writeValueAsString(binding));
  }

  @Test
  void typeTheInteractionSpecificationAnSdkBuilds() {
    ShapePart response = new ShapePart();
    response.setAdditionalProperty("body", Map.of("shape", "object", "members",
        Map.of("status", Map.of("shape", "any-of", "options", List.of("PENDING", "SHIPPED")))));
    State state = new State();
    state.setName("an order exists");
    InteractionSpec interaction = new InteractionSpec();
    interaction.setDescription("get an order");
    interaction.setStates(List.of(state));
    interaction.setParts(Map.of("response", response));

    JsonNode written = MAPPER.valueToTree(interaction);
    assertEquals("any-of", written.at("/parts/response/body/members/status/shape").asText());
    assertEquals(false, written.has("requires"));
  }
}
