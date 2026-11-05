// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.generators;

import java.util.Locale;

/** Where in a part a generator applies. */
public enum Category {
  METHOD,
  PATH,
  HEADER,
  QUERY,
  BODY,
  STATUS,
  METADATA,
  CONTENT;

  /** As a pact names it. */
  public String key() {
    return name().toLowerCase(Locale.ROOT);
  }
}
