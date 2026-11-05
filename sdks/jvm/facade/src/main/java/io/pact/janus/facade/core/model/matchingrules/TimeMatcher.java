// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A time in a format. */
public final class TimeMatcher implements MatchingRule {
  private final String format;

  public TimeMatcher(String format) {
    this.format = format;
  }

  public String getFormat() {
    return format;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "time");
    map.put("format", format);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof TimeMatcher that && java.util.Objects.equals(format, that.format);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(format);
  }

  @Override
  public String toString() {
    return "TimeMatcher(" + "format=" + format + ")";
  }
}
