// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model;

import java.util.LinkedHashMap;
import java.util.Map;

/** A provider state an interaction is given, with its parameters. */
public final class ProviderState {
  private final String name;
  private final Map<String, Object> params;

  public ProviderState(String name) {
    this(name, Map.of());
  }

  public ProviderState(String name, Map<String, ?> params) {
    this.name = name;
    this.params = new LinkedHashMap<>(params == null ? Map.of() : params);
  }

  public String getName() {
    return name;
  }

  public Map<String, Object> getParams() {
    return params;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof ProviderState state && name.equals(state.name) && params.equals(state.params);
  }

  @Override
  public int hashCode() {
    return name.hashCode() * 31 + params.hashCode();
  }

  @Override
  public String toString() {
    return "ProviderState(name=" + name + ", params=" + params + ")";
  }
}
