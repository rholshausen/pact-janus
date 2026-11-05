// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A part's matching rules, by category. */
public interface MatchingRules {
  /** The category named {@code category}, empty when there is none. */
  MatchingRuleCategory rulesForCategory(String category);

  /** The category named {@code category}, added when there is none. */
  MatchingRuleCategory addCategory(String category);

  /** Adds every rule {@code category} has to the category of its name. */
  MatchingRuleCategory addCategory(MatchingRuleCategory category);

  Map<String, MatchingRuleCategory> getCategories();

  default boolean isEmpty() {
    return getCategories().values().stream().allMatch(MatchingRuleCategory::isEmpty);
  }

  default boolean isNotEmpty() {
    return !isEmpty();
  }

  /** The rules as a pact of {@code spec} writes them, leaving out empty categories. */
  default Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    getCategories().forEach((name, category) -> {
      if (category.isNotEmpty()) {
        map.put(name, category.toMap(spec));
      }
    });
    return map;
  }
}
