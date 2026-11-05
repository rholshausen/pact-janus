// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.generators;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** The current date and time, or an expression's, in a format. */
public final class DateTimeGenerator implements Generator {
  private final String format;
  private final String expression;

  public DateTimeGenerator(String format, String expression) {
    this.format = format;
    this.expression = expression;
  }

  public String getFormat() {
    return format;
  }

  public String getExpression() {
    return expression;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("type", "DateTime");
    if (format != null && !format.isEmpty()) {
      map.put("format", format);
    }
    if (expression != null && !expression.isEmpty()) {
      map.put("expression", expression);
    }
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof DateTimeGenerator that && java.util.Objects.equals(format, that.format) && java.util.Objects.equals(expression, that.expression);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(format, expression);
  }

  @Override
  public String toString() {
    return "DateTimeGenerator(" + "format=" + format + ", " + "expression=" + expression + ")";
  }
}
