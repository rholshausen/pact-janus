// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;

/** A number of a kind: an integer, a decimal, or either. */
public final class NumberTypeMatcher implements MatchingRule {
  /** The kinds of number. */
  public enum NumberType {
    NUMBER,
    INTEGER,
    DECIMAL
  }

  private final NumberType numberType;

  public NumberTypeMatcher(NumberType numberType) {
    this.numberType = numberType;
  }

  public NumberType getNumberType() {
    return numberType;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    boolean v3 = spec == null || spec.atLeast(PactSpecVersion.V3);
    map.put("match", v3 ? numberType.name().toLowerCase(Locale.ROOT) : "type");
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof NumberTypeMatcher that && numberType == that.numberType;
  }

  @Override
  public int hashCode() {
    return numberType.hashCode();
  }

  @Override
  public String toString() {
    return "NumberTypeMatcher(numberType=" + numberType + ")";
  }
}
