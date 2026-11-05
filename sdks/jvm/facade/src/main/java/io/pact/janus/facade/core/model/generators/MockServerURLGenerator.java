// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.generators;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A URL on the mock server, in place of the example's base. */
public final class MockServerURLGenerator implements Generator {
  private final String example;
  private final String regex;

  public MockServerURLGenerator(String example, String regex) {
    this.example = example;
    this.regex = regex;
  }

  public String getExample() {
    return example;
  }

  public String getRegex() {
    return regex;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("type", "MockServerURL");
    map.put("example", example);
    map.put("regex", regex);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof MockServerURLGenerator that && java.util.Objects.equals(example, that.example) && java.util.Objects.equals(regex, that.regex);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(example, regex);
  }

  @Override
  public String toString() {
    return "MockServerURLGenerator(" + "example=" + example + ", " + "regex=" + regex + ")";
  }
}
