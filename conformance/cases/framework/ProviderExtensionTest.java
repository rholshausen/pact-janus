// framework/provider-extension-test-per-interaction, under JUnit 5: JanusProviderExtension runs one
// verification, and each interaction it verifies is a test of its own, its state set up by a @State
// method. The provider has no invoice endpoint, so that interaction's test fails and the other passes.
import com.fasterxml.jackson.databind.ObjectMapper;
import com.sun.net.httpserver.HttpServer;
import io.pact.janus.sdk.JanusProviderExtension;
import io.pact.janus.sdk.State;
import io.pact.janus.sdk.Verifier;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Stream;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.TestFactory;
import org.junit.jupiter.api.extension.RegisterExtension;

class ProviderExtensionTest {
  static final Map<Integer, Map<String, Object>> orders = new ConcurrentHashMap<>();
  static HttpServer server;

  @RegisterExtension
  static final JanusProviderExtension janus = JanusProviderExtension.of(
      () -> Verifier.forProvider("http://127.0.0.1:" + server.getAddress().getPort())
          .withPacts(List.of(Path.of("pacts/web-app-orders-api.json"))));

  @BeforeAll
  static void start() throws IOException {
    server = HttpServer.create(new InetSocketAddress(InetAddress.getByName("127.0.0.1"), 0), 0);
    server.createContext("/orders/", exchange -> {
      String[] path = exchange.getRequestURI().getPath().split("/");
      Map<String, Object> order = path.length == 3 ? orders.get(Integer.valueOf(path[2])) : null;
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

  @State("an order exists")
  void anOrderExists(Map<String, Object> params) {
    int id = ((Number) params.get("id")).intValue();
    orders.put(id, Map.of("id", id, "status", "shipped"));
  }

  @TestFactory
  Stream<DynamicTest> pacts() {
    return janus.interactions(this);
  }
}
