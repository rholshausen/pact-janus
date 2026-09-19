package io.pact.janus.sdk;

import static io.pact.janus.sdk.Shapes.anyOf;
import static io.pact.janus.sdk.Shapes.date;
import static io.pact.janus.sdk.Shapes.datetime;
import static io.pact.janus.sdk.Shapes.eachLike;
import static io.pact.janus.sdk.Shapes.integer;
import static io.pact.janus.sdk.Shapes.json;
import static io.pact.janus.sdk.Shapes.map;
import static io.pact.janus.sdk.Shapes.oneOf;
import static io.pact.janus.sdk.Shapes.optional;
import static io.pact.janus.sdk.Shapes.regex;
import static io.pact.janus.sdk.Shapes.string;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.RegisterExtension;

/**
 * The RFC's consumer example (SDK spec examples/order-example-mapping.md §1), written as a JVM
 * consumer would write it, against the real engine. Its contract lands in
 * {@code build/contracts/web-app-orders-api.janus.json}; {@link EndToEndTest} checks the same run's
 * outcomes in detail.
 */
class OrderConsumerTest {
  @RegisterExtension
  static final JanusExtension janus = JanusExtension.of("web-app", "orders-api",
      JanusOptions.defaults().withContractDirectory(Path.of(System.getProperty("janus.test.contracts", "contracts"))));

  static Interaction getOrder(JanusExtension janus) {
    return janus.interaction("get an order")
        .given("an order exists", map("id", "42"))
        .request(r -> r.method("GET").path("/orders/42"))
        .response(r -> r
            .status(200)
            .body(json(map(
                "id", integer(42),
                "status", anyOf("PENDING", "SHIPPED", "DELIVERED"),
                "shippedAt", optional(datetime("2026-07-30T10:00:00Z")),
                "payment", oneOf("type", map(
                    "card", map("type", "card", "last4", regex("\\d{4}", "1234")),
                    "invoice", map("type", "invoice", "dueDate", date("2026-08-30")))),
                "items", eachLike(map("sku", string("SKU-1"), "qty", integer(1)), Cardinality.min(1))))));
  }

  @Test
  void getsAnOrder() {
    janus.execute(getOrder(janus), (mock, variant) -> {
      OrderClient client = OrderClient.at(mock.url());
      OrderClient.Order order = client.getOrder("42");
      assertTrue(order.lineCount() > 0);
    });
  }
}
