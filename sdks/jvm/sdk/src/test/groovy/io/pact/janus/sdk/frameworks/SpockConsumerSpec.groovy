package io.pact.janus.sdk.frameworks

import static io.pact.janus.sdk.Shapes.integer
import static io.pact.janus.sdk.Shapes.json
import static io.pact.janus.sdk.Shapes.map
import static io.pact.janus.sdk.Shapes.optional
import static io.pact.janus.sdk.Shapes.string

import io.pact.janus.sdk.Interaction
import io.pact.janus.sdk.Janus
import io.pact.janus.sdk.JanusOptions
import java.nio.file.Path
import spock.lang.AutoCleanup
import spock.lang.Requires
import spock.lang.Shared
import spock.lang.Specification

/**
 * A consumer suite under Spock: the specification's one {@code Janus} is a shared field, which Spock
 * finalises after the last feature, and {@code execute} takes a closure.
 */
@Requires({ sys["janus.harness"] == "true" })
class SpockConsumerSpec extends Specification {
  @Shared @AutoCleanup("finalise")
  Janus janus = Janus.of("spock", "orders-api",
      JanusOptions.defaults().withContractDirectory(Path.of(System.getProperty("janus.test.contracts"))))

  @Shared
  Interaction getOrder = janus.interaction("get an order")
      .request { it.method("GET").path("/orders/42") }
      .response { it.status(200).body(json(map("id", integer(42), "shippedAt", optional(string("2026-07-30"))))) }

  def "gets an order"() {
    expect:
    janus.execute(getOrder) { mock, variant ->
      assert Orders.get("${mock.url()}/orders/42").contains("42")
    }
  }
}
