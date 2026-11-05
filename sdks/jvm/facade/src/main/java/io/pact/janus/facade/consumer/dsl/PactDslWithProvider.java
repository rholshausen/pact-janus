// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.consumer.ConsumerPactBuilder;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.ProviderState;
import java.util.LinkedHashMap;
import java.util.Map;

/** A pact with one provider, before its first interaction. */
public class PactDslWithProvider {
  private final ConsumerPactBuilder consumerPactBuilder;
  private final String providerName;
  private final PactSpecVersion version;
  private PactDslRequestWithoutPath defaultRequestValues;
  private PactDslResponse defaultResponseValues;

  public PactDslWithProvider(ConsumerPactBuilder consumerPactBuilder, String providerName) {
    this(consumerPactBuilder, providerName, PactSpecVersion.V3);
  }

  public PactDslWithProvider(ConsumerPactBuilder consumerPactBuilder, String providerName, PactSpecVersion version) {
    this.consumerPactBuilder = consumerPactBuilder;
    this.providerName = providerName;
    this.version = version;
  }

  public ConsumerPactBuilder getConsumerPactBuilder() {
    return consumerPactBuilder;
  }

  public PactSpecVersion getVersion() {
    return version;
  }

  private PactDslWithState state(ProviderState state) {
    PactDslWithState withState = new PactDslWithState(consumerPactBuilder, consumerPactBuilder.getConsumerName(),
        providerName, defaultRequestValues, defaultResponseValues, version);
    if (state != null) {
      withState.state.add(state);
    }
    return withState;
  }

  public PactDslWithState given(String state) {
    return state(new ProviderState(state));
  }

  public PactDslWithState given(String state, Map<String, ?> params) {
    return state(new ProviderState(state, params));
  }

  public PactDslWithState given(String state, String firstKey, Object firstValue, Object... paramsKeyValuePair) {
    if (paramsKeyValuePair.length % 2 != 0) {
      throw new IllegalArgumentException("Pair key value should be provided, but there is one key without value.");
    }
    Map<String, Object> params = new LinkedHashMap<>();
    params.put(firstKey, firstValue);
    for (int i = 0; i < paramsKeyValuePair.length; i += 2) {
      params.put(paramsKeyValuePair[i].toString(), paramsKeyValuePair[i + 1]);
    }
    return state(new ProviderState(state, params));
  }

  public PactDslRequestWithoutPath uponReceiving(String description) {
    return state(null).uponReceiving(description);
  }

  public void setDefaultRequestValues(PactDslRequestWithoutPath defaultRequestValues) {
    this.defaultRequestValues = defaultRequestValues;
  }

  public void setDefaultResponseValues(PactDslResponse defaultResponseValues) {
    this.defaultResponseValues = defaultResponseValues;
  }

  /** Accepted, with no effect: a contract carries no pact metadata. */
  public PactDslWithProvider addMetadataValue(String key, Object value) {
    return this;
  }
}
