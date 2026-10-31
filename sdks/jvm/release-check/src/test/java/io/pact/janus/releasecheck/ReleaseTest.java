package io.pact.janus.releasecheck;

import static io.pact.janus.sdk.Shapes.integer;
import static io.pact.janus.sdk.Shapes.json;
import static io.pact.janus.sdk.Shapes.map;
import static io.pact.janus.sdk.Shapes.string;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import com.sun.net.httpserver.HttpServer;
import io.pact.janus.sdk.Janus;
import io.pact.janus.sdk.JanusOptions;
import io.pact.janus.sdk.VerificationResult;
import io.pact.janus.sdk.Verifier;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.MethodOrderer;
import org.junit.jupiter.api.Order;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestMethodOrder;

/**
 * A JVM SDK release, resolved and used as a user would (staged plan S1.36, ADR 0060 decision 7): a
 * consumer test records a contract against the engine's mock, and a provider test verifies that
 * contract against a provider it serves. Nothing names an engine, so the SDK fetches the one it was
 * released with. release-sdks.yml runs this outside the repository, against the staged release files
 * and an empty {@code JANUS_ENGINE_CACHE}, before anything is published.
 */
@TestMethodOrder(MethodOrderer.OrderAnnotation.class)
class ReleaseTest {
  private static Path contract;

  @Test
  @Order(1)
  void nothingNamesAnEngine() {
    String named = System.getenv("JANUS_ENGINE");
    assertTrue(named == null || named.isEmpty(), "JANUS_ENGINE is " + named);
  }

  @Test
  @Order(2)
  void aConsumerTestRecordsAContract() throws Exception {
    Path contracts = Files.createTempDirectory("pact-janus-release-check-");
    try (Janus janus = Janus.of("release-check-web", "release-check-orders",
        JanusOptions.defaults().withContractDirectory(contracts))) {
      janus.execute(janus.interaction("get an order")
          .given("an order exists", map("id", 42))
          .request(r -> r.method("GET").path("/orders/42"))
          .response(r -> r.status(200).body(json(map("id", integer(42), "status", string("SHIPPED"))))),
          (mock, variant) -> {
            HttpResponse<String> response = HttpClient.newHttpClient().send(
                HttpRequest.newBuilder(URI.create(mock.url() + "/orders/42")).GET().build(),
                HttpResponse.BodyHandlers.ofString());
            assertEquals(200, response.statusCode());
            assertTrue(response.body().contains("\"id\""), response.body());
          });
      contract = janus.finalise().orElseThrow();
    }
  }

  @Test
  @Order(3)
  void aProviderTestVerifiesIt() throws Exception {
    assertTrue(contract != null, "the consumer test wrote a contract");
    List<Map<String, Object>> states = new ArrayList<>();
    HttpServer provider = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 0);
    provider.createContext("/orders/42", exchange -> {
      byte[] body = "{\"id\": 42, \"status\": \"PENDING\"}".getBytes(StandardCharsets.UTF_8);
      exchange.getResponseHeaders().add("Content-Type", "application/json");
      exchange.sendResponseHeaders(200, body.length);
      try (OutputStream out = exchange.getResponseBody()) {
        out.write(body);
      }
    });
    provider.start();
    try {
      VerificationResult result = Verifier.forProvider("http://127.0.0.1:" + provider.getAddress().getPort())
          .withPacts(List.of(contract))
          .state("an order exists", params -> {
            states.add(params);
            return Map.of();
          })
          .verify();
      assertEquals("verified", result.status());
      assertEquals(1, states.size());
      assertEquals(42, ((Number) states.get(0).get("id")).intValue());
    } finally {
      provider.stop(0);
    }
  }

  @Test
  @Order(4)
  void theEngineItRanIsTheOneTheReleasePinsFetchedIntoTheCache() throws Exception {
    String cache = System.getenv("JANUS_ENGINE_CACHE");
    assumeTrue(cache != null && !cache.isEmpty(), "JANUS_ENGINE_CACHE is not set, so the cache is the user's");
    try (Stream<Path> versions = Files.list(Path.of(cache))) {
      List<Path> fetched = versions.toList();
      assertEquals(1, fetched.size(), fetched.toString());
      try (Stream<Path> files = Files.list(fetched.get(0))) {
        assertEquals(1, files.filter(file -> file.getFileName().toString().startsWith("janus-engine-")).count());
      }
    }
  }
}
