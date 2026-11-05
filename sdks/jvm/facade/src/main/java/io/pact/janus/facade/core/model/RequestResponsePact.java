// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model;

import java.util.ArrayList;
import java.util.List;

/** A pact of HTTP interactions. */
public class RequestResponsePact extends BasePact {
  public RequestResponsePact(Provider provider, Consumer consumer, List<? extends Interaction> interactions) {
    this(provider, consumer, interactions, PactSpecVersion.V3);
  }

  public RequestResponsePact(Provider provider, Consumer consumer, List<? extends Interaction> interactions, PactSpecVersion version) {
    super(consumer, provider, interactions, version);
  }

  @Override
  public BasePact mergePact(BasePact other) {
    List<Interaction> merged = new ArrayList<>(getInteractions());
    merged.addAll(other.getInteractions());
    return new RequestResponsePact(getProvider(), getConsumer(), merged, getSpecVersion());
  }
}
