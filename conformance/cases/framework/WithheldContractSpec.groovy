// framework/withheld-contract-fails-the-suite, under Spock: one feature's client crashes on the variant
// without shippedAt, so that feature fails, the other passes, and Spock's cleanup of the shared Janus
// fails the specification with the contract the engine withheld.
import static io.pact.janus.sdk.Shapes.integer
import static io.pact.janus.sdk.Shapes.json
import static io.pact.janus.sdk.Shapes.map
import static io.pact.janus.sdk.Shapes.optional
import static io.pact.janus.sdk.Shapes.string

import com.fasterxml.jackson.databind.ObjectMapper
import io.pact.janus.sdk.Interaction
import io.pact.janus.sdk.Janus
import spock.lang.AutoCleanup
import spock.lang.Shared
import spock.lang.Specification

class WithheldContractSpec extends Specification {
  @Shared @AutoCleanup("finalise")
  Janus janus = Janus.of("web-app", "orders-api")

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
      def order = new ObjectMapper().readValue(URI.create("${mock.url()}/orders/42").toURL(), Map)
      assert order.shippedAt.substring(0, 4) == "2026"
    }
  }

  def "deletes an order"() {
    expect:
    janus.execute(deleteOrder) { mock, variant ->
      def connection = URI.create("${mock.url()}/orders/42").toURL().openConnection() as HttpURLConnection
      connection.requestMethod = "DELETE"
      assert connection.responseCode == 204
    }
  }
}
