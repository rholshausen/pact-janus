// framework/test-per-variant, under Spock: a feature whose where: block is janus.variants runs once per
// variant the engine selects, each row run with janus.run and named by its variant. The client reads
// shippedAt as if it were always there, so the variant without it fails its own row, the base passes,
// and the withheld contract fails the specification.
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

class VariantTestsSpec extends Specification {
  @Shared @AutoCleanup("finalise")
  Janus janus = Janus.of("web-app", "orders-api")

  @Shared
  Interaction getOrder = janus.interaction("get an order")
      .request { it.method("GET").path("/orders/42") }
      .response { it.status(200).body(json(map("id", integer(42), "shippedAt", optional(string("2026-07-30"))))) }

  def "#variant"() {
    expect:
    janus.run(variant) { mock, selected ->
      def order = new ObjectMapper().readValue(URI.create("${mock.url()}/orders/42").toURL(), Map)
      assert order.shippedAt != null : "shippedAt is missing"
    }

    where:
    variant << janus.variants(getOrder)
  }
}
