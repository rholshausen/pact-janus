package io.pact.janus.sdk.frameworks;

import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Collectors;

/**
 * The provider the framework suites' provider tests verify: pact-js's User Service, on the JDK's HTTP
 * server, answering {@code GET /users} and {@code GET /users/<id>} from the users its states seed.
 * Its pact is a fixture of S1.1's corpus.
 */
public final class UserService implements AutoCloseable {
  /** The pact the provider tests verify. */
  public static final Path PACT = Path.of(System.getProperty("janus.conformance.suite")).getParent()
      .resolve("corpora/verifier/examples/pact-js/http/pact.json");

  private final Map<Integer, String> users = new TreeMap<>();
  private final HttpServer server;

  private UserService(HttpServer server) {
    this.server = server;
  }

  public static UserService start() {
    try {
      HttpServer server = HttpServer.create(new InetSocketAddress(InetAddress.getByName("127.0.0.1"), 0), 0);
      UserService service = new UserService(server);
      server.createContext("/", http -> {
        String[] path = http.getRequestURI().getPath().split("/");
        String body;
        synchronized (service.users) {
          body = path.length == 2
              ? service.users.values().stream().collect(Collectors.joining(",", "[", "]"))
              : service.users.get(Integer.valueOf(path[2]));
        }
        byte[] bytes = (body == null ? "{}" : body).getBytes(StandardCharsets.UTF_8);
        http.getResponseHeaders().add("Content-Type", "application/json");
        http.sendResponseHeaders(body == null ? 404 : 200, bytes.length);
        http.getResponseBody().write(bytes);
        http.close();
      });
      server.start();
      return service;
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }

  public String url() {
    return "http://127.0.0.1:" + server.getAddress().getPort();
  }

  /** The state both of the pact's interactions are given: Alice exists. */
  public Map<String, Object> aliceExists() {
    synchronized (users) {
      users.put(1, "{\"id\":1,\"name\":\"Alice\",\"email\":\"alice@example.com\"}");
    }
    return null;
  }

  @Override
  public void close() {
    server.stop(0);
  }
}
