package io.pact.janus.sdk.frameworks;

import static io.pact.janus.sdk.Shapes.integer;
import static io.pact.janus.sdk.Shapes.json;
import static io.pact.janus.sdk.Shapes.map;
import static io.pact.janus.sdk.Shapes.optional;
import static io.pact.janus.sdk.Shapes.string;

import com.fasterxml.jackson.databind.ObjectMapper;
import io.pact.janus.sdk.Interaction;
import io.pact.janus.sdk.JanusExtension;
import io.pact.janus.sdk.JanusOptions;
import java.nio.file.Path;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;
import org.junit.jupiter.api.extension.RegisterExtension;

/**
 * A suite whose contract the engine withholds: one test's client crashes on the variant without
 * {@code shippedAt}, so that test fails, the other passes, and finalise fails the class.
 */
@EnabledIfSystemProperty(named = "janus.harness", matches = "true")
public class JUnitWithheldTest {
  @RegisterExtension
  static final JanusExtension janus = JanusExtension.of("junit5-careless", "orders-api",
      JanusOptions.defaults().withContractDirectory(Path.of(System.getProperty("janus.test.contracts"))));

  static final Interaction getOrder = janus.interaction("get an order")
      .request(r -> r.method("GET").path("/orders/42"))
      .response(r -> r.status(200).body(json(map("id", integer(42), "shippedAt", optional(string("2026-07-30"))))));

  static final Interaction deleteOrder = janus.interaction("delete an order")
      .request(r -> r.method("DELETE").path("/orders/42"))
      .response(r -> r.status(204));

  @Test
  void readsWhenAnOrderShipped() {
    janus.execute(getOrder, (mock, variant) -> {
      Map<?, ?> order = new ObjectMapper().readValue(Orders.get(mock.url() + "/orders/42"), Map.class);
      ((String) order.get("shippedAt")).substring(0, 4);
    });
  }

  @Test
  void deletesAnOrder() {
    janus.execute(deleteOrder, (mock, variant) -> Orders.delete(mock.url() + "/orders/42"));
  }
}
