// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/junit5 and consumer; ADR 0070.
package io.pact.janus.facade.consumer.junit5;

/** What a provider is: an HTTP one, or one that sends messages. */
public enum ProviderType {
  SYNCH,
  ASYNCH,
  SYNCH_MESSAGE,
  UNSPECIFIED
}
