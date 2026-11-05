// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.messaging;

import io.pact.janus.facade.core.model.OptionalBody;
import io.pact.janus.facade.core.model.generators.Generators;
import io.pact.janus.facade.core.model.matchingrules.MatchingRules;
import java.util.Map;

/** A V4 message's contents: its body and metadata, with their rules and generators. */
public record MessageContents(OptionalBody contents, Map<String, Object> metadata, MatchingRules matchingRules,
    Generators generators) {
  public OptionalBody getContents() {
    return contents;
  }

  public Map<String, Object> getMetadata() {
    return metadata;
  }

  public MatchingRules getMatchingRules() {
    return matchingRules;
  }

  public Generators getGenerators() {
    return generators;
  }
}
