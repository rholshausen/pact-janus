// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.generators;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.Map;

/** A generator, which a mock or verifier fills a fresh value from, as a pact writes it. */
public interface Generator {
  Map<String, Object> toMap(PactSpecVersion spec);
}
