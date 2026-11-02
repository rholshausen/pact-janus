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
 * Both classes' interactions recorded in one, into a directory of its own: the merged contract
 * must be these bytes.
 */
@EnabledIfSystemProperty(named = "janus.harness", matches = "true")
public class JUnitMergedTogetherTest {
  @RegisterExtension
  static final JanusExtension janus = JanusExtension.of("junit5-merged", "orders-api",
      JanusOptions.defaults()
          .withContractDirectory(Path.of(System.getProperty("janus.test.contracts")).resolve("together")));

  @Test
  void deletesAnOrder() {
    janus.execute(janus.interaction("delete an order")
            .request(r -> r.method("DELETE").path("/orders/42"))
            .response(r -> r.status(204)),
        (mock, variant) -> Orders.delete(mock.url() + "/orders/42"));
  }

  @Test
  void getsAnOrder() {
    janus.execute(janus.interaction("get an order")
            .request(r -> r.method("GET").path("/orders/42"))
            .response(r -> r.status(200).body(json(map("id", integer(42))))),
        (mock, variant) -> Orders.get(mock.url() + "/orders/42"));
  }
}
