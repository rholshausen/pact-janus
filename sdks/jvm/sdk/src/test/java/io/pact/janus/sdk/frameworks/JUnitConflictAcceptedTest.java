package io.pact.janus.sdk.frameworks;

import io.pact.janus.sdk.JanusExtension;
import io.pact.janus.sdk.JanusOptions;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;
import org.junit.jupiter.api.extension.RegisterExtension;

/**
 * The other of two classes recording 'create an order' differently: see {@link
 * JUnitConflictOkTest}.
 */
@EnabledIfSystemProperty(named = "janus.harness", matches = "true")
public class JUnitConflictAcceptedTest {
  @RegisterExtension
  static final JanusExtension janus = JanusExtension.of("junit5-conflicting", "orders-api",
      JanusOptions.defaults().withContractDirectory(Path.of(System.getProperty("janus.test.contracts"))));

  @Test
  void createsAnOrder() {
    janus.execute(janus.interaction("create an order")
            .request(r -> r.method("POST").path("/orders"))
            .response(r -> r.status(202)),
        (mock, variant) -> Orders.post(mock.url() + "/orders"));
  }
}
