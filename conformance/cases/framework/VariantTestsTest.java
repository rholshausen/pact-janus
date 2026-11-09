// framework/junit5-test-per-variant, under JUnit 5: a @TestFactory returns janus.variants, so each
// variant the engine selects is a test of its own. The client reads shippedAt as if it were always
// there, so the variant without it fails with the closure's own exception, the base passes, and the withheld contract fails the class.
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
import java.util.stream.Stream;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.TestFactory;
import org.junit.jupiter.api.extension.RegisterExtension;

class VariantTestsTest {
  @RegisterExtension
  static final JanusExtension janus = JanusExtension.of("web-app", "orders-api");

  static final HttpClient http = HttpClient.newHttpClient();

  static final Interaction getOrder = janus.interaction("get an order")
      .request(r -> r.method("GET").path("/orders/42"))
      .response(r -> r.status(200).body(json(map("id", integer(42), "shippedAt", optional(string("2026-07-30"))))));

  @TestFactory
  @DisplayName("reads when an order shipped")
  Stream<DynamicTest> readsWhenAnOrderShipped() {
    return janus.variants(getOrder, (mock, variant) -> {
      HttpRequest request = HttpRequest.newBuilder(URI.create(mock.url() + "/orders/42")).GET().build();
      Map<?, ?> order = new ObjectMapper().readValue(http.send(request, HttpResponse.BodyHandlers.ofString()).body(), Map.class);
      assertEquals("2026", ((String) order.get("shippedAt")).substring(0, 4));
    });
  }
}
