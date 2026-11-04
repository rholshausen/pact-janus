package io.pact.janus.sdk;

/**
 * The {@code execute} closure for a message: the consumer's test, run once per variant the engine
 * selected, handed that variant's message (ADR 0068 decision 3). Throwing anything fails the variant.
 */
@FunctionalInterface
public interface MessageTest {
  void run(ReceivedMessage message, Variant variant) throws Exception;
}
