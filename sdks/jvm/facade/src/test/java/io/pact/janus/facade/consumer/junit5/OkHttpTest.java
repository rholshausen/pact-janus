// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../OkHttpTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.V4Pact;
import io.pact.janus.facade.core.model.annotations.Pact;
import okhttp3.OkHttpClient;
import okhttp3.Request;
import okhttp3.RequestBody;
import okhttp3.MediaType;
import okhttp3.Response;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

import java.io.IOException;
import java.util.Map;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.equalTo;
import static org.hamcrest.Matchers.is;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "example-provider", pactVersion = PactSpecVersion.V4)
public class OkHttpTest {
  private final OkHttpClient client = new OkHttpClient();

  @Pact(consumer = "example-consumer")
  public V4Pact createOrderPact(PactDslWithProvider builder) {
    return builder
      .given("provider accepts an order")
      .uponReceiving("a request to create an order")
      .path("/orders")
      .method("POST")
      .headers(Map.of("Content-Type", "application/json"))
      .body(new PactDslJsonBody()
        .stringType("customerId", "cust-123")
        .decimalType("amount", 100.00))
      .willRespondWith()
      .status(200)
      .headers(Map.of("Content-Type", "application/json"))
      .body(new PactDslJsonBody()
        .stringType("orderId", "order-456")
        .stringValue("status", "CREATED"))
      .toPact(V4Pact.class);
  }

  @Test
  @PactTestFor(pactMethod = "createOrderPact")
  void should_create_order(MockServer mockServer) throws IOException {
    Request request = new Request.Builder()
      .url(mockServer.getUrl() + "/orders")
      .post(RequestBody.create(
        "{\"customerId\":\"cust-123\",\"amount\":100.00}",
        MediaType.get("application/json")))
      .build();

    try (Response response = client.newCall(request).execute()) {
      assertThat(response.code(), is(equalTo(200)));
    }
  }
}
