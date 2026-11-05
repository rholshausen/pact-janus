// Ported from pact-jvm at 1928f2247 (4.7.8), core/support; ADR 0070.
package io.pact.janus.facade.core.support.expressions;

/** The type a provider-state expression's value is converted to. */
public enum DataType {
  STRING,
  INTEGER,
  DECIMAL,
  FLOAT,
  RAW,
  BOOLEAN
}
