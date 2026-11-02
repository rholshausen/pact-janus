package io.pact.janus.sdk.frameworks;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;

/** The consumer the framework suites test: a client of the orders API that returns its bodies. */
public final class Orders {
  private static final HttpClient CLIENT = HttpClient.newHttpClient();

  private Orders() {}

  public static String get(String url) throws IOException, InterruptedException {
    return send(HttpRequest.newBuilder(URI.create(url)).GET());
  }

  public static String delete(String url) throws IOException, InterruptedException {
    return send(HttpRequest.newBuilder(URI.create(url)).DELETE());
  }

  private static String send(HttpRequest.Builder request) throws IOException, InterruptedException {
    return CLIENT.send(request.build(), HttpResponse.BodyHandlers.ofString()).body();
  }
}
