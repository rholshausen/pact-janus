// live/exchange-events-logged, in Java: the client asks for the wrong order, and the line the SDK logs
// to io.pact.janus.engine says so while the test runs.
import io.pact.janus.sdk.Interaction;
import io.pact.janus.sdk.Janus;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import org.junit.jupiter.api.Test;

class ExchangeEventsLogged {
  @Test
  void aRejectedRequestIsLogged() {
    try (Janus janus = Janus.of("web-app", "orders-api")) {
      Interaction getOrder = janus.interaction("get an order")
          .request(r -> r.method("GET").path("/orders/42"))
          .response(r -> r.status(200));
      janus.execute(getOrder, (mock, variant) -> HttpClient.newHttpClient().send(
          HttpRequest.newBuilder(URI.create(mock.url() + "/orders/7")).GET().build(),
          HttpResponse.BodyHandlers.discarding()));
    }
  }
}
