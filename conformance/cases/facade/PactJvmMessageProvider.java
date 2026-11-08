// facade/pact-jvm-message-provider: pact-jvm's @PactVerifyProvider as the JVM SDK's @ProvidesMessage: a
// method asked for each message, given today's request for it.
import static org.junit.jupiter.api.Assertions.assertEquals;

import io.pact.janus.sdk.JanusProviderExtension;
import io.pact.janus.sdk.ProvidesMessage;
import io.pact.janus.sdk.Verifier;
import io.pact.janus.sdk.Message;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.stream.Stream;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.TestFactory;
import org.junit.jupiter.api.extension.RegisterExtension;

class PactJvmMessageProvider {
  static final List<Object> ASKED = new CopyOnWriteArrayList<>();

  @RegisterExtension
  static final JanusProviderExtension janus = JanusProviderExtension.of(
      () -> Verifier.forProvider("http://127.0.0.1:1").withPacts(List.of(pact())));

  @ProvidesMessage("an order placed event")
  Message orderPlaced(Map<String, Object> request) {
    ASKED.add(request.get("description"));
    return Message.of(Map.of("id", 1, "sku", "A-1"));
  }

  @TestFactory
  Stream<DynamicTest> pacts() {
    return janus.interactions(this);
  }

  @AfterAll
  static void askedForTheMessage() {
    assertEquals(List.of("an order placed event"), ASKED);
  }

  private static Path pact() {
    try {
      Path file = Files.createTempFile("pact-jvm-messages-", ".json");
      Files.writeString(file, """
          {
            "consumer": { "name": "web-app" },
            "provider": { "name": "orders-events" },
            "messages": [
              {
                "description": "an order placed event",
                "contents": { "id": 1, "sku": "A-1" },
                "metadata": { "contentType": "application/json" }
              }
            ],
            "metadata": { "pactSpecification": { "version": "3.0.0" } }
          }
          """);
      return file;
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }
}
