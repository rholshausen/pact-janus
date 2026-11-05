// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** Matching a regular expression. The example is not part of the rule. */
public final class RegexMatcher implements MatchingRule {
  private final String regex;
  private final String example;

  public RegexMatcher(String regex) {
    this(regex, null);
  }

  public RegexMatcher(String regex, String example) {
    this.regex = regex;
    this.example = example;
  }

  public String getRegex() {
    return regex;
  }

  public String getExample() {
    return example;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "regex");
    map.put("regex", regex);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof RegexMatcher that && java.util.Objects.equals(regex, that.regex) && java.util.Objects.equals(example, that.example);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(regex, example);
  }

  @Override
  public String toString() {
    return "RegexMatcher(" + "regex=" + regex + ", " + "example=" + example + ")";
  }
}
