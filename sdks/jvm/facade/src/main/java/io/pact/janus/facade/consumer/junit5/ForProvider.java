// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/junit5 and consumer; ADR 0070.
package io.pact.janus.facade.consumer.junit5;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/** Which provider a parameter is for, in a test of several. */
@Target(ElementType.PARAMETER)
@Retention(RetentionPolicy.RUNTIME)
public @interface ForProvider {
  String value();
}
