// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

/** An example that does not match the rule it is given with. */
public class InvalidMatcherException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  public InvalidMatcherException(String message) {
    super(message);
  }
}
