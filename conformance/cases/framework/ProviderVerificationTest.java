// framework/provider-verifies-in-a-test, under JUnit 5: the provider is started for the class, and one
// test verifies it, the Verifier serving its state handler for the run.
import static org.junit.jupiter.api.Assertions.assertEquals;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.sun.net.httpserver.HttpServer;
import io.pact.janus.sdk.VerificationResult;
import io.pact.janus.sdk.Verifier;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;

class ProviderVerificationTest {
  static final Map<Integer, Map<String, Object>> orders = new ConcurrentHashMap<>();
  static HttpServer server;

  @BeforeAll
  static void start() throws IOException {
    server = HttpServer.create(new InetSocketAddress(InetAddress.getByName("127.0.0.1"), 0), 0);
    server.createContext("/orders/", exchange -> {
      Map<String, Object> order = orders.get(Integer.valueOf(exchange.getRequestURI().getPath().split("/")[2]));
      byte[] body = new ObjectMapper().writeValueAsBytes(order == null ? Map.of() : order);
      exchange.getResponseHeaders().add("Content-Type", "application/json");
      exchange.sendResponseHeaders(order == null ? 404 : 200, body.length);
      try (OutputStream out = exchange.getResponseBody()) {
        out.write(body);
      }
    });
    server.start();
  }

  @AfterAll
  static void stop() {
    server.stop(0);
  }

  @Test
  @DisplayName("verifies the orders API")
  void verifiesTheOrdersApi() throws Exception {
    VerificationResult result = Verifier.forProvider("http://127.0.0.1:" + server.getAddress().getPort())
        .withPacts(List.of(Path.of("pacts/web-app-orders-api.json")))
        .state("an order exists", params -> {
          int id = ((Number) params.get("id")).intValue();
          orders.put(id, Map.of("id", id, "status", "shipped"));
          return null;
        })
        .verify();
    assertEquals("verified", result.status());
  }
}
