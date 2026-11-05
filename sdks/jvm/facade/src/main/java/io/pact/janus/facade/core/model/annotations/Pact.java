// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.annotations;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/** A method that builds a pact for a consumer test, from the builder it is given. */
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface Pact {
  /** The provider; empty takes the test's. */
  String provider() default "";

  /** The consumer. */
  String consumer();
}
