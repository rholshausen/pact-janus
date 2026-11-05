// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * The rules of one category, by path: {@code body} by JSON path, {@code header} and {@code query} by
 * name, and {@code path} and {@code status} at the one key {@code ""}.
 */
public final class MatchingRuleCategory {
  private final String name;
  private final Map<String, MatchingRuleGroup> matchingRules;

  public MatchingRuleCategory(String name) {
    this(name, new LinkedHashMap<>());
  }

  public MatchingRuleCategory(String name, Map<String, MatchingRuleGroup> matchingRules) {
    this.name = name;
    this.matchingRules = new LinkedHashMap<>(matchingRules);
  }

  public String getName() {
    return name;
  }

  public Map<String, MatchingRuleGroup> getMatchingRules() {
    return matchingRules;
  }

  /** Adds {@code rule} to those at {@code item}. */
  public MatchingRuleCategory addRule(String item, MatchingRule rule) {
    return addRule(item, rule, RuleLogic.AND);
  }

  public MatchingRuleCategory addRule(String item, MatchingRule rule, RuleLogic ruleLogic) {
    MatchingRuleGroup group = matchingRules.get(item);
    List<MatchingRule> rules = group == null ? new ArrayList<>() : new ArrayList<>(group.getRules());
    rules.add(rule);
    matchingRules.put(item, new MatchingRuleGroup(rules, ruleLogic));
    return this;
  }

  /** Adds {@code rule} at the category's one key, as the {@code path} category has. */
  public MatchingRuleCategory addRule(MatchingRule rule) {
    return addRule("", rule);
  }

  /** Replaces the rules at {@code item} with {@code rule}. */
  public MatchingRuleCategory setRule(String item, MatchingRule rule) {
    matchingRules.put(item, new MatchingRuleGroup(List.of(rule)));
    return this;
  }

  public MatchingRuleCategory setRules(String item, MatchingRuleGroup group) {
    matchingRules.put(item, group);
    return this;
  }

  /** Adds every rule {@code other} has, at its path. */
  public MatchingRuleCategory addRules(MatchingRuleCategory other) {
    other.matchingRules.forEach((item, group) -> {
      MatchingRuleGroup mine = matchingRules.get(item);
      if (mine == null) {
        matchingRules.put(item, group);
      } else {
        List<MatchingRule> rules = new ArrayList<>(mine.getRules());
        rules.addAll(group.getRules());
        matchingRules.put(item, new MatchingRuleGroup(rules, group.getRuleLogic()));
      }
    });
    return this;
  }

  public boolean isEmpty() {
    return matchingRules.isEmpty();
  }

  public boolean isNotEmpty() {
    return !isEmpty();
  }

  /** Every rule in the category, at any path. */
  public List<MatchingRule> allMatchingRules() {
    return matchingRules.values().stream().flatMap(group -> group.getRules().stream()).toList();
  }

  /**
   * The category as a V3 pact writes it, each rule as a pact of {@code spec} does: by key, and the
   * rules at the empty key, as the {@code path} category has them, in the category itself.
   */
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    matchingRules.forEach((item, group) -> {
      if (item.isEmpty()) {
        map.putAll(group.toMap(spec));
      } else {
        map.put(item, group.toMap(spec));
      }
    });
    return map;
  }

  /** Re-keys every rule under {@code prefix}, in place of a leading {@code $}. */
  public void applyMatcherRootPrefix(String prefix) {
    Map<String, MatchingRuleGroup> rekeyed = new LinkedHashMap<>();
    matchingRules.forEach((item, group) ->
        rekeyed.put(item.startsWith("$") ? prefix + item.substring(1) : prefix + item, group));
    matchingRules.clear();
    matchingRules.putAll(rekeyed);
  }

  /** A copy with every rule re-keyed under {@code prefix}. */
  public MatchingRuleCategory copyWithUpdatedMatcherRootPrefix(String prefix) {
    MatchingRuleCategory copy = copy();
    copy.applyMatcherRootPrefix(prefix);
    return copy;
  }

  /** Adds {@code rules} at {@code item}. */
  public MatchingRuleCategory addRules(String item, java.util.List<MatchingRule> rules) {
    rules.forEach(rule -> addRule(item, rule));
    return this;
  }

  /** A copy with the same rules. */
  public MatchingRuleCategory copy() {
    return new MatchingRuleCategory(name, matchingRules);
  }

  @Override
  public String toString() {
    return "MatchingRuleCategory(name=" + name + ", matchingRules=" + matchingRules + ")";
  }
}
