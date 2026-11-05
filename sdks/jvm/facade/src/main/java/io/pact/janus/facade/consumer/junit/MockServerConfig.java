// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/junit5 and consumer; ADR 0070.
package io.pact.janus.facade.consumer.junit;

import io.pact.janus.facade.consumer.model.MockServerImplementation;
import java.lang.annotation.ElementType;
import java.lang.annotation.Inherited;
import java.lang.annotation.Repeatable;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/** Where a provider's mock listens. TLS and a named implementation are refused (ADR 0070 decision 6). */
@Target({ElementType.TYPE, ElementType.METHOD})
@Retention(RetentionPolicy.RUNTIME)
@Inherited
@Repeatable(MockServers.class)
public @interface MockServerConfig {
  MockServerImplementation implementation() default MockServerImplementation.Default;

  String registryEntry() default "";

  String hostInterface() default "";

  String port() default "";

  boolean tls() default false;

  String keyStorePath() default "";

  String keyStoreAlias() default "";

  String keyStorePassword() default "";

  String privateKeyPassword() default "";

  String providerName() default "";

  TransportConfigurationEntry[] transportConfig() default {};
}
