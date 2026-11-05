// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** The rules at one path, and how they combine. */
public final class MatchingRuleGroup {
  private final List<MatchingRule> rules;
  private final RuleLogic ruleLogic;

  public MatchingRuleGroup() {
    this(new ArrayList<>(), RuleLogic.AND);
  }

  public MatchingRuleGroup(List<MatchingRule> rules) {
    this(rules, RuleLogic.AND);
  }

  public MatchingRuleGroup(List<MatchingRule> rules, RuleLogic ruleLogic) {
    this.rules = new ArrayList<>(rules);
    this.ruleLogic = ruleLogic;
  }

  public List<MatchingRule> getRules() {
    return rules;
  }

  public RuleLogic getRuleLogic() {
    return ruleLogic;
  }

  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("combine", ruleLogic.name());
    map.put("matchers", rules.stream().map(rule -> rule.toMap(spec)).toList());
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof MatchingRuleGroup group && rules.equals(group.rules) && ruleLogic == group.ruleLogic;
  }

  @Override
  public int hashCode() {
    return rules.hashCode() * 31 + ruleLogic.hashCode();
  }

  @Override
  public String toString() {
    return "MatchingRuleGroup(rules=" + rules + ", ruleLogic=" + ruleLogic + ")";
  }
}
