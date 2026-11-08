// facade/pact-jvm-several-providers: one test against two providers, a mock for each, configured by
// @MockServers, each mock injected by @ForProvider, and each provider's contract recorded apart.
import static org.junit.jupiter.api.Assertions.assertEquals;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit.MockServerConfig;
import io.pact.janus.facade.consumer.junit.MockServers;
import io.pact.janus.facade.consumer.junit5.ForProvider;
import io.pact.janus.facade.consumer.junit5.PactConsumerTestExt;
import io.pact.janus.facade.consumer.junit5.PactTestFor;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

@ExtendWith(PactConsumerTestExt.class)
@MockServers({
    @MockServerConfig(providerName = "orders-api", hostInterface = "127.0.0.1"),
    @MockServerConfig(providerName = "stock-api", hostInterface = "127.0.0.1")
})
class PactJvmSeveralProviders {
  @Pact(consumer = "web-app", provider = "orders-api")
  RequestResponsePact order(PactDslWithProvider builder) {
    return builder.uponReceiving("get an order's sku").path("/orders/1").method("GET")
        .willRespondWith().status(200).body("{\"sku\": \"A-1\"}", "application/json").toPact();
  }

  @Pact(consumer = "web-app", provider = "stock-api")
  RequestResponsePact stock(PactDslWithProvider builder) {
    return builder.uponReceiving("get a sku's stock").path("/stock/A-1").method("GET")
        .willRespondWith().status(200).body("{\"level\": 3}", "application/json").toPact();
  }

  @Test
  @PactTestFor(pactMethods = {"order", "stock"})
  void asksBothProviders(@ForProvider("orders-api") MockServer orders, @ForProvider("stock-api") MockServer stock)
      throws Exception {
    assertEquals("{\"sku\": \"A-1\"}", get(orders.getUrl() + "/orders/1"));
    assertEquals("{\"level\": 3}", get(stock.getUrl() + "/stock/A-1"));
  }

  private static String get(String url) throws Exception {
    return HttpClient.newHttpClient()
        .send(HttpRequest.newBuilder(URI.create(url)).GET().build(), HttpResponse.BodyHandlers.ofString())
        .body();
  }
}
