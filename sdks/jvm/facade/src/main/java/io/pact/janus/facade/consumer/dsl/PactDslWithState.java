// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.consumer.ConsumerPactBuilder;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.ProviderState;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/** An interaction's provider states, before its description. */
public class PactDslWithState {
  private final ConsumerPactBuilder consumerPactBuilder;
  private final String consumerName;
  private final String providerName;
  private final PactDslRequestWithoutPath defaultRequestValues;
  private final PactDslResponse defaultResponseValues;
  private final PactSpecVersion version;
  public final List<ProviderState> state = new ArrayList<>();
  private final List<String> comments = new ArrayList<>();

  public PactDslWithState(ConsumerPactBuilder consumerPactBuilder, String consumerName, String providerName,
      PactDslRequestWithoutPath defaultRequestValues, PactDslResponse defaultResponseValues, PactSpecVersion version) {
    this.consumerPactBuilder = consumerPactBuilder;
    this.consumerName = consumerName;
    this.providerName = providerName;
    this.defaultRequestValues = defaultRequestValues;
    this.defaultResponseValues = defaultResponseValues;
    this.version = version;
  }

  public PactDslRequestWithoutPath uponReceiving(String description) {
    return new PactDslRequestWithoutPath(consumerPactBuilder, consumerName, providerName, state, description,
        defaultRequestValues, defaultResponseValues, version);
  }

  public PactDslWithState given(String stateDesc) {
    state.add(new ProviderState(stateDesc));
    return this;
  }

  public PactDslWithState given(String stateDesc, Map<String, ?> params) {
    state.add(new ProviderState(stateDesc, params));
    return this;
  }

  /** Accepted, with no effect: a contract carries no comments (ADR 0066 decision 6). */
  public PactDslWithState comment(String comment) {
    comments.add(comment);
    return this;
  }

  /** Accepted, with no effect: a contract carries no pact metadata. */
  public PactDslWithState addMetadataValue(String key, Object value) {
    return this;
  }
}
