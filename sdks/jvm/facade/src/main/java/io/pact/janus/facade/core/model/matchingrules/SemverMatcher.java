// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A semantic version. */
public final class SemverMatcher implements MatchingRule {


  /** The one instance, as pact-jvm's matcher object is. */
  public static final SemverMatcher INSTANCE = new SemverMatcher();

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "semver");
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof SemverMatcher that && true;
  }

  @Override
  public int hashCode() {
    return "SemverMatcher".hashCode();
  }

  @Override
  public String toString() {
    return "SemverMatcher";
  }
}
