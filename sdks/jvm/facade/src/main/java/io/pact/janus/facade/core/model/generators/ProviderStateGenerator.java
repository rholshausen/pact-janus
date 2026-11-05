// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.generators;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A value from the provider state. */
public final class ProviderStateGenerator implements Generator {
  private final String expression;
  private final io.pact.janus.facade.core.support.expressions.DataType dataType;

  public ProviderStateGenerator(String expression, io.pact.janus.facade.core.support.expressions.DataType dataType) {
    this.expression = expression;
    this.dataType = dataType;
  }

  public String getExpression() {
    return expression;
  }

  public io.pact.janus.facade.core.support.expressions.DataType getDataType() {
    return dataType;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("type", "ProviderState");
    map.put("expression", expression);
    map.put("dataType", dataType.name());
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof ProviderStateGenerator that && java.util.Objects.equals(expression, that.expression) && java.util.Objects.equals(dataType, that.dataType);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(expression, dataType);
  }

  @Override
  public String toString() {
    return "ProviderStateGenerator(" + "expression=" + expression + ", " + "dataType=" + dataType + ")";
  }
}
