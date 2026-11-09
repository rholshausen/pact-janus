// framework/withheld-contract-fails-the-suite, under JUnit 5: one test's client crashes on the variant
// without shippedAt, so that test fails, the other passes, and the extension's afterAll fails the class
// with the contract the engine withheld.
import static io.pact.janus.sdk.Shapes.integer;
import static io.pact.janus.sdk.Shapes.json;
import static io.pact.janus.sdk.Shapes.map;
import static io.pact.janus.sdk.Shapes.optional;
import static io.pact.janus.sdk.Shapes.string;
import static org.junit.jupiter.api.Assertions.assertEquals;

import com.fasterxml.jackson.databind.ObjectMapper;
import io.pact.janus.sdk.Interaction;
import io.pact.janus.sdk.JanusExtension;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.Map;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.RegisterExtension;

class WithheldContractTest {
  @RegisterExtension
  static final JanusExtension janus = JanusExtension.of("web-app", "orders-api");

  static final HttpClient http = HttpClient.newHttpClient();

  static final Interaction getOrder = janus.interaction("get an order")
      .request(r -> r.method("GET").path("/orders/42"))
      .response(r -> r.status(200).body(json(map("id", integer(42), "shippedAt", optional(string("2026-07-30"))))));

  static final Interaction deleteOrder = janus.interaction("delete an order")
      .request(r -> r.method("DELETE").path("/orders/42"))
      .response(r -> r.status(204));

  @Test
  @DisplayName("reads when an order shipped")
  void readsWhenAnOrderShipped() {
    janus.execute(getOrder, (mock, variant) -> {
      HttpRequest request = HttpRequest.newBuilder(URI.create(mock.url() + "/orders/42")).GET().build();
      Map<?, ?> order = new ObjectMapper().readValue(http.send(request, HttpResponse.BodyHandlers.ofString()).body(), Map.class);
      assertEquals("2026", ((String) order.get("shippedAt")).substring(0, 4));
    });
  }

  @Test
  @DisplayName("deletes an order")
  void deletesAnOrder() {
    janus.execute(deleteOrder, (mock, variant) -> {
      HttpRequest request = HttpRequest.newBuilder(URI.create(mock.url() + "/orders/42")).DELETE().build();
      assertEquals(204, http.send(request, HttpResponse.BodyHandlers.discarding()).statusCode());
    });
  }
}
