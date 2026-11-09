// framework/consumer-suite, under Spock: the specification's one Janus is a shared field Spock
// finalises after the last feature, and execute takes a closure.
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

class ConsumerSuiteSpec extends Specification {
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

  def "gets an order"() {
    expect:
    janus.execute(getOrder) { mock, variant ->
      def order = new ObjectMapper().readValue(URI.create("${mock.url()}/orders/42").toURL(), Map)
      assert order.id == 42
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
