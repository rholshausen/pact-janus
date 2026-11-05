// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/junit5 and consumer; ADR 0070.
package io.pact.janus.facade.consumer.junit;

import java.lang.annotation.ElementType;
import java.lang.annotation.Inherited;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/** Several {@link MockServerConfig}s, one per provider. */
@Target({ElementType.TYPE, ElementType.METHOD})
@Retention(RetentionPolicy.RUNTIME)
@Inherited
public @interface MockServers {
  MockServerConfig[] value();
}
