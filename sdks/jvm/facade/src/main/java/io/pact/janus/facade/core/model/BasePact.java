// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model;

import java.util.ArrayList;
import java.util.List;

/** A pact between a consumer and a provider: its interactions, as pact-jvm's DSL built them. */
public abstract class BasePact {
  private final Consumer consumer;
  private final Provider provider;
  private final List<Interaction> interactions;
  private final PactSpecVersion specVersion;

  protected BasePact(Consumer consumer, Provider provider, List<? extends Interaction> interactions,
      PactSpecVersion specVersion) {
    this.consumer = consumer;
    this.provider = provider;
    this.interactions = new ArrayList<>(interactions);
    this.specVersion = specVersion;
  }

  public Consumer getConsumer() {
    return consumer;
  }

  public Provider getProvider() {
    return provider;
  }

  public List<Interaction> getInteractions() {
    return interactions;
  }

  /** The specification the DSL built the pact to. */
  public PactSpecVersion getSpecVersion() {
    return specVersion;
  }

  /** This pact with {@code other}'s interactions after its own. */
  public abstract BasePact mergePact(BasePact other);

  @Override
  public String toString() {
    return getClass().getSimpleName() + "(consumer=" + consumer.getName() + ", provider=" + provider.getName()
        + ", interactions=" + interactions + ")";
  }
}
