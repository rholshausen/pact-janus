// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/src/main/kotlin/.../dsl/PactDslJsonArrayContaining.kt; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.core.model.generators.Category;
import io.pact.janus.facade.core.model.generators.Generator;
import io.pact.janus.facade.core.model.matchingrules.ArrayContainsMatcher;
import io.pact.janus.facade.core.model.matchingrules.MatchingRuleCategory;
import io.pact.janus.facade.core.model.matchingrules.MatchingRuleGroup;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** An array that contains an item like each of its items, in any order. */
public class PactDslJsonArrayContaining extends PactDslJsonArray {
  private static final Pattern PREFIX = Pattern.compile("^\\[(\\d+)]");

  private final String root;

  public PactDslJsonArrayContaining(String root, String rootName, DslPart parent) {
    super("", rootName, parent);
    this.root = root;
  }

  private static int index(String key) {
    Matcher matcher = PREFIX.matcher(key);
    return matcher.find() ? Integer.parseInt(matcher.group(1)) : -1;
  }

  @Override
  public DslPart closeArray() {
    Map<Integer, MatchingRuleCategory> rules = new LinkedHashMap<>();
    matchers.getMatchingRules().forEach((key, group) -> rules
        .computeIfAbsent(index(key), i -> new MatchingRuleCategory("body"))
        .setRules(PREFIX.matcher(key).replaceFirst("\\$"), group));
    Map<Integer, Map<String, Generator>> generatorsByIndex = new LinkedHashMap<>();
    Map<String, Generator> body = generators.getCategories().get(Category.BODY);
    if (body != null) {
      body.forEach((key, generator) -> generatorsByIndex
          .computeIfAbsent(index(key), i -> new LinkedHashMap<>())
          .put(PREFIX.matcher(key).replaceFirst("\\$"), generator));
    }
    List<ArrayContainsMatcher.Variant> variants = new ArrayList<>();
    for (int index = 0; index < getBody().size(); index++) {
      variants.add(new ArrayContainsMatcher.Variant(index,
          rules.getOrDefault(index, new MatchingRuleCategory("body")),
          generatorsByIndex.getOrDefault(index, Map.of())));
    }
    MatchingRuleCategory category = new MatchingRuleCategory("");
    category.setRules(root + rootName, new MatchingRuleGroup(List.of(new ArrayContainsMatcher(variants))));
    matchers = category;
    if (body != null) {
      body.clear();
    }
    return super.closeArray();
  }
}
