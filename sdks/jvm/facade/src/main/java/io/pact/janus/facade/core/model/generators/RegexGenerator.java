// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.generators;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A random string matching a regular expression. */
public final class RegexGenerator implements Generator {
  private final String regex;

  public RegexGenerator(String regex) {
    this.regex = regex;
  }

  public String getRegex() {
    return regex;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("type", "Regex");
    map.put("regex", regex);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof RegexGenerator that && java.util.Objects.equals(regex, that.regex);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(regex);
  }

  @Override
  public String toString() {
    return "RegexGenerator(" + "regex=" + regex + ")";
  }
}
