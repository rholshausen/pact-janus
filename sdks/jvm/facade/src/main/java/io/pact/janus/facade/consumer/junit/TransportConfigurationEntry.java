// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/junit5 and consumer; ADR 0070.
package io.pact.janus.facade.consumer.junit;

import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;

/** A transport setting, for a plugin's mock. */
@Retention(RetentionPolicy.RUNTIME)
public @interface TransportConfigurationEntry {
  String key();

  String value();
}
