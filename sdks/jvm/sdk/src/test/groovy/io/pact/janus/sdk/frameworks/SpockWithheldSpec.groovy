package io.pact.janus.sdk.frameworks

import static io.pact.janus.sdk.Shapes.integer
import static io.pact.janus.sdk.Shapes.json
import static io.pact.janus.sdk.Shapes.map
import static io.pact.janus.sdk.Shapes.optional
import static io.pact.janus.sdk.Shapes.string

import com.fasterxml.jackson.databind.ObjectMapper
import io.pact.janus.sdk.Interaction
import io.pact.janus.sdk.Janus
import io.pact.janus.sdk.JanusOptions
import java.nio.file.Path
import spock.lang.AutoCleanup
import spock.lang.Requires
import spock.lang.Shared
import spock.lang.Specification

/**
 * A specification whose contract the engine withholds: one feature's client crashes on the variant
 * without {@code shippedAt}, so that feature fails, the other passes, and finalise fails the spec.
 */
@Requires({ sys["janus.harness"] == "true" })
class SpockWithheldSpec extends Specification {
  @Shared @AutoCleanup("finalise")
  Janus janus = Janus.of("spock-careless", "orders-api",
      JanusOptions.defaults().withContractDirectory(Path.of(System.getProperty("janus.test.contracts"))))

  @Shared
  Interaction getOrder = janus.interaction("get an order")
      .request { it.method("GET").path("/orders/42") }
      .response { it.status(200).body(json(map("id", integer(42), "shippedAt", optional(string("2026-07-30"))))) }

  @Shared
  Interaction deleteOrder = janus.interaction("delete an order")
      .request { it.method("DELETE").path("/orders/42") }
      .response { it.status(204) }

  def "reads when an order shipped"() {
    expect:
    janus.execute(getOrder) { mock, variant ->
      def order = new ObjectMapper().readValue(Orders.get("${mock.url()}/orders/42"), Map)
      order.shippedAt.substring(0, 4)
    }
  }

  def "deletes an order"() {
    expect:
    janus.execute(deleteOrder) { mock, variant -> Orders.delete("${mock.url()}/orders/42") }
  }
}
