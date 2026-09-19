package io.pact.janus.sdk;

import io.pact.janus.bindings.shape.v1.Shape;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Objects;

/**
 * The options bag of {@code request}: any of method, path, query, headers and body. A member not
 * given produces no slot. Method, path and body are compiled by the literal rules and otherwise
 * untouched — the method is not upper-cased and the path is not normalised.
 */
public final class RequestParts extends HttpPart<RequestParts> {
  private static final Object UNSET = new Object();
  private Object method = UNSET;
  private Object path = UNSET;
  private Object body = UNSET;
  private final Map<String, Object> query = new LinkedHashMap<>();

  RequestParts() {
    super("request");
  }

  @Override
  RequestParts self() {
    return this;
  }

  /** The method: a bare value (an {@code equality}) or a shape helper's result. */
  public RequestParts method(Object method) {
    this.method = method;
    return this;
  }

  /** The path: a bare value (an {@code equality}) or a shape helper's result. */
  public RequestParts path(Object path) {
    this.path = path;
    return this;
  }

  /**
   * One query parameter. Its name is kept exactly as written — query names are case-sensitive.
   * Values follow the same rule as headers.
   */
  public RequestParts query(String name, Object value) {
    Objects.requireNonNull(name, "query parameter name");
    if (query.containsKey(name)) {
      throw new IllegalArgumentException(
          "query parameter '" + name + "' is already declared; give a list of values instead of declaring it twice");
    }
    query.put(name, value);
    return this;
  }

  /** Several query parameters, in the map's order. */
  public RequestParts query(Map<String, ?> parameters) {
    for (Map.Entry<String, Object> e : Json.entries(parameters, "request query")) {
      query(e.getKey(), e.getValue());
    }
    return this;
  }

  /** The body: any value or shape, compiled by the literal rules ({@link Shapes#json} is sugar for it). */
  public RequestParts body(Object body) {
    this.body = body;
    return this;
  }

  Map<String, Shape> compile() {
    Map<String, Shape> slots = new LinkedHashMap<>();
    if (method != UNSET) {
      slot(slots, "method", method, "request.method");
    }
    if (path != UNSET) {
      slot(slots, "path", path, "request.path");
    }
    if (!query.isEmpty()) {
      slots.put("query", valueLists(query, "request.query"));
    }
    if (hasHeaders()) {
      slots.put("headers", headersShape());
    }
    if (body != UNSET) {
      slot(slots, "body", body, "request.body");
    }
    return slots;
  }
}
