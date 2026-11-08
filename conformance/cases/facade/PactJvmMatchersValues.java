// facade/pact-jvm-matchers-values: pact-jvm's matchers of single values in a PactDslJsonBody — types
// and their plural forms, number kinds, equality, patterns, dates and times, includes, null, a value
// from provider state, and several rules at once — each served as its example.
import static org.junit.jupiter.api.Assertions.assertEquals;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit5.PactConsumerTestExt;
import io.pact.janus.facade.consumer.junit5.PactTestFor;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import io.pact.janus.facade.core.model.matchingrules.IncludeMatcher;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.model.matchingrules.TypeMatcher;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.LocalDate;
import java.util.Date;
import java.util.TimeZone;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "orders-api")
class PactJvmMatchersValues {
  private static final TimeZone UTC = TimeZone.getTimeZone("UTC");
  private static final Date NOON = new Date(1_762_689_600_000L);

  @Pact(consumer = "web-app")
  RequestResponsePact values(PactDslWithProvider builder) {
    return builder
        .uponReceiving("an order of typed values").path("/orders/1").method("GET")
        .willRespondWith().status(200)
        .body(new PactDslJsonBody()
            .stringType("sku", "A-1")
            .like("note", "fragile")
            .numberType("weight", 2)
            .booleanType("paid", true)
            .stringTypes("first", "last")
            .numberTypes("height")
            .booleanTypes("gift")
            .integerType("id", 1)
            .decimalType("total", 9.99)
            .integerType("lines")
            .decimalType("tax")
            .stringValue("currency", "EUR")
            .numberValue("version", 3)
            .booleanValue("express", false)
            .equalTo("channel", "web"))
        .uponReceiving("an order of patterned values").path("/orders/2").method("GET")
        .willRespondWith().status(200)
        .body(new PactDslJsonBody()
            .stringMatcher("code", "\\d{3}-\\d{2}", "123-45")
            .uuid("ref", "ce118b6e-d8e1-11e7-9296-cec278b6b50a")
            .hexValue("colour", "3F")
            .ipAddress("origin")
            .integerMatching("count", "\\d+", 12)
            .decimalMatching("price", "\\d+\\.\\d{2}", 9.99)
            .numberMatching("rate", "\\d+(\\.\\d+)?", 1.5)
            .includesStr("comment", "fragile")
            .nullValue("coupon")
            .valueFromProviderState("customer", "${customerId}", 42)
            .and("postcode", "AB1 2CD", new RegexMatcher("[A-Z]{2}\\d \\d[A-Z]{2}"), TypeMatcher.INSTANCE)
            .and("district", "AB1", new RegexMatcher("[A-Z]{2}\\d"), new IncludeMatcher("AB")))
        .uponReceiving("an order of dates").path("/orders/3").method("GET")
        .willRespondWith().status(200)
        .body(new PactDslJsonBody()
            .date("due", "yyyy-MM-dd", NOON, UTC)
            .time("slot", "HH:mm", NOON, UTC)
            .datetime("placed", "yyyy-MM-dd'T'HH:mm:ss", NOON, UTC)
            .localDate("delivered", "yyyy-MM-dd", LocalDate.of(2026, 11, 20)))
        .toPact();
  }

  @Test
  void servesEachExample(MockServer mockServer) throws Exception {
    String typed = get(mockServer.getUrl() + "/orders/1");
    assertEquals(true, typed.contains("\"sku\":\"A-1\""), typed);
    String patterned = get(mockServer.getUrl() + "/orders/2");
    assertEquals(true, patterned.contains("\"code\":\"123-45\""), patterned);
    String dated = get(mockServer.getUrl() + "/orders/3");
    assertEquals(true, dated.contains("\"delivered\":\"2026-11-20\""), dated);
  }

  private static String get(String url) throws Exception {
    return HttpClient.newHttpClient()
        .send(HttpRequest.newBuilder(URI.create(url)).GET().build(), HttpResponse.BodyHandlers.ofString()).body();
  }
}
