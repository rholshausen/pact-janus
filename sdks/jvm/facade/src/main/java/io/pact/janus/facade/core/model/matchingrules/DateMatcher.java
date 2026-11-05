// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A date in a format. */
public final class DateMatcher implements MatchingRule {
  private final String format;

  public DateMatcher(String format) {
    this.format = format;
  }

  public String getFormat() {
    return format;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "date");
    map.put("format", format);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof DateMatcher that && java.util.Objects.equals(format, that.format);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(format);
  }

  @Override
  public String toString() {
    return "DateMatcher(" + "format=" + format + ")";
  }
}
