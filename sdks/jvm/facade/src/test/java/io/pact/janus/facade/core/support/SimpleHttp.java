package io.pact.janus.facade.core.support;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;

/**
 * A stand-in for pact-jvm's internal {@code SimpleHttp}, which pact-jvm's own tests use, so those tests
 * run here with only their imports changed. Test-only: the facade has no HTTP client.
 */
public class SimpleHttp {
  private final String baseUrl;
  private final HttpClient client = HttpClient.newHttpClient();

  public SimpleHttp(String baseUrl) {
    this.baseUrl = baseUrl;
  }

  public Response get(String path) {
    return send(HttpRequest.newBuilder(URI.create(baseUrl + path)).GET().build());
  }

  public Response post(String path, String body, String contentType) {
    HttpRequest request = HttpRequest.newBuilder(URI.create(baseUrl + path))
        .header("Content-Type", contentType)
        .POST(HttpRequest.BodyPublishers.ofString(body))
        .build();
    return send(request);
  }

  private Response send(HttpRequest request) {
    try {
      HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());
      return new Response(response.statusCode(), response.headers().map(), response.body());
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new IllegalStateException(e);
    }
  }
}
