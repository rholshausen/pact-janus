// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.generators;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A random boolean. */
public final class RandomBooleanGenerator implements Generator {


  /** The one instance, as pact-jvm's generator object is. */
  public static final RandomBooleanGenerator INSTANCE = new RandomBooleanGenerator();

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("type", "RandomBoolean");
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof RandomBooleanGenerator that && true;
  }

  @Override
  public int hashCode() {
    return "RandomBooleanGenerator".hashCode();
  }

  @Override
  public String toString() {
    return "RandomBooleanGenerator";
  }
}
