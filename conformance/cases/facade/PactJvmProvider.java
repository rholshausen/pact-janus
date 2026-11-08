// facade/pact-jvm-provider: pact-jvm's provider test as the JVM SDK's JUnit 5 extension runs it: its
// @State methods set up and tear down each state, and a failed interaction fails its own test.
import static org.junit.jupiter.api.Assertions.assertEquals;

import com.sun.net.httpserver.HttpServer;
import io.pact.janus.sdk.JanusProviderExtension;
import io.pact.janus.sdk.State;
import io.pact.janus.sdk.Verifier;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.stream.Stream;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.TestFactory;
import org.junit.jupiter.api.extension.RegisterExtension;

class PactJvmProvider {
  static final List<String> CALLS = new CopyOnWriteArrayList<>();
  static final List<Integer> ORDERS = new CopyOnWriteArrayList<>();
  static final HttpServer SERVER = provider();

  @RegisterExtension
  static final JanusProviderExtension janus = JanusProviderExtension.of(
      () -> Verifier.forProvider("http://127.0.0.1:" + SERVER.getAddress().getPort()).withPacts(List.of(pact())));

  @State("an order exists")
  Map<String, Object> anOrderExists(Map<String, Object> params) {
    CALLS.add("setup " + params);
    ORDERS.add(((Number) params.get("id")).intValue());
    return Map.of("id", params.get("id"));
  }

  @State(value = "an order exists", action = State.Action.TEARDOWN)
  void noOrders() {
    CALLS.add("teardown");
    ORDERS.clear();
  }

  @State("orders are archived")
  void archived() {
    CALLS.add("archived");
  }

  @TestFactory
  Stream<DynamicTest> pacts() {
    return janus.interactions(this);
  }

  @AfterAll
  static void calledEachState() {
    SERVER.stop(0);
    assertEquals(List.of("setup {id=42}", "teardown", "archived"), new ArrayList<>(CALLS));
  }

  private static HttpServer provider() {
    try {
      HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
      server.createContext("/", exchange -> {
        String path = exchange.getRequestURI().getPath();
        boolean found = path.equals("/orders/42") && ORDERS.contains(42);
        byte[] body = (found ? "{\"id\":42}" : "{}").getBytes(StandardCharsets.UTF_8);
        exchange.getResponseHeaders().add("Content-Type", "application/json");
        exchange.sendResponseHeaders(found ? 200 : 404, body.length);
        exchange.getResponseBody().write(body);
        exchange.close();
      });
      server.start();
      return server;
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }

  private static Path pact() {
    try {
      Path file = Files.createTempFile("pact-jvm-provider-", ".json");
      Files.writeString(file, """
          {
            "consumer": { "name": "web-app" },
            "provider": { "name": "orders-api" },
            "interactions": [
              {
                "description": "get an order",
                "providerStates": [{ "name": "an order exists", "params": { "id": 42 } }],
                "request": { "method": "GET", "path": "/orders/42" },
                "response": { "status": 200, "headers": { "Content-Type": "application/json" }, "body": { "id": 42 } }
              },
              {
                "description": "get an archived order",
                "providerStates": [{ "name": "orders are archived" }],
                "request": { "method": "GET", "path": "/orders/7" },
                "response": { "status": 200, "headers": { "Content-Type": "application/json" }, "body": { "id": 7 } }
              }
            ],
            "metadata": { "pactSpecification": { "version": "3.0.0" } }
          }
          """);
      return file;
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }
}
