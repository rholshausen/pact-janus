package io.pact.janus.sdk;

import io.pact.janus.bindings.shape.v1.Shape;
import java.util.Map;

/**
 * What a shape helper returns: one compiled shape-language node (shape spec §3.1). A value of this
 * type is used as authored wherever a shape is expected; any other value is compiled by the
 * {@code literal} rules. That is how the SDK tells a helper's result from a plain map that happens
 * to have a {@code shape} member — by its Java type, never by looking inside it.
 *
 * <p>Immutable once built. Opaque to the protocol beyond its document, which {@link #toDocument()}
 * shows for inspection.
 */
public final class ShapeNode {
  private final Shape node;

  ShapeNode(Shape node) {
    this.node = node;
  }

  /** The generated binding this node is. The idiomatic layer never mutates it after construction. */
  Shape binding() {
    return node;
  }

  /** The operator name, e.g. {@code each-like}. */
  public String operator() {
    return node.getShape();
  }

  /** The node's shape document, as JSON-shaped maps and lists. */
  @SuppressWarnings("unchecked")
  public Map<String, Object> toDocument() {
    return Json.MAPPER.convertValue(node, Map.class);
  }

  @Override
  public String toString() {
    try {
      return Json.MAPPER.writeValueAsString(node);
    } catch (Exception e) {
      return "ShapeNode[" + node.getShape() + "]";
    }
  }

  @Override
  public boolean equals(Object o) {
    return o instanceof ShapeNode other && other.toDocument().equals(toDocument());
  }

  @Override
  public int hashCode() {
    return toDocument().hashCode();
  }
}
