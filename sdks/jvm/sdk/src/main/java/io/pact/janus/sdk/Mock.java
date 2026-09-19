package io.pact.janus.sdk;

import java.net.URI;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * The first argument of the {@code execute} closure: the started transport's endpoint. The
 * endpoint descriptor is an open document (engine-protocol spec §8.2, design 2.6), never assumed to
 * be a URL; for HTTP it carries {@code base-url}, which {@link #url()} returns.
 */
public final class Mock {
  private final Map<String, Object> endpoint;

  Mock(Map<String, Object> endpoint) {
    this.endpoint = Collections.unmodifiableMap(new LinkedHashMap<>(endpoint));
  }

  /** The endpoint descriptor exactly as {@code consumer-session/start-transport} returned it. */
  public Map<String, Object> endpoint() {
    return endpoint;
  }

  /** The HTTP mock's base URL — the descriptor's {@code base-url} — e.g. {@code http://127.0.0.1:40235}. */
  public String url() {
    Object url = endpoint.get("base-url");
    if (!(url instanceof String s)) {
      throw new IllegalStateException("the endpoint descriptor carries no 'base-url': " + endpoint);
    }
    return s;
  }

  /** {@link #url()} as a {@link URI}. */
  public URI uri() {
    return URI.create(url());
  }

  /** {@link #url()} with {@code path} appended, e.g. {@code mock.uri("/orders/42")}. */
  public URI uri(String path) {
    String base = url();
    return URI.create(base.endsWith("/") && path.startsWith("/") ? base + path.substring(1) : base + path);
  }

  @Override
  public String toString() {
    return "Mock" + endpoint;
  }
}
