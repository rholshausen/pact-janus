package io.pact.janus.sdk;

import io.pact.janus.bindings.shape.v1.Shape;
import io.pact.janus.bindings.shape.v1.Vocabulary.ShapeShape;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.ArrayList;
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
   * names under — and a name already declared under another spelling is refused, rather than one of
   * the two being dropped where nobody can see it (ADR 0019). A value is an {@code equality} over the
   * one-element list {@code [value]}, a list an {@code equality} over that list, and a shape helper's
   * result describes the one value and becomes an {@code each-like} whose {@code items} is that shape,
   * bounded at exactly one (an unbounded one would add a request variant sending the header twice).
   * A value that is not a string is written as the one string that spells it, where every SDK spells
   * it the same way: see {@code text} below for what is written and what is refused.
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
    if (value instanceof Content) {
      throw Content.misplaced(where);
    }
    if (value instanceof ShapeNode node) {
      return listed(node.binding());
    }
    if (value instanceof List<?> list) {
      List<String> values = new ArrayList<>(list.size());
      int i = 0;
      for (Object item : list) {
        values.add(text(item, where + "[" + i++ + "]"));
      }
      return Literals.equality(values, where);
    }
    return Literals.equality(List.of(text(value, where)), where);
  }

  /**
   * The one-element list treatment, applied where the value actually is (behavioural spec
   * {@code request}). A shape that admits absence is a fact about the member, not about the value in
   * it, so the list goes inside the modifier: {@code forbidden} stands as the member's shape — there
   * is no value to carry a list — and {@code optional} keeps the {@code each-like} as its {@code of}.
   *
   * <p>The two are named rather than detected by asking whether a node admits absence, because that
   * question belongs to the shape language and its answer for a component operator is opaque to an
   * SDK (shape spec §3.5); an SDK that guessed would hold a second copy of §5.1.
   */
  private static Shape listed(Shape node) {
    if (ShapeShape.FORBIDDEN.equals(node.getShape())) {
      return node;
    }
    if (ShapeShape.OPTIONAL.equals(node.getShape())) {
      node.setOf(listed(node.getOf()));
      return node;
    }
    Shape each = Literals.node(ShapeShape.EACH_LIKE);
    each.setItems(node);
    each.setMin(1L);
    each.setMax(1L);
    return each;
  }

  /**
   * The one string that spells a header or query value (behavioural spec {@code request}, ADR 0019).
   * A string is itself; a whole number whose magnitude is at most 2^53 - 1 is its shortest decimal
   * form, so {@code 3} and {@code 3.0} are both {@code "3"}; a boolean is {@code true}/{@code false}.
   * Everything else is refused here, at the call: a value whose spelling differs between languages
   * would have two SDKs send different bytes for the same test, and a value that is not a string at
   * all compiles to a shape the transport's values can never match.
   */
  private static String text(Object value, String where) {
    if (value instanceof String s) {
      return s;
    }
    if (value instanceof Boolean b) {
      return b ? "true" : "false";
    }
    if (value instanceof Number n) {
      BigDecimal exact = decimalOf(n);
      if (exact != null) {
        BigDecimal whole = exact.stripTrailingZeros();
        if (whole.scale() <= 0 && whole.abs().compareTo(SAFE_INTEGER) <= 0) {
          return whole.toBigIntegerExact().toString();
        }
      }
    }
    throw new IllegalArgumentException(where + ": " + describe(value)
        + " has no spelling every Janus SDK agrees on; a header or query value is a string, a whole"
        + " number up to 2^53 - 1, a boolean, a list of those, or a shape helper's result — write the"
        + " string you mean");
  }

  /** The largest integer every Janus SDK spells the same way: JavaScript's exact-integer bound. */
  private static final BigDecimal SAFE_INTEGER = BigDecimal.valueOf(9007199254740991L);

  private static BigDecimal decimalOf(Number n) {
    try {
      if (n instanceof BigDecimal d) {
        return d;
      }
      if (n instanceof BigInteger i) {
        return new BigDecimal(i);
      }
      if (n instanceof Double || n instanceof Float) {
        return BigDecimal.valueOf(n.doubleValue());
      }
      return BigDecimal.valueOf(n.longValue());
    } catch (NumberFormatException notANumber) {
      // NaN and the infinities: no decimal form at all, so no spelling either.
      return null;
    }
  }

  private static String describe(Object value) {
    if (value == null) {
      return "null";
    }
    return value instanceof Number || value instanceof CharSequence
        ? value + " (a " + value.getClass().getSimpleName() + ")"
        : "a " + value.getClass().getName();
  }

  static void slot(Map<String, Shape> part, String name, Object value, String where) {
    part.put(name, Literals.compile(value, where));
  }

  /** A body slot: {@link Content}'s document by the literal rules, or the value itself. */
  static void body(Map<String, Shape> part, Object value, String where) {
    slot(part, "body", value instanceof Content content ? content.document() : value, where);
  }

  /** The media type a body written with {@link Shapes#content} declares, or {@code null}. */
  static String declaredType(Object body) {
    return body instanceof Content content ? content.mediaType() : null;
  }
}
