package io.pact.janus.releasecheck;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit5.PactConsumerTestExt;
import io.pact.janus.facade.consumer.junit5.PactTestFor;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

/**
 * The release's `janus-facade` jar, resolved and used as a user moving a pact-jvm suite would (staged
 * plan S2.5b, ADR 0070): a consumer test written in pact-jvm's Java DSL under JUnit 5, its imports
 * changed, records a contract where pact-jvm writes its pacts. The engine is the one the release pins.
 */
@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "release-check-orders")
class FacadeReleaseTest {
  @Pact(consumer = "release-check-facade")
  RequestResponsePact anOrder(PactDslWithProvider builder) {
    return builder
        .given("an order exists")
        .uponReceiving("get an order, in pact-jvm's DSL")
        .path("/orders/42")
        .method("GET")
        .willRespondWith()
        .status(200)
        .body(new PactDslJsonBody().integerType("id", 42).stringType("status", "SHIPPED"))
        .toPact();
  }

  @Test
  void aPactJvmConsumerTestRunsOnTheFacade(MockServer mockServer) throws Exception {
    HttpResponse<String> response = HttpClient.newHttpClient().send(
        HttpRequest.newBuilder(URI.create(mockServer.getUrl() + "/orders/42")).GET().build(),
        HttpResponse.BodyHandlers.ofString());
    assertEquals(200, response.statusCode());
    assertTrue(response.body().contains("\"SHIPPED\""), response.body());
  }

  /** The extension writes the contract after each test, so it is there once they have all run. */
  @AfterAll
  static void theContractIsWrittenWherePactJvmWritesItsPacts() throws Exception {
    Path contract = Path.of(System.getProperty("pact.rootDir"),
        "release-check-facade-release-check-orders.janus.json");
    assertTrue(Files.exists(contract), contract + " was not written");
    assertTrue(Files.readString(contract).contains("get an order, in pact-jvm's DSL"), Files.readString(contract));
  }
}
