package io.pact.janus.sdk;

import io.pact.janus.bindings.shape.v1.Shape;
import io.pact.janus.bindings.shape.v1.Vocabulary.ShapeShape;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * The {@code literal} primitive: the rule every nested-shape parameter, and every member of a
 * request or response part, is compiled by. Not API — it has no call site of its own.
 */
final class Literals {
  private Literals() {}

  /**
   * A helper's result is used as authored; a string, number, boolean or null becomes an
   * {@code equality} node; a map an {@code object} node, member by member; a list an {@code array}
   * node, entry by entry. Nothing else is inferred.
   */
  static Shape compile(Object value, String where) {
    if (value instanceof ShapeNode node) {
      return node.binding();
    }
    if (value == null || value instanceof String || value instanceof Number || value instanceof Boolean) {
      return equality(value, where);
    }
    if (value instanceof Map<?, ?> map) {
      Map<String, Shape> members = new LinkedHashMap<>();
      for (Map.Entry<String, Object> e : Json.entries(map, where)) {
        members.put(e.getKey(), compile(e.getValue(), where + "." + e.getKey()));
      }
      Shape shape = node(ShapeShape.OBJECT);
      shape.setMembers(members);
      return shape;
    }
    if (value instanceof List<?> list) {
      List<Shape> entries = new ArrayList<>(list.size());
      for (int i = 0; i < list.size(); i++) {
        entries.add(compile(list.get(i), where + "[" + i + "]"));
      }
      Shape shape = node(ShapeShape.ARRAY);
      shape.setEntries(entries);
      return shape;
    }
    throw new IllegalArgumentException(
        where + " is a " + value.getClass().getName() + ", which is neither a shape helper's result nor a"
            + " JSON value (string, number, boolean, null, Map or List); nothing else is inferred");
  }

  static Shape equality(Object example, String where) {
    Shape shape = node(ShapeShape.EQUALITY);
    example(shape, example, where);
    return shape;
  }

  static Shape node(String operator) {
    Shape shape = new Shape();
    shape.setShape(operator);
    return shape;
  }

  /**
   * Sets {@code example}. For {@code equality} the example is the operator's parameter and MUST be
   * present (shape spec §3.3) — including when it is {@code null}, which the generated binding's
   * {@code NON_NULL} inclusion would drop, so a null example travels as an additional property.
   */
  static void example(Shape shape, Object example, String where) {
    Object json = Json.value(example, where);
    if (json == null) {
      shape.setAdditionalProperty("example", null);
    } else {
      shape.setExample(json);
    }
  }
}
