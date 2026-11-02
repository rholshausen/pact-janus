package io.pact.janus.sdk;

import io.pact.janus.bindings.protocol.v1.VariantResult;

/**
 * One variant the engine did not verify, and why: its {@code result}, as the engine answered the
 * test's report (ADR 0064), and its {@code cause}, what the test threw, or an {@link AssertionError}
 * saying the engine's verdict when the test passed.
 */
public record VariantFailure(Variant variant, VariantResult result, Throwable cause) {
  @Override
  public String toString() {
    return variant.displayName() + ": " + cause;
  }
}
