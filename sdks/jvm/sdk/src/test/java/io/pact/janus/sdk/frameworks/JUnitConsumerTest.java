package io.pact.janus.sdk.frameworks;

import static io.pact.janus.sdk.Shapes.integer;
import static io.pact.janus.sdk.Shapes.json;
import static io.pact.janus.sdk.Shapes.map;
import static io.pact.janus.sdk.Shapes.optional;
import static io.pact.janus.sdk.Shapes.string;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.pact.janus.sdk.Interaction;
import io.pact.janus.sdk.JanusExtension;
import io.pact.janus.sdk.JanusOptions;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;
import org.junit.jupiter.api.extension.RegisterExtension;

/** A consumer suite under JUnit 5: one extension per class, {@code execute} in a test. */
@EnabledIfSystemProperty(named = "janus.harness", matches = "true")
public class JUnitConsumerTest {
  @RegisterExtension
  static final JanusExtension janus = JanusExtension.of("junit5", "orders-api",
      JanusOptions.defaults().withContractDirectory(Path.of(System.getProperty("janus.test.contracts"))));

  static final Interaction getOrder = janus.interaction("get an order")
      .request(r -> r.method("GET").path("/orders/42"))
      .response(r -> r.status(200).body(json(map("id", integer(42), "shippedAt", optional(string("2026-07-30"))))));

  @Test
  void getsAnOrder() {
    janus.execute(getOrder, (mock, variant) -> assertTrue(Orders.get(mock.url() + "/orders/42").contains("42")));
  }
}
