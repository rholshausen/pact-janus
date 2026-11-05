// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.core.model.generators.Generators;
import io.pact.janus.facade.core.model.matchingrules.MatchingRuleCategory;

/** A builder of a body that is not JSON: its octets, its type, and its rules and generators. */
public interface BodyBuilder {
  MatchingRuleCategory getMatchers();

  Generators getGenerators();

  /** The body's content type, as a header writes it. */
  String getContentType();

  byte[] buildBody();

  default MatchingRuleCategory getHeaderMatchers() {
    return null;
  }
}
