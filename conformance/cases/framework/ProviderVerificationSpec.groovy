// framework/provider-verifies-in-a-test, under Spock: the provider is started for the specification,
// and one feature verifies it, the Verifier serving its state handler, a closure, for the run.
import com.fasterxml.jackson.databind.ObjectMapper
import com.sun.net.httpserver.HttpServer
import io.pact.janus.sdk.Verifier
import java.nio.file.Path
import java.util.concurrent.ConcurrentHashMap
import spock.lang.Shared
import spock.lang.Specification

class ProviderVerificationSpec extends Specification {
  @Shared
  Map<Integer, Map> orders = new ConcurrentHashMap<>()

  @Shared
  HttpServer server = HttpServer.create(new InetSocketAddress(InetAddress.getByName("127.0.0.1"), 0), 0)

  def setupSpec() {
    server.createContext("/orders/") { exchange ->
      def order = orders[exchange.requestURI.path.split("/")[2] as Integer]
      byte[] body = new ObjectMapper().writeValueAsBytes(order ?: [:])
      exchange.responseHeaders.add("Content-Type", "application/json")
      exchange.sendResponseHeaders(order ? 200 : 404, body.length)
      exchange.responseBody.withCloseable { it.write(body) }
    }
    server.start()
  }

  def cleanupSpec() {
    server.stop(0)
  }

  def "verifies the orders API"() {
    when:
    def result = Verifier.forProvider("http://127.0.0.1:${server.address.port}")
        .withPacts([Path.of("pacts/web-app-orders-api.json")])
        .state("an order exists") { params ->
          orders[params.id as Integer] = [id: params.id, status: "shipped"]
          null
        }
        .verify()

    then:
    result.status() == "verified"
  }
}
