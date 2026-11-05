// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** A status in a class ({@code success}, {@code clientError}, ...), or one of the statuses given. */
public final class StatusCodeMatcher implements MatchingRule {
  private final String statusType;
  private final List<Integer> values;

  public StatusCodeMatcher(String statusType, List<Integer> values) {
    this.statusType = statusType;
    this.values = List.copyOf(values);
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "statusCode");
    map.put("status", statusType.equals("statusCodes") ? values : statusType);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof StatusCodeMatcher that && statusType.equals(that.statusType) && values.equals(that.values);
  }

  @Override
  public int hashCode() {
    return statusType.hashCode() * 31 + values.hashCode();
  }
}
