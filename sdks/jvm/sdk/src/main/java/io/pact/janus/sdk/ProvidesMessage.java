package io.pact.janus.sdk;

import java.lang.annotation.Documented;
import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marks a test-class method as the producer of a message interaction, for
 * {@link JanusProviderExtension} (ADR 0056 decision 3). The method takes no parameter, or today's
 * request for the message as a {@code Map}, and returns a {@link Message}, bytes, a string, or any
 * other value, which is written as JSON.
 */
@Documented
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface ProvidesMessage {
  /** The message interaction's description, exactly as the pacts record it. */
  String value();
}
