// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** The same type as the example, with at least {@code min} items. */
public final class MinTypeMatcher implements MatchingRule {
  private final int min;

  public MinTypeMatcher(int min) {
    this.min = min;
  }

  public int getMin() {
    return min;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "type");
    map.put("min", min);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof MinTypeMatcher that && java.util.Objects.equals(min, that.min);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(min);
  }

  @Override
  public String toString() {
    return "MinTypeMatcher(" + "min=" + min + ")";
  }
}
