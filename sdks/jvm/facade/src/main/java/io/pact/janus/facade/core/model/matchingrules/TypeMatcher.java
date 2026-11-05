// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** The same type as the example. */
public final class TypeMatcher implements MatchingRule {


  /** The one instance, as pact-jvm's matcher object is. */
  public static final TypeMatcher INSTANCE = new TypeMatcher();

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "type");
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof TypeMatcher that && true;
  }

  @Override
  public int hashCode() {
    return "TypeMatcher".hashCode();
  }

  @Override
  public String toString() {
    return "TypeMatcher";
  }
}
