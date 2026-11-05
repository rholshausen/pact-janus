// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** Including a string. */
public final class IncludeMatcher implements MatchingRule {
  private final String value;

  public IncludeMatcher(String value) {
    this.value = value;
  }

  public String getValue() {
    return value;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "include");
    map.put("value", value);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof IncludeMatcher that && java.util.Objects.equals(value, that.value);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(value);
  }

  @Override
  public String toString() {
    return "IncludeMatcher(" + "value=" + value + ")";
  }
}
