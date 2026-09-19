package io.pact.janus.sdk;

/**
 * The {@code execute} closure: the consumer's test, run once per variant the engine selected, with
 * the mock armed for that variant. Blocking — it returns when the test is done. Throwing anything
 * (an assertion, an exception from the client under test) fails that variant.
 */
@FunctionalInterface
public interface VariantTest {
  void run(Mock mock, Variant variant) throws Exception;
}
