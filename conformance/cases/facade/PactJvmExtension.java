// facade/pact-jvm-extension: pact-jvm's JUnit 5 consumer extension, its @Pact methods returning a
// RequestResponsePact and a V4Pact, @PactTestFor on the class and the method, and the pact injected.
import static org.junit.jupiter.api.Assertions.assertEquals;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit5.PactConsumerTestExt;
import io.pact.janus.facade.consumer.junit5.PactTestFor;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.V4Pact;
import io.pact.janus.facade.core.model.annotations.Pact;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "orders-api")
class PactJvmExtension {
  @Pact(consumer = "web-app", provider = "orders-api")
  RequestResponsePact orders(PactDslWithProvider builder) {
    return builder
        .given("an order exists", Map.of("id", 1))
        .given("the customer is known")
        .uponReceiving("get an order")
        .path("/orders/1")
        .method("GET")
        .willRespondWith()
        .status(200)
        .body("{\"id\": 1}", "application/json")
        .given("no orders exist")
        .uponReceiving("list no orders")
        .path("/orders")
        .method("GET")
        .willRespondWith()
        .status(200)
        .body("[]", "application/json")
        .toPact();
  }

  @Pact(consumer = "web-app", provider = "orders-api")
  V4Pact cancel(PactDslWithProvider builder) {
    return builder
        .uponReceiving("cancel an order")
        .path("/orders/1")
        .method("DELETE")
        .willRespondWith()
        .status(204)
        .toPact(V4Pact.class);
  }

  @Test
  @PactTestFor(pactMethod = "orders")
  void getsOrders(MockServer mockServer, RequestResponsePact pact) throws Exception {
    assertEquals(2, pact.getInteractions().size());
    assertEquals("{\"id\": 1}", send("GET", mockServer.getUrl() + "/orders/1").body());
    assertEquals("[]", send("GET", mockServer.getUrl() + "/orders").body());
  }

  @Test
  @PactTestFor(pactMethod = "cancel", pactVersion = PactSpecVersion.V4)
  void cancelsAnOrder(MockServer mockServer, V4Pact pact) throws Exception {
    assertEquals(1, pact.getInteractions().size());
    assertEquals(204, send("DELETE", mockServer.getUrl() + "/orders/1").statusCode());
  }

  private static HttpResponse<String> send(String method, String url) throws Exception {
    HttpRequest request = HttpRequest.newBuilder(URI.create(url)).method(method, HttpRequest.BodyPublishers.noBody()).build();
    return HttpClient.newHttpClient().send(request, HttpResponse.BodyHandlers.ofString());
  }
}
