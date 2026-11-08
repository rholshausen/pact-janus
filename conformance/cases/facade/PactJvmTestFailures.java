// facade/pact-jvm-test-failures: a request the mock did not match, an interaction never called and a
// test that throws each fail their test as pact-jvm's do, recording no contract; a @Pact method no test
// ran fails the class; and @PactIgnore leaves a test alone.
import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit5.PactConsumerTestExt;
import io.pact.janus.facade.consumer.junit5.PactIgnore;
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
@PactTestFor(providerName = "orders-api")
class PactJvmTestFailures {
  private static RequestResponsePact getOrder(PactDslWithProvider builder, String description) {
    return builder.uponReceiving(description).path("/orders/1").method("GET")
        .willRespondWith().status(200).body("{\"id\": 1}", "application/json").toPact();
  }

  @Pact(consumer = "web-app")
  RequestResponsePact mismatched(PactDslWithProvider builder) {
    return getOrder(builder, "get an order at the wrong path");
  }

  @Pact(consumer = "web-app")
  RequestResponsePact notCalled(PactDslWithProvider builder) {
    return getOrder(builder, "get an order nobody asks for");
  }

  @Pact(consumer = "web-app")
  RequestResponsePact throwing(PactDslWithProvider builder) {
    return getOrder(builder, "get an order for a test that throws");
  }

  @Pact(consumer = "web-app")
  RequestResponsePact unused(PactDslWithProvider builder) {
    return getOrder(builder, "get an order no test asks for");
  }

  @Test
  @PactTestFor(pactMethod = "mismatched")
  void requestMismatched(MockServer mockServer) throws Exception {
    HttpClient.newHttpClient().send(HttpRequest.newBuilder(URI.create(mockServer.getUrl() + "/orders/2")).GET().build(),
        HttpResponse.BodyHandlers.discarding());
  }

  @Test
  @PactTestFor(pactMethod = "notCalled")
  void requestMissing(MockServer mockServer) {
  }

  @Test
  @PactTestFor(pactMethod = "throwing")
  void testThrows(MockServer mockServer) {
    throw new IllegalStateException("the client could not read the order");
  }

  @Test
  @PactIgnore
  void ignored() {
  }
}
