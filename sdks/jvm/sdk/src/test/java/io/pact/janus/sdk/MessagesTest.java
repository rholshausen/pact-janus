package io.pact.janus.sdk;

import static io.pact.janus.sdk.Shapes.content;
import static io.pact.janus.sdk.Shapes.integer;
import static io.pact.janus.sdk.Shapes.optional;
import static io.pact.janus.sdk.Shapes.string;
import static io.pact.janus.sdk.VariantBinding.whenVariant;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * Message interactions in a consumer test (staged plan S2.14, ADR 0068), against the real engine: the
 * test is handed each variant's message, the contract records it, and the provider's message
 * producers verify that contract, each told by a provider-state parameter which variant to produce.
 */
class MessagesTest {
  @TempDir Path contracts;

  private Janus janus() {
    return Janus.of("shipping", "orders", JanusOptions.defaults().withContractDirectory(contracts));
  }

  /** The consumer's own code: what an order-placed event becomes. */
  @SuppressWarnings("unchecked")
  private static String label(Object event) {
    Map<String, Object> order = (Map<String, Object>) event;
    Object note = order.get("note");
    return note == null ? "order " + order.get("id") : "order " + order.get("id") + " (" + note + ")";
  }

  private static MessageInteraction orderPlaced(Janus janus) {
    return janus.message("an order was placed")
        .given("an order exists", Map.of("noted", whenVariant("note", "present")))
        .contents(slots -> slots
            .metadata("queue", "orders.placed")
            .body(Map.of("id", integer(42), "note", optional(string("fragile")))));
  }

  @Test
  @DisplayName("a message's variants are handed to the test, and the contract it records is verified by message producers")
  void recordedAndVerified() {
    Janus janus = janus();
    List<String> labels = new ArrayList<>();
    janus.execute(orderPlaced(janus), (message, variant) -> {
      assertEquals("application/json", message.contentType().orElseThrow());
      assertEquals(Map.of("queue", "orders.placed"), message.metadata());
      labels.add(label(message.json()));
    });
    assertEquals(List.of("order 42 (fragile)", "order 42"), labels);

    // A synchronous message: the request the consumer sends, and the reply it is handed.
    MessageInteraction ask = janus.message("a square's area")
        .request(slots -> slots.body(Map.of("side", 3)))
        .contents(slots -> slots.body(Map.of("area", integer(9))));
    janus.execute(ask, (reply, variant) -> {
      assertEquals(Map.of("side", 3), reply.request().orElseThrow().json());
      assertEquals(Map.of("area", 9), reply.json());
    });

    MessageInteraction cancelled = janus.message("an order was cancelled")
        .contents(slots -> slots.body(content("text/plain", "cancelled")));
    janus.execute(cancelled, (message, variant) -> {
      assertEquals("cancelled", message.text());
      assertEquals("text/plain", message.contentType().orElseThrow());
    });

    Path contract = janus.finalise().orElseThrow();

    List<Object> asked = new ArrayList<>();
    VerificationResult result = Verifier.forProvider("http://127.0.0.1:1")
        .withPacts(List.of(contract))
        .state("an order exists", params -> null)
        .message("an order was placed", request -> {
          asked.add(request.get("providerStates"));
          @SuppressWarnings("unchecked")
          Map<String, Object> state = ((List<Map<String, Object>>) request.get("providerStates")).get(0);
          @SuppressWarnings("unchecked")
          boolean noted = Boolean.TRUE.equals(((Map<String, Object>) state.get("params")).get("noted"));
          Map<String, Object> body = noted ? Map.of("id", 7, "note", "handle with care") : Map.of("id", 7);
          return Message.of(body).withContentType("application/json").withMetadata(Map.of("queue", "orders.placed"));
        })
        .message("a square's area", request -> {
          @SuppressWarnings("unchecked")
          Map<String, Object> contents = (Map<String, Object>) ((Map<String, Object>) request.get("request")).get("contents");
          @SuppressWarnings("unchecked")
          int side = (Integer) ((Map<String, Object>) contents.get("content")).get("side");
          return Message.of(Map.of("area", side * side)).withContentType("application/json");
        })
        .message("an order was cancelled", request -> Message.of("cancelled").withContentType("text/plain"))
        .verify();

    assertEquals("verified", result.status());
    assertEquals(4, result.results().size());
    assertEquals(List.of(
        List.of(Map.of("name", "an order exists", "params", Map.of("noted", true))),
        List.of(Map.of("name", "an order exists", "params", Map.of("noted", false)))), asked);
  }

  @Test
  @DisplayName("a variant the test fails on fails execute, naming it")
  void failedVariant() {
    Janus janus = janus();
    ExecuteFailedException failed = assertThrows(ExecuteFailedException.class,
        () -> janus.execute(orderPlaced(janus), (message, variant) -> {
          // A careless consumer: it assumes every order has a note.
          if (!label(message.json()).contains("(")) {
            throw new AssertionError("note is missing");
          }
        }));
    assertTrue(failed.getMessage().contains("message.body.note#presence=absent"), failed.getMessage());
    assertThrows(ContractWithheldException.class, janus::finalise);
  }
}
