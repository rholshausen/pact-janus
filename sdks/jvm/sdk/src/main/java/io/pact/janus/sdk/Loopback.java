package io.pact.janus.sdk;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.util.HexFormat;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.function.Function;

/**
 * The loopback endpoint a provider verification's handlers are served from (ADR 0055 decision 2):
 * one HTTP server on 127.0.0.1, at a port the system chooses, for one run. The engine reaches it as
 * it reaches any {@code http} hook, POSTing a hook context and reading an invoke result. Every
 * request must carry the run's bearer token, so no other process on the machine can drive the
 * handlers. Decides nothing about what a handler means. Not API.
 */
final class Loopback implements AutoCloseable {
  private final String authorization;
  private final Map<String, Function<Map<String, Object>, Map<String, Object>>> routes = new ConcurrentHashMap<>();
  private final HttpServer server;
  private final ExecutorService executor = Executors.newCachedThreadPool(runnable -> {
    Thread thread = new Thread(runnable, "janus-loopback");
    thread.setDaemon(true);
    return thread;
  });

  Loopback() throws IOException {
    byte[] token = new byte[16];
    new SecureRandom().nextBytes(token);
    authorization = "Bearer " + HexFormat.of().formatHex(token);
    server = HttpServer.create(new InetSocketAddress(InetAddress.getByName("127.0.0.1"), 0), 0);
    server.createContext("/", this::serve);
    server.setExecutor(executor);
    server.start();
  }

  /** The {@code authorization} header every entry carries. */
  String authorization() {
    return authorization;
  }

  /** Serves {@code route} at {@code path}: the hook context in, the invoke result out. It does not throw. */
  void route(String path, Function<Map<String, Object>, Map<String, Object>> route) {
    routes.put(path, route);
  }

  String url(String path) {
    return "http://127.0.0.1:" + server.getAddress().getPort() + path;
  }

  @Override
  public void close() {
    server.stop(0);
    executor.shutdownNow();
  }

  @SuppressWarnings("unchecked")
  private void serve(HttpExchange exchange) throws IOException {
    try (exchange) {
      String given = exchange.getRequestHeaders().getFirst("authorization");
      if (given == null || !MessageDigest.isEqual(
          given.getBytes(StandardCharsets.UTF_8), authorization.getBytes(StandardCharsets.UTF_8))) {
        exchange.sendResponseHeaders(401, -1);
        return;
      }
      var route = "POST".equals(exchange.getRequestMethod()) ? routes.get(exchange.getRequestURI().getPath()) : null;
      if (route == null) {
        exchange.sendResponseHeaders(404, -1);
        return;
      }
      Map<String, Object> context;
      try (InputStream in = exchange.getRequestBody()) {
        context = Json.MAPPER.readValue(in, Map.class);
      } catch (IOException e) {
        exchange.sendResponseHeaders(400, -1);
        return;
      }
      byte[] body = Json.MAPPER.writeValueAsBytes(route.apply(context));
      exchange.getResponseHeaders().set("content-type", "application/json");
      exchange.sendResponseHeaders(200, body.length);
      try (OutputStream out = exchange.getResponseBody()) {
        out.write(body);
      }
    }
  }
}
