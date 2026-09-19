package io.pact.janus.sdk;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.Collection;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.SortedMap;

/** JSON plumbing shared by the idiomatic layer. Not API. */
final class Json {
  static final ObjectMapper MAPPER =
      new ObjectMapper().disable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES);

  private Json() {}

  /**
   * A map's entries in the order they are written into a document. A map with a defined encounter
   * order — a {@link LinkedHashMap} (which {@link Shapes#map} returns), a {@link SortedMap} — keeps
   * it. A map without one ({@code Map.of}, {@code HashMap}) has no "order written" left to honour,
   * and its iteration order can differ from one JVM run to the next ({@code Map.of} salts it), so its
   * entries are written in key order: the same DSL must always produce the same document (contract
   * spec §2.4). STYLE.md records this as a deviation.
   */
  static List<Map.Entry<String, Object>> entries(Map<?, ?> map, String what) {
    List<Map.Entry<String, Object>> out = new ArrayList<>(map.size());
    for (Map.Entry<?, ?> e : map.entrySet()) {
      if (!(e.getKey() instanceof String key)) {
        throw new IllegalArgumentException(
            what + " has a key that is not a string (" + e.getKey() + "); JSON object member names are strings");
      }
      out.add(new java.util.AbstractMap.SimpleImmutableEntry<>(key, e.getValue()));
    }
    if (!(map instanceof LinkedHashMap) && !(map instanceof SortedMap)) {
      out.sort(Map.Entry.comparingByKey());
    }
    return out;
  }

  /**
   * A plain Java value as the JSON value it denotes — for example values, {@code any-of} options and
   * state parameters, which are values and never shapes. Rejects anything with no JSON reading
   * rather than guessing one (SDK spec §2.1: no coercion).
   */
  static Object value(Object v, String what) {
    if (v == null || v instanceof String || v instanceof Boolean) {
      return v;
    }
    if (v instanceof Number n) {
      return number(n, what);
    }
    if (v instanceof Map<?, ?> m) {
      Map<String, Object> out = new LinkedHashMap<>();
      for (Map.Entry<String, Object> e : entries(m, what)) {
        out.put(e.getKey(), value(e.getValue(), what + "." + e.getKey()));
      }
      return out;
    }
    if (v instanceof Collection<?> c) {
      List<Object> out = new ArrayList<>(c.size());
      int i = 0;
      for (Object item : c) {
        out.add(value(item, what + "[" + i++ + "]"));
      }
      return out;
    }
    if (v instanceof ShapeNode) {
      throw new IllegalArgumentException(
          what + " is a shape helper's result, but a value is expected here, not a shape");
    }
    throw new IllegalArgumentException(
        what + " is a " + v.getClass().getName() + ", which has no JSON reading; write it as a string, number, boolean, null, Map or List");
  }

  static Number number(Number n, String what) {
    if (n instanceof Double d && (d.isNaN() || d.isInfinite())
        || n instanceof Float f && (f.isNaN() || f.isInfinite())) {
      throw new IllegalArgumentException(what + " is " + n + ", which is not a JSON number");
    }
    if (n instanceof Integer || n instanceof Long || n instanceof Short || n instanceof Byte
        || n instanceof Double || n instanceof Float || n instanceof BigDecimal || n instanceof BigInteger) {
      return n;
    }
    // AtomicInteger, LongAdder and friends: their value, not their bean properties.
    return new BigDecimal(n.toString());
  }
}
