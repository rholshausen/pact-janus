// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/junit5 and consumer; ADR 0070.
package io.pact.janus.facade.consumer;

/** The mock a test's requests go to, for one provider. */
public interface MockServer {
  /** The mock's base URL, {@code http://host:port}. */
  String getUrl();

  int getPort();
}
