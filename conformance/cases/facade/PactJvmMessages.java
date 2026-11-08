// facade/pact-jvm-messages: pact-jvm's MessagePactBuilder, its contents as a DslPart, a String with a
// content type and XML, its metadata as a Map and a MetadataBuilder, handed to the test as a
// List<Message>, a MessagePact, and a V4 pact's List<V4Interaction.AsynchronousMessage>; no mock.
import static org.junit.jupiter.api.Assertions.assertEquals;

import io.pact.janus.facade.consumer.MessagePactBuilder;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.consumer.junit5.PactConsumerTestExt;
import io.pact.janus.facade.consumer.junit5.PactTestFor;
import io.pact.janus.facade.consumer.junit5.ProviderType;
import io.pact.janus.facade.consumer.xml.PactXmlBuilder;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.V4Interaction;
import io.pact.janus.facade.core.model.V4Pact;
import io.pact.janus.facade.core.model.annotations.Pact;
import io.pact.janus.facade.core.model.messaging.Message;
import io.pact.janus.facade.core.model.messaging.MessagePact;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "orders-events", providerType = ProviderType.ASYNCH, pactVersion = PactSpecVersion.V3)
class PactJvmMessages {
  @Pact(consumer = "web-app")
  MessagePact placed(MessagePactBuilder builder) {
    return builder
        .given("an order was placed", Map.of("id", 1))
        .expectsToReceive("an order placed event")
        .withMetadata(Map.of("topic", "orders", "contentType", "application/json; charset=UTF-8"))
        .withContent(new PactDslJsonBody().integerType("id", 1).stringType("sku", "A-1"))
        .toPact();
  }

  @Pact(consumer = "web-app")
  MessagePact noted(MessagePactBuilder builder) {
    return builder
        .expectsToReceive("an order note")
        .withMetadata(metadata -> metadata.add("queue", "notes"))
        .withContent("call before delivery", "text/plain")
        .toPact();
  }

  @Pact(consumer = "web-app")
  MessagePact labelled(MessagePactBuilder builder) {
    return builder
        .expectsToReceive("an order label")
        .withContent(new PactXmlBuilder("label").build(root -> root.appendElement("sku", Map.of(), "A-1")))
        .toPact();
  }

  @Pact(consumer = "web-app")
  V4Pact shipped(MessagePactBuilder builder) {
    return builder
        .expectsToReceive("an order shipped event")
        .withContent(new PactDslJsonBody().stringValue("status", "shipped"))
        .toPact(V4Pact.class);
  }

  @Test
  @PactTestFor(pactMethod = "placed")
  void aListOfMessages(List<Message> messages) {
    assertEquals("{\"id\":1,\"sku\":\"A-1\"}", messages.get(0).contentsAsString());
    assertEquals("orders", messages.get(0).getMetadata().get("topic"));
  }

  @Test
  @PactTestFor(pactMethod = "noted")
  void aMessagePact(MessagePact pact) {
    Message note = pact.getMessages().get(0);
    assertEquals("call before delivery", new String(note.contentsAsBytes(), StandardCharsets.UTF_8));
    assertEquals("notes", note.getMetadata().get("queue"));
  }

  @Test
  @PactTestFor(pactMethod = "labelled")
  void anXmlMessage(List<Message> messages) {
    assertEquals(true, messages.get(0).contentsAsString().contains("<sku>A-1</sku>"));
  }

  @Test
  @PactTestFor(pactMethod = "shipped", pactVersion = PactSpecVersion.V4)
  void aV4PactsMessages(List<V4Interaction.AsynchronousMessage> messages) {
    assertEquals("{\"status\":\"shipped\"}", messages.get(0).contentsAsString());
  }
}
