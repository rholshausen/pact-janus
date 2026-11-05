// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** The same type as the example, with {@code min} to {@code max} items. */
public final class MinMaxTypeMatcher implements MatchingRule {
  private final int min;
  private final int max;

  public MinMaxTypeMatcher(int min, int max) {
    this.min = min;
    this.max = max;
  }

  public int getMin() {
    return min;
  }

  public int getMax() {
    return max;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "type");
    map.put("min", min);
    map.put("max", max);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof MinMaxTypeMatcher that && java.util.Objects.equals(min, that.min) && java.util.Objects.equals(max, that.max);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(min, max);
  }

  @Override
  public String toString() {
    return "MinMaxTypeMatcher(" + "min=" + min + ", " + "max=" + max + ")";
  }
}
