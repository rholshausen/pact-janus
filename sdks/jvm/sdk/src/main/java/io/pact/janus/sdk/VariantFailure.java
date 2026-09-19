package io.pact.janus.sdk;

/** One variant the closure failed on, and why. */
public record VariantFailure(Variant variant, Throwable cause) {
  @Override
  public String toString() {
    return variant.displayName() + ": " + cause;
  }
}
