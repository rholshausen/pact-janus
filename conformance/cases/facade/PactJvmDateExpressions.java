// facade/pact-jvm-date-expressions: pact-jvm's date, time and datetime expressions, and a timestamp at
// the root, each served as the example pact-jvm generates from today's date.
import static org.junit.jupiter.api.Assertions.assertEquals;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.consumer.dsl.PactDslJsonRootValue;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit5.PactConsumerTestExt;
import io.pact.janus.facade.consumer.junit5.PactTestFor;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.Date;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "orders-api")
class PactJvmDateExpressions {
  @Pact(consumer = "web-app")
  RequestResponsePact expressions(PactDslWithProvider builder) {
    return builder
        .uponReceiving("an order of date expressions").path("/orders/1").method("GET")
        .willRespondWith().status(200)
        .body(new PactDslJsonBody()
            .dateExpression("due", "today + 1 day", "yyyy-MM-dd")
            .timeExpression("slot", "now + 1 hour", "HH:mm")
            .datetimeExpression("ships", "tomorrow", "yyyy-MM-dd'T'HH:mm:ss"))
        .uponReceiving("an order's timestamp").path("/orders/1/stamped").method("GET")
        .willRespondWith().status(200)
        .body(PactDslJsonRootValue.timestamp("yyyy-MM-dd HH:mm:ss", new Date()))
        .toPact();
  }

  @Test
  void servesTodaysExamples(MockServer mockServer) throws Exception {
    String body = get(mockServer.getUrl() + "/orders/1");
    assertEquals(true, body.matches(".*\"due\":\"\\d{4}-\\d{2}-\\d{2}\".*") && body.contains("\"slot\":\"")
        && body.contains("\"ships\":\""), body);
    assertEquals(true, get(mockServer.getUrl() + "/orders/1/stamped").matches("\"\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}:\\d{2}\""));
  }

  private static String get(String url) throws Exception {
    return HttpClient.newHttpClient()
        .send(HttpRequest.newBuilder(URI.create(url)).GET().build(), HttpResponse.BodyHandlers.ofString()).body();
  }
}
