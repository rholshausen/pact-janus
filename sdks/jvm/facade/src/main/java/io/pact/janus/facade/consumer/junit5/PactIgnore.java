// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/junit5 and consumer; ADR 0070.
package io.pact.janus.facade.consumer.junit5;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/** A test the extension leaves alone. */
@Target(ElementType.METHOD)
@Retention(RetentionPolicy.RUNTIME)
public @interface PactIgnore {}
