// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** Equal to the example. */
public final class EqualsMatcher implements MatchingRule {


  /** The one instance, as pact-jvm's matcher object is. */
  public static final EqualsMatcher INSTANCE = new EqualsMatcher();

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "equality");
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof EqualsMatcher that && true;
  }

  @Override
  public int hashCode() {
    return "EqualsMatcher".hashCode();
  }

  @Override
  public String toString() {
    return "EqualsMatcher";
  }
}
