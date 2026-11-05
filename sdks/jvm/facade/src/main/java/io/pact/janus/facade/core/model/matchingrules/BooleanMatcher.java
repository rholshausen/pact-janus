// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A boolean. */
public final class BooleanMatcher implements MatchingRule {


  /** The one instance, as pact-jvm's matcher object is. */
  public static final BooleanMatcher INSTANCE = new BooleanMatcher();

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", spec == PactSpecVersion.V4 ? "boolean" : "type");
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof BooleanMatcher that && true;
  }

  @Override
  public int hashCode() {
    return "BooleanMatcher".hashCode();
  }

  @Override
  public String toString() {
    return "BooleanMatcher";
  }
}
