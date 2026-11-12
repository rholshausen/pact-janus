package io.pact.janus.sdk;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.fasterxml.jackson.databind.JsonNode;
import com.sun.net.httpserver.HttpServer;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import org.junit.jupiter.api.Test;

/**
 * The shape recorder's own defaults (behavioural spec 'shape-recorder', STYLE.md "Recording a
 * provider shape"): what a conformance case cannot say, because every case names its own source and
 * builds its answers from data.
 */
class ShapeRecorderTest {
  private static ScriptedEngine engine() {
    return new ScriptedEngine()
        .ok("provider-shape-session/create", "{\"session\":\"r-1\"}")
        .ok("provider-shape-session/observe", "{\"observations\":1,\"interactions\":1}")
        .on("provider-shape-session/finalise", body -> "{\"ok\":{\"provider-shape\":{},\"written\":"
            + body.path("write").path("path") + "}}");
  }

  @Test
  void itsSourceIsTheClassThatCreatedIt() throws Exception {
    ScriptedEngine engine = engine();
    Path dir = Files.createTempDirectory("janus-recorder-");
    ShapeRecorder recorder = ShapeRecorder.forProvider("orders-api").withShapeDir(dir).withEvidenceDir(dir).withEmbedding(engine);
    recorder.observe(Map.of("method", "GET", "path", Shapes.pathTemplate("/orders/{id}")), new ShapeRecorder.Response(200, null, null));
    assertEquals(dir.resolve("orders-api.provider-shape.json"), recorder.finish());
    assertEquals(ShapeRecorderTest.class.getName(), engine.body("provider-shape-session/finalise").path("write").path("source").asText());
  }

  @Test
  void anHttpResponseIsReadWithTheRequestItAnswered() throws Exception {
    HttpServer server = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 0);
    server.createContext("/", exchange -> {
      byte[] body = "{\"id\":\"66\"}".getBytes(StandardCharsets.UTF_8);
      exchange.getResponseHeaders().add("Content-Type", "application/json");
      exchange.sendResponseHeaders(200, body.length);
      exchange.getResponseBody().write(body);
      exchange.close();
    });
    server.start();
    try {
      HttpResponse<byte[]> response = HttpClient.newHttpClient().send(
          HttpRequest.newBuilder(URI.create("http://127.0.0.1:" + server.getAddress().getPort() + "/orders/66")).build(),
          HttpResponse.BodyHandlers.ofByteArray());
      ScriptedEngine engine = engine();
      ShapeRecorder recorder = ShapeRecorder.forProvider("orders-api").withSource("s").withEmbedding(engine);
      recorder.observe(Map.of("method", "GET", "path", Shapes.pathTemplate("/orders/{id}")), ShapeRecorder.Response.of(response));
      JsonNode parts = engine.body("provider-shape-session/observe").path("parts");
      assertEquals(ScriptedEngine.json("{\"method\":{\"content\":\"GET\"},\"path\":{\"content\":\"/orders/66\"}}"), parts.path("request"));
      assertEquals("application/json", parts.path("response").path("body").path("content-type").asText());
      assertEquals("GET /orders/{id}", engine.body("provider-shape-session/observe").path("description").asText());
    } finally {
      server.stop(0);
    }
  }
}
