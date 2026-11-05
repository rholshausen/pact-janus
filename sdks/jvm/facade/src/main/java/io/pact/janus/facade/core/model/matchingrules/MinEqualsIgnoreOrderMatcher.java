// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** Equal to the example in any order, with at least {@code min} items. */
public final class MinEqualsIgnoreOrderMatcher implements MatchingRule {
  private final int min;

  public MinEqualsIgnoreOrderMatcher(int min) {
    this.min = min;
  }

  public int getMin() {
    return min;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "ignore-order");
    map.put("min", min);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof MinEqualsIgnoreOrderMatcher that && java.util.Objects.equals(min, that.min);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(min);
  }

  @Override
  public String toString() {
    return "MinEqualsIgnoreOrderMatcher(" + "min=" + min + ")";
  }
}
