package io.pact.janus.sdk.frameworks;

import static io.pact.janus.sdk.Shapes.integer;
import static io.pact.janus.sdk.Shapes.json;
import static io.pact.janus.sdk.Shapes.map;

import io.pact.janus.sdk.JanusExtension;
import io.pact.janus.sdk.JanusOptions;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;
import org.junit.jupiter.api.extension.RegisterExtension;

/**
 * One of two classes recording one consumer and provider, each with an engine of its own: the
 * engine merges both into one contract (ADR 0065).
 */
@EnabledIfSystemProperty(named = "janus.harness", matches = "true")
public class JUnitMergedOrdersTest {
  @RegisterExtension
  static final JanusExtension janus = JanusExtension.of("junit5-merged", "orders-api",
      JanusOptions.defaults().withContractDirectory(Path.of(System.getProperty("janus.test.contracts"))));

  @Test
  void getsAnOrder() {
    janus.execute(janus.interaction("get an order")
            .request(r -> r.method("GET").path("/orders/42"))
            .response(r -> r.status(200).body(json(map("id", integer(42))))),
        (mock, variant) -> Orders.get(mock.url() + "/orders/42"));
  }
}
