// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import java.util.LinkedHashMap;
import java.util.Map;

/** A part's matching rules, by category, in the order they were first added. */
public class MatchingRulesImpl implements MatchingRules {
  private final Map<String, MatchingRuleCategory> categories = new LinkedHashMap<>();

  @Override
  public MatchingRuleCategory rulesForCategory(String category) {
    MatchingRuleCategory found = categories.get(category);
    return found == null ? new MatchingRuleCategory(category) : found;
  }

  @Override
  public MatchingRuleCategory addCategory(String category) {
    return categories.computeIfAbsent(category, MatchingRuleCategory::new);
  }

  @Override
  public MatchingRuleCategory addCategory(MatchingRuleCategory category) {
    return addCategory(category.getName()).addRules(category);
  }

  @Override
  public Map<String, MatchingRuleCategory> getCategories() {
    return categories;
  }

  /** A copy whose categories are copies. */
  public MatchingRulesImpl copy() {
    MatchingRulesImpl copy = new MatchingRulesImpl();
    categories.forEach((name, category) -> copy.categories.put(name, category.copy()));
    return copy;
  }

  @Override
  public String toString() {
    return "MatchingRules(" + categories + ")";
  }
}
