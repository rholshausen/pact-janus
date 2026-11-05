// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model;

import java.util.List;
import java.util.Map;

/** One interaction a pact holds. */
public interface Interaction {
  String getDescription();

  List<ProviderState> getProviderStates();

  /**
   * The {@code pact_ffi} calls that build what pact-jvm built, which {@code add-pact-interaction}
   * makes (ADR 0070). Not API.
   */
  List<Map<String, Object>> calls(PactSpecVersion spec);
}
