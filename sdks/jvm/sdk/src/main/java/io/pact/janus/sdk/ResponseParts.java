package io.pact.janus.sdk;

import io.pact.janus.bindings.shape.v1.Shape;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * The options bag of {@code response}: any of status, headers and body — the same treatment as
 * {@link RequestParts}.
 */
public final class ResponseParts extends HttpPart<ResponseParts> {
  private static final Object UNSET = new Object();
  private Object status = UNSET;
  private Object body = UNSET;

  ResponseParts() {
    super("response");
  }

  @Override
  ResponseParts self() {
    return this;
  }

  /** The status: a bare value (an {@code equality}) or a shape helper's result. */
  public ResponseParts status(Object status) {
    this.status = status;
    return this;
  }

  /** The body: any value or shape, compiled by the literal rules. */
  public ResponseParts body(Object body) {
    this.body = body;
    return this;
  }

  Map<String, Shape> compile() {
    Map<String, Shape> slots = new LinkedHashMap<>();
    if (status != UNSET) {
      slot(slots, "status", status, "response.status");
    }
    if (hasHeaders()) {
      slots.put("headers", headersShape());
    }
    if (body != UNSET) {
      slot(slots, "body", body, "response.body");
    }
    return slots;
  }
}
