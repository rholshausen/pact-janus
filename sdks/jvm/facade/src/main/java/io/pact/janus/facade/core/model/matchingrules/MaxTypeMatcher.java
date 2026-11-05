// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** The same type as the example, with at most {@code max} items. */
public final class MaxTypeMatcher implements MatchingRule {
  private final int max;

  public MaxTypeMatcher(int max) {
    this.max = max;
  }

  public int getMax() {
    return max;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "type");
    map.put("max", max);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof MaxTypeMatcher that && java.util.Objects.equals(max, that.max);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(max);
  }

  @Override
  public String toString() {
    return "MaxTypeMatcher(" + "max=" + max + ")";
  }
}
