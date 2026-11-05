// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** Present and not empty. */
public final class NotEmptyMatcher implements MatchingRule {


  /** The one instance, as pact-jvm's matcher object is. */
  public static final NotEmptyMatcher INSTANCE = new NotEmptyMatcher();

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", spec == PactSpecVersion.V4 ? "notEmpty" : "type");
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof NotEmptyMatcher that && true;
  }

  @Override
  public int hashCode() {
    return "NotEmptyMatcher".hashCode();
  }

  @Override
  public String toString() {
    return "NotEmptyMatcher";
  }
}
