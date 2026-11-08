// facade/pact-jvm-mock-server-config: the mock's address, by @MockServerConfig and by @PactTestFor,
// each port read from a system property, as pact-jvm reads `${name}`.
import static org.junit.jupiter.api.Assertions.assertEquals;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit.MockServerConfig;
import io.pact.janus.facade.consumer.junit5.PactConsumerTest;
import io.pact.janus.facade.consumer.junit5.PactTestFor;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.net.ServerSocket;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import org.junit.jupiter.api.Test;

@PactConsumerTest
@PactTestFor(providerName = "orders-api")
class PactJvmMockServerConfig {
  static final int CLASS_PORT = freePort();
  static final int METHOD_PORT = freePort();

  static {
    // Where a build would set them: the ports a CI agent has free.
    System.setProperty("orders.mock.port", String.valueOf(CLASS_PORT));
    System.setProperty("orders.method.port", String.valueOf(METHOD_PORT));
  }

  @Pact(consumer = "web-app")
  RequestResponsePact ping(PactDslWithProvider builder) {
    return builder.uponReceiving("ping the mock at its address").path("/ping").method("GET")
        .willRespondWith().status(204).toPact();
  }

  @Pact(consumer = "web-app")
  RequestResponsePact pong(PactDslWithProvider builder) {
    return builder.uponReceiving("pong the mock at the method's address").path("/pong").method("GET")
        .willRespondWith().status(204).toPact();
  }

  @Test
  @PactTestFor(pactMethod = "ping")
  @MockServerConfig(hostInterface = "127.0.0.1", port = "${orders.mock.port}")
  void mockServerConfigsAddress(MockServer mockServer) throws Exception {
    assertEquals(CLASS_PORT, mockServer.getPort());
    assertEquals("http://127.0.0.1:" + CLASS_PORT, mockServer.getUrl());
    assertEquals(204, get("http://127.0.0.1:" + CLASS_PORT + "/ping"));
  }

  @Test
  @PactTestFor(pactMethod = "pong", hostInterface = "127.0.0.1", port = "${orders.method.port}")
  void pactTestForsAddress(MockServer mockServer) throws Exception {
    assertEquals(METHOD_PORT, mockServer.getPort());
    assertEquals(204, get("http://127.0.0.1:" + METHOD_PORT + "/pong"));
  }

  private static int get(String url) throws Exception {
    return HttpClient.newHttpClient()
        .send(HttpRequest.newBuilder(URI.create(url)).GET().build(), HttpResponse.BodyHandlers.discarding())
        .statusCode();
  }

  private static int freePort() {
    try (ServerSocket socket = new ServerSocket(0)) {
      return socket.getLocalPort();
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }
}
