// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.Map;

/** One matching rule, as a pact writes it. */
public interface MatchingRule {
  /** The rule as a pact of {@code spec} writes it. */
  Map<String, Object> toMap(PactSpecVersion spec);
}
