// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/junit5 and consumer; ADR 0070.
package io.pact.janus.facade.consumer.model;

/** Which mock server pact-jvm runs. The engine runs its own, so naming one is refused (ADR 0070 decision 6). */
public enum MockServerImplementation {
  JavaHttpServer,
  KTorServer,
  Plugin,
  Default
}
