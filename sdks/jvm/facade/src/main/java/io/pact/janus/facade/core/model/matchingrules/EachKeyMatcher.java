// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** A map whose every key matches the rules. */
public final class EachKeyMatcher implements MatchingRule {
  private final List<MatchingRule> rules;
  private final Object value;

  public EachKeyMatcher(List<MatchingRule> rules, Object value) {
    this.rules = List.copyOf(rules);
    this.value = value;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "eachKey");
    map.put("rules", rules.stream().map(rule -> rule.toMap(spec)).toList());
    if (value != null) {
      map.put("value", value);
    }
    return map;
  }
}
