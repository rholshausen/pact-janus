// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/junit5 and consumer; ADR 0070.
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.model.MockServerImplementation;
import io.pact.janus.facade.core.model.PactSpecVersion;
import java.lang.annotation.ElementType;
import java.lang.annotation.Inherited;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/** Which provider a test is for, and which {@code @Pact} methods build its pact. */
@Target({ElementType.TYPE, ElementType.METHOD})
@Retention(RetentionPolicy.RUNTIME)
@Inherited
public @interface PactTestFor {
  String providerName() default "";

  String hostInterface() default "";

  String port() default "";

  PactSpecVersion pactVersion() default PactSpecVersion.UNSPECIFIED;

  String pactMethod() default "";

  ProviderType providerType() default ProviderType.UNSPECIFIED;

  /** Refused: the mock serves plain HTTP (ADR 0070 decision 6). */
  boolean https() default false;

  String[] pactMethods() default {};

  String keyStorePath() default "";

  String keyStoreAlias() default "";

  String keyStorePassword() default "";

  String privateKeyPassword() default "";

  /** Refused when named: the engine runs its own mock (ADR 0070 decision 6). */
  MockServerImplementation mockServerImplementation() default MockServerImplementation.Default;
}
