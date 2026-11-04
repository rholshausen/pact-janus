package io.pact.janus.sdk;

import io.pact.janus.bindings.shape.v1.Shape;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Objects;

/**
 * The options bag of {@code message-contents} and {@code message-request}: any of metadata and body.
 * A member not given produces no slot. The metadata is a map of name to template, compiled as an
 * {@code object} with one member per name by the literal rules; the body is compiled by the literal
 * rules and always declares its type: {@link Shapes#content}'s, else {@code application/json}
 * (ADR 0068 decision 2).
 */
public final class MessageSlots {
  private static final Object UNSET = new Object();
  private final String part;
  private final Map<String, Object> metadata = new LinkedHashMap<>();
  private Object body = UNSET;

  MessageSlots(String part) {
    this.part = part;
  }

  /** One metadata member, its name exactly as written. */
  public MessageSlots metadata(String name, Object value) {
    Objects.requireNonNull(name, "metadata name");
    if (metadata.containsKey(name)) {
      throw new IllegalArgumentException("metadata '" + name + "' is already declared");
    }
    metadata.put(name, value);
    return this;
  }

  /** Several metadata members, in the map's order. */
  public MessageSlots metadata(Map<String, ?> members) {
    for (Map.Entry<String, Object> e : Json.entries(members, part + " metadata")) {
      metadata(e.getKey(), e.getValue());
    }
    return this;
  }

  /** The body: any value or shape, compiled by the literal rules, or {@link Shapes#content}'s result. */
  public MessageSlots body(Object body) {
    this.body = body;
    return this;
  }

  Map<String, Shape> compile() {
    Map<String, Shape> slots = new LinkedHashMap<>();
    if (!metadata.isEmpty()) {
      // A content() inside it is refused by the literal rules: a metadata member is not a slot.
      slots.put("metadata", Literals.compile(metadata, part + ".metadata"));
    }
    if (body != UNSET) {
      HttpPart.body(slots, body, part + ".body");
    }
    return slots;
  }

  /** The body's declared type, or {@code null} when there is no body. */
  String bodyType() {
    if (body == UNSET) {
      return null;
    }
    String declared = HttpPart.declaredType(body);
    return declared == null ? "application/json" : declared;
  }
}
