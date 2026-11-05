// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model;

/** The pact specification a pact is written to, as pact-jvm names it. */
public enum PactSpecVersion {
  /** No version given; the facade uses its default. */
  UNSPECIFIED,
  V1,
  V1_1,
  V2,
  V3,
  V4;

  /** At least {@code other}, an unspecified version counting as V3, pact-jvm's DSL default. */
  public boolean atLeast(PactSpecVersion other) {
    return effective().ordinal() >= other.ordinal();
  }

  /** This version, V3 when it is unspecified. */
  public PactSpecVersion effective() {
    return this == UNSPECIFIED ? V3 : this;
  }

  /** The specification {@code add-pact-interaction} is told: V2 for V1 and V1.1, which it has none of. */
  public String forEngine() {
    PactSpecVersion version = effective();
    return version.ordinal() < V2.ordinal() ? "V2" : version.name();
  }
}
