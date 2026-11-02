package io.pact.janus.sdk.frameworks;

import io.pact.janus.sdk.JanusExtension;
import io.pact.janus.sdk.JanusOptions;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;
import org.junit.jupiter.api.extension.RegisterExtension;

/**
 * The other of two classes recording one consumer and provider: see {@link JUnitMergedOrdersTest}.
 */
@EnabledIfSystemProperty(named = "janus.harness", matches = "true")
public class JUnitMergedDeletesTest {
  @RegisterExtension
  static final JanusExtension janus = JanusExtension.of("junit5-merged", "orders-api",
      JanusOptions.defaults().withContractDirectory(Path.of(System.getProperty("janus.test.contracts"))));

  @Test
  void deletesAnOrder() {
    janus.execute(janus.interaction("delete an order")
            .request(r -> r.method("DELETE").path("/orders/42"))
            .response(r -> r.status(204)),
        (mock, variant) -> Orders.delete(mock.url() + "/orders/42"));
  }
}
