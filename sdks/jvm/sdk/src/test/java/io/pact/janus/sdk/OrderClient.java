package io.pact.janus.sdk;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.time.Instant;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

/**
 * The consumer under test in the end-to-end tests: a small client for the orders API, written the
 * way an application would write it — java.net.http and Jackson, nothing Janus-specific.
 *
 * <p>{@link #careless} builds the version with the classic bug: it assumes every order has
 * shipped, so it dereferences {@code shippedAt} without looking.
 */
final class OrderClient {
  record Payment(String type, String last4, LocalDate dueDate) {}

  record Item(String sku, long qty) {}

  record Order(long id, String status, Optional<Instant> shippedAt, Payment payment, List<Item> items) {
    int lineCount() {
      return items.size();
    }
  }

  private static final ObjectMapper MAPPER = new ObjectMapper();
  private static final HttpClient HTTP = HttpClient.newHttpClient();
  private final URI baseUrl;
  private final boolean careless;

  private OrderClient(String baseUrl, boolean careless) {
    this.baseUrl = URI.create(baseUrl.endsWith("/") ? baseUrl : baseUrl + "/");
    this.careless = careless;
  }

  static OrderClient at(String baseUrl) {
    return new OrderClient(baseUrl, false);
  }

  static OrderClient careless(String baseUrl) {
    return new OrderClient(baseUrl, true);
  }

  Order getOrder(String id) throws IOException, InterruptedException {
    HttpRequest request = HttpRequest.newBuilder(baseUrl.resolve("orders/" + id))
        .header("Accept", "application/json")
        .timeout(Duration.ofSeconds(10))
        .GET()
        .build();
    HttpResponse<String> response = HTTP.send(request, HttpResponse.BodyHandlers.ofString());
    if (response.statusCode() != 200) {
      throw new IOException("GET /orders/" + id + " answered " + response.statusCode() + ": " + response.body());
    }
    JsonNode json = MAPPER.readTree(response.body());
    Optional<Instant> shippedAt = careless
        ? Optional.of(Instant.parse(json.get("shippedAt").asText()))
        : Optional.ofNullable(json.get("shippedAt")).map(n -> Instant.parse(n.asText()));
    JsonNode p = json.get("payment");
    Payment payment = new Payment(p.get("type").asText(),
        p.has("last4") ? p.get("last4").asText() : null,
        p.has("dueDate") ? LocalDate.parse(p.get("dueDate").asText()) : null);
    List<Item> items = new ArrayList<>();
    for (JsonNode item : json.get("items")) {
      items.add(new Item(item.get("sku").asText(), item.get("qty").asLong()));
    }
    return new Order(json.get("id").asLong(), json.get("status").asText(), shippedAt, payment, items);
  }
}
