// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

/** How the rules at one path combine. */
public enum RuleLogic {
  AND,
  OR
}
