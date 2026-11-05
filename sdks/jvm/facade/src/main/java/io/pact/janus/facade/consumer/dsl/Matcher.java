// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.core.model.generators.Generator;
import io.pact.janus.facade.core.model.matchingrules.MatchingRule;

/** A value with the rule it is matched by and the generator that fills it, as {@link Matchers} makes them. */
public class Matcher {
  private final Object value;
  private final MatchingRule matcher;
  private final Generator generator;

  public Matcher(Object value, MatchingRule matcher, Generator generator) {
    this.value = value;
    this.matcher = matcher;
    this.generator = generator;
  }

  public Object getValue() {
    return value;
  }

  public MatchingRule getMatcher() {
    return matcher;
  }

  public Generator getGenerator() {
    return generator;
  }

  @Override
  public String toString() {
    return "Matcher(value=" + value + ", matcher=" + matcher + ", generator=" + generator + ")";
  }
}
