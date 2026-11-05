// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.generators;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A random hexadecimal string of {@code digits} digits. */
public final class RandomHexadecimalGenerator implements Generator {
  private final int digits;

  public RandomHexadecimalGenerator(int digits) {
    this.digits = digits;
  }

  public int getDigits() {
    return digits;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("type", "RandomHexadecimal");
    map.put("digits", digits);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof RandomHexadecimalGenerator that && java.util.Objects.equals(digits, that.digits);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(digits);
  }

  @Override
  public String toString() {
    return "RandomHexadecimalGenerator(" + "digits=" + digits + ")";
  }
}
