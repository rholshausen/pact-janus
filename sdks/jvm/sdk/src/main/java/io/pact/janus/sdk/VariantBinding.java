package io.pact.janus.sdk;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * A provider-state parameter bound to the running variant ({@code when-variant},
 * {@code variant-cases}; variant semantics spec §6). It has no name of its own: the
 * {@link Interaction#given(String, Map)} parameter it is written as names it, and {@code given}
 * moves it out of {@code params} into {@code variant-params}, because a binding written in-band as
 * a parameter value would be indistinguishable from user data.
 */
public final class VariantBinding {
  private final String dimension;
  private final List<Map<String, Object>> cases;
  private final boolean hasDefault;
  private final Object fallback;

  private VariantBinding(String dimension, List<Map<String, Object>> cases, boolean hasDefault, Object fallback) {
    this.dimension = Objects.requireNonNull(dimension, "dimension");
    this.cases = cases;
    this.hasDefault = hasDefault;
    this.fallback = fallback;
  }

  /** True exactly when the running variant is at {@code point} of {@code dimension} — the RFC's {@code whenVariant}. */
  public static VariantBinding whenVariant(String dimension, String point) {
    return new VariantBinding(dimension, List.of(point(Objects.requireNonNull(point, "point"), true)), true, false);
  }

  /**
   * The value the running variant's point for {@code dimension} selects, with no default: a variant
   * at a point no case names leaves the parameter absent (variant semantics spec §6.4). Write
   * {@code cases} with {@link Shapes#map} to keep the order the points are written in.
   */
  public static VariantBinding variantCases(String dimension, Map<String, ?> cases) {
    return new VariantBinding(dimension, cases(dimension, cases), false, null);
  }

  /** As {@link #variantCases(String, Map)}, with the value for an inactive dimension or an unnamed point. */
  public static VariantBinding variantCases(String dimension, Map<String, ?> cases, Object fallback) {
    return new VariantBinding(dimension, cases(dimension, cases), true, Json.value(fallback, "variantCases default"));
  }

  /** The binding document (variant-params.schema.json's {@code Binding}) for the parameter {@code name}. */
  Map<String, Object> binding(String name) {
    Map<String, Object> out = new LinkedHashMap<>();
    out.put("name", name);
    out.put("dimension", dimension);
    out.put("cases", cases);
    if (hasDefault) {
      out.put("default", fallback);
    }
    return out;
  }

  private static List<Map<String, Object>> cases(String dimension, Map<String, ?> cases) {
    List<Map<String, Object>> out = new ArrayList<>();
    for (Map.Entry<String, Object> e : Json.entries(Objects.requireNonNull(cases, "cases"), "variantCases cases")) {
      out.add(point(e.getKey(), Json.value(e.getValue(), "variantCases case '" + e.getKey() + "'")));
    }
    if (out.isEmpty()) {
      throw new IllegalArgumentException(
          "variantCases('" + dimension + "'): at least one case is needed — with none, it is only ever its default");
    }
    return out;
  }

  private static Map<String, Object> point(String point, Object value) {
    Map<String, Object> out = new LinkedHashMap<>();
    out.put("point", point);
    out.put("value", value);
    return out;
  }
}
