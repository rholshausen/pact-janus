// facade/pact-jvm-matchers-collections: pact-jvm's array and map matchers, an array with no rule, and
// bodies whose root is a value — each served as its example.
import static org.junit.jupiter.api.Assertions.assertEquals;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslJsonArray;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.consumer.dsl.PactDslJsonRootValue;
import io.pact.janus.facade.consumer.dsl.PactDslRootValue;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit5.PactConsumerTestExt;
import io.pact.janus.facade.consumer.junit5.PactTestFor;
import io.pact.janus.facade.core.model.RequestResponsePact;
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
class PactJvmMatchersCollections {
  @Pact(consumer = "web-app")
  RequestResponsePact collections(PactDslWithProvider builder) {
    return builder
        .uponReceiving("orders of every array").path("/orders").method("GET")
        .willRespondWith().status(200)
        .body(new PactDslJsonBody()
            .eachLike("any").integerType("id", 1).closeObject().closeArray()
            .minArrayLike("some", 2).stringType("sku", "A-1").closeObject().closeArray()
            .maxArrayLike("few", 3).stringType("sku", "B-2").closeObject().closeArray()
            .minMaxArrayLike("between", 1, 4).stringType("sku", "C-3").closeObject().closeArray()
            .array("exact").stringValue("first").stringValue("second").closeArray())
        .uponReceiving("orders of nested arrays").path("/batches").method("GET")
        .willRespondWith().status(200)
        .body(new PactDslJsonArray()
            .eachArrayLike().stringType("A-1").closeArray()
            .eachArrayWithMinLike(2).stringType("B-2").closeArray()
            .eachArrayWithMaxLike(3).stringType("C-3").closeArray()
            .eachArrayWithMinMaxLike(1, 2).stringType("D-4").closeArray())
        .uponReceiving("orders by key").path("/orders/by-id").method("GET")
        .willRespondWith().status(200)
        .body(new PactDslJsonBody()
            .object("byId").eachKeyLike("order-1").integerType("total", 10).closeObject().closeObject()
            .object("labels").eachValueLike("first", PactDslJsonRootValue.stringType("new")).closeObject()
            .object("codes").eachValueMatching("first").stringMatcher("code", "[A-Z]-\\d+", "A-1").closeObject().closeObject()
            .object("lines").eachKeyMappedToAnArrayLike("line-1").integerType("qty", 1).closeObject().closeArray().closeObject())
        .uponReceiving("an order count").path("/orders/count").method("GET")
        .willRespondWith().status(200)
        .body(PactDslJsonRootValue.integerType(12))
        .uponReceiving("an order's status as text").path("/orders/1/status").method("GET")
        .willRespondWith().status(200)
        .headers(Map.of("Content-Type", "text/plain"))
        .body(PactDslRootValue.stringMatcher("open|closed", "open"))
        .toPact();
  }

  @Test
  void servesEachExample(MockServer mockServer) throws Exception {
    String url = mockServer.getUrl();
    assertEquals(true, get(url + "/orders").contains("\"exact\":[\"first\",\"second\"]"));
    assertEquals(true, get(url + "/batches").startsWith("[["));
    assertEquals(true, get(url + "/orders/by-id").contains("\"order-1\""));
    assertEquals("12", get(url + "/orders/count"));
    assertEquals("open", get(url + "/orders/1/status"));
  }

  private static String get(String url) throws Exception {
    return HttpClient.newHttpClient()
        .send(HttpRequest.newBuilder(URI.create(url)).GET().build(), HttpResponse.BodyHandlers.ofString()).body();
  }
}
