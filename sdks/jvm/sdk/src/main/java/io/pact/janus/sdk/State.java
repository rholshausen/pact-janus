package io.pact.janus.sdk;

import java.lang.annotation.Documented;
import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marks a test-class method as the handler of provider states, for {@link JanusProviderExtension}
 * (ADR 0056 decision 3). The method takes no parameter, or the state's parameters as a {@code Map}.
 * A setup may return a {@code Map} of provider-state values; throwing
 * {@link StateUnsupportedException} says the provider cannot reach the state.
 */
@Documented
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface State {
  /** The provider states, by name, exactly as the pacts record them. */
  String[] value();

  /** Whether this method sets the states up or tears them down. */
  Action action() default Action.SETUP;

  /** The two halves of a state handler. */
  enum Action {
    SETUP,
    TEARDOWN
  }
}
