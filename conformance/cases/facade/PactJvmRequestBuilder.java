// facade/pact-jvm-request-builder: pact-jvm's request and response builders, from ConsumerPactBuilder
// itself: paths, methods, queries and headers, their regex forms, the default request and response
// values, values from provider state, and the status classes.
import static org.junit.jupiter.api.Assertions.assertEquals;

import io.pact.janus.facade.consumer.ConsumerPactBuilder;
import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslRequestWithoutPath;
import io.pact.janus.facade.consumer.dsl.PactDslResponse;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit5.PactConsumerTestExt;
import io.pact.janus.facade.consumer.junit5.PactTestFor;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "orders-api")
class PactJvmRequestBuilder {
  @Pact(consumer = "web-app")
  RequestResponsePact requests(PactDslWithProvider ignored) {
    PactDslWithProvider builder = ConsumerPactBuilder.consumer("web-app").pactSpecVersion(PactSpecVersion.V3)
        .hasPactWith("orders-api");
    builder.setDefaultRequestValues(new PactDslRequestWithoutPath().headers("Accept", "application/json"));
    builder.setDefaultResponseValues(new PactDslResponse().headers(Map.of("Cache-Control", "no-store")));
    return builder
        .given("orders exist")
        .uponReceiving("search orders")
        .path("/orders")
        .method("GET")
        .query("status=open&page=2")
        .headers("X-Request-Id", "r-1")
        .willRespondWith()
        .status(200)
        .headers(Map.of("Content-Type", "application/json"))
        .body("[]")
        .uponReceiving("search orders by encoded query")
        .path("/orders")
        .method("GET")
        .encodedQuery("q=a%20b")
        .willRespondWith()
        .successStatus()
        .uponReceiving("get orders by pattern")
        .matchPath("/orders/\\d+", "/orders/7")
        .method("GET")
        .matchHeader("X-Tenant", "[a-z]+", "acme")
        .matchQuery("sort", "asc|desc", "asc")
        .queryMatchingDate("from", "yyyy-MM-dd", "2026-11-01")
        .queryMatchingTime("at", "HH:mm", "09:30")
        .queryMatchingDatetime("since", "yyyy-MM-dd'T'HH:mm:ss", "2026-11-01T09:30:00")
        .queryMatchingISODate("on", "2026-11-02")
        .queryMatchingISOTime("until", "10:00:00")
        .queryMatchingISODatetime("after", "2026-11-01T09:30:00Z")
        .willRespondWith()
        .statusCodes(List.of(200, 203))
        .matchHeader("ETag", "\"\\w+\"", "\"abc\"")
        .matchSetCookie("session", "[0-9a-f]+", "a1b2")
        .given("an order exists", Map.of("id", 42))
        .uponReceiving("get the order the state made")
        .pathFromProviderState("/orders/${id}", "/orders/42")
        .method("GET")
        .headerFromProviderState("X-Order", "${id}", "42")
        .queryParameterFromProviderState("expand", "${expand}", "lines")
        .willRespondWith()
        .clientErrorStatus()
        .toPact();
  }

  @Test
  void sendsEachRequest(MockServer mockServer) throws Exception {
    String url = mockServer.getUrl();
    assertEquals(200, get(url + "/orders?status=open&page=2", "X-Request-Id", "r-1"));
    assertEquals(200, get(url + "/orders?q=a%20b"));
    assertEquals(200, get(url + "/orders/9?sort=desc&from=2026-12-01&at=10:45&since=2026-12-01T10:45:00"
        + "&on=2026-12-02&until=11:00:00&after=2026-12-01T10:45:00Z", "X-Tenant", "other"));
    assertEquals(400, get(url + "/orders/42?expand=lines", "X-Order", "42"));
  }

  private static int get(String url, String... header) throws Exception {
    HttpRequest.Builder request = HttpRequest.newBuilder(URI.create(url)).GET().header("Accept", "application/json");
    for (int i = 0; i < header.length; i += 2) {
      request.header(header[i], header[i + 1]);
    }
    return HttpClient.newHttpClient().send(request.build(), HttpResponse.BodyHandlers.discarding()).statusCode();
  }
}
