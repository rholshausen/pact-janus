// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.generators;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A random UUID, in a format when one is named. */
public final class UuidGenerator implements Generator {
  private final String format;

  public UuidGenerator(String format) {
    this.format = format;
  }

  public String getFormat() {
    return format;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("type", "Uuid");
    if (format != null) {
      map.put("format", format);
    }
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof UuidGenerator that && java.util.Objects.equals(format, that.format);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(format);
  }

  @Override
  public String toString() {
    return "UuidGenerator(" + "format=" + format + ")";
  }
}
