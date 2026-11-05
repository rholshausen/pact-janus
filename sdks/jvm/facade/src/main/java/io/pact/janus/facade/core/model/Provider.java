// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model;

/** The provider a pact is between. */
public final class Provider {
  private final String name;

  public Provider(String name) {
    this.name = name;
  }

  public String getName() {
    return name;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof Provider that && name.equals(that.name);
  }

  @Override
  public int hashCode() {
    return name.hashCode();
  }

  @Override
  public String toString() {
    return "Provider(name=" + name + ")";
  }
}
