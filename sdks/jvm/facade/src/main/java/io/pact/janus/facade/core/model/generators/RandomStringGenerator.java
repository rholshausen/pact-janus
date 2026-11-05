// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.generators;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A random string of {@code size} characters. */
public final class RandomStringGenerator implements Generator {
  private final int size;

  public RandomStringGenerator(int size) {
    this.size = size;
  }

  public int getSize() {
    return size;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("type", "RandomString");
    map.put("size", size);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof RandomStringGenerator that && java.util.Objects.equals(size, that.size);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(size);
  }

  @Override
  public String toString() {
    return "RandomStringGenerator(" + "size=" + size + ")";
  }
}
