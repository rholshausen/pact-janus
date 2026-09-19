package io.pact.janus.sdk;

import io.pact.janus.bindings.shape.v1.Shape;
import io.pact.janus.bindings.shape.v1.Vocabulary.ShapeShape;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;

/**
 * What {@link RequestParts} and {@link ResponseParts} share: slots compiled by the literal rules,
 * and the name-to-list-of-values rule for {@code headers} and {@code query}. Not API.
 */
abstract class HttpPart<SELF extends HttpPart<SELF>> {
  private final String part;
  private final Map<String, Object> headers = new LinkedHashMap<>();

  HttpPart(String part) {
    this.part = part;
  }

  abstract SELF self();

  /**
   * One header. Its name is lower-cased — the only spelling the HTTP transport presents header
   * names under. A string value is an {@code equality} over the one-element list {@code [value]};
   * a list of strings an {@code equality} over that list; a shape helper's result describes each
   * value and becomes an {@code each-like} whose {@code items} is that shape.
   */
  public SELF header(String name, Object value) {
    Objects.requireNonNull(name, "header name");
    String lower = name.toLowerCase(Locale.ROOT);
    if (headers.containsKey(lower)) {
      throw new IllegalArgumentException(
          part + " header '" + name + "' is already declared (header names are case-insensitive, and"
              + " are written lower-cased); give a list of values instead of declaring it twice");
    }
    headers.put(lower, value);
    return self();
  }

  /** Several headers, in the map's order (see {@link Shapes#map} for an ordered map literal). */
  public SELF headers(Map<String, ?> headers) {
    for (Map.Entry<String, Object> e : Json.entries(headers, part + " headers")) {
      header(e.getKey(), e.getValue());
    }
    return self();
  }

  boolean hasHeaders() {
    return !headers.isEmpty();
  }

  Shape headersShape() {
    return valueLists(headers, part + ".headers");
  }

  /** The HTTP transport carries headers and query parameters as a map of name to a list of values. */
  static Shape valueLists(Map<String, Object> slots, String where) {
    Map<String, Shape> members = new LinkedHashMap<>();
    for (Map.Entry<String, Object> e : slots.entrySet()) {
      members.put(e.getKey(), valueList(e.getValue(), where + "." + e.getKey()));
    }
    Shape shape = Literals.node(ShapeShape.OBJECT);
    shape.setMembers(members);
    return shape;
  }

  private static Shape valueList(Object value, String where) {
    if (value instanceof ShapeNode node) {
      Shape each = Literals.node(ShapeShape.EACH_LIKE);
      each.setItems(node.binding());
      return each;
    }
    if (value instanceof String s) {
      return Literals.equality(List.of(s), where);
    }
    if (value instanceof List<?> list && list.stream().allMatch(v -> v instanceof String)) {
      return Literals.equality(list, where);
    }
    throw new IllegalArgumentException(
        where + " must be a string, a list of strings, or a shape helper's result describing each value;"
            + " got " + (value == null ? "null" : value.getClass().getName()));
  }

  static void slot(Map<String, Shape> part, String name, Object value, String where) {
    part.put(name, Literals.compile(value, where));
  }
}
