package io.pact.janus.facade.consumer.dsl;

import static org.junit.jupiter.api.Assertions.assertEquals;

import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.internal.Dates;
import java.util.Map;
import org.junit.jupiter.api.Test;

/**
 * A metadata value generated from an expression is what pact-jvm's {@code MetadataBuilder} builds: an
 * example in the format for 2000-01-01, its rule, and a generator carrying the expression.
 */
class MetadataBuilderTest {
  @Test
  void expressionsBuildWhatPactJvmBuilds() {
    MetadataBuilder metadata = new MetadataBuilder()
        .dateExpression("date", "today + 1 day")
        .timeExpression("time", "now + 1 hour")
        .datetimeExpression("datetime", "today + 1 day @ 1 o'clock", "yyyy-MM-dd HH:mm");

    assertEquals(Dates.local2000("yyyy-MM-dd"), metadata.getValues().get("date"));
    assertEquals(Dates.local2000("HH:mm:ss"), metadata.getValues().get("time"));
    assertEquals(Dates.local2000("yyyy-MM-dd HH:mm"), metadata.getValues().get("datetime"));

    assertEquals(Map.of("type", "Date", "format", "yyyy-MM-dd", "expression", "today + 1 day"),
        metadata.getGenerators().get("date").toMap(PactSpecVersion.V4));
    assertEquals(Map.of("type", "Time", "format", "HH:mm:ss", "expression", "now + 1 hour"),
        metadata.getGenerators().get("time").toMap(PactSpecVersion.V4));
    assertEquals(Map.of("type", "DateTime", "format", "yyyy-MM-dd HH:mm", "expression", "today + 1 day @ 1 o'clock"),
        metadata.getGenerators().get("datetime").toMap(PactSpecVersion.V4));

    Map<String, Object> rules = metadata.getMatchers().toMap(PactSpecVersion.V4);
    assertEquals(Map.of("matchers", java.util.List.of(Map.of("match", "date", "format", "yyyy-MM-dd")), "combine", "AND"),
        rules.get("date"));
    assertEquals(Map.of("matchers", java.util.List.of(Map.of("match", "time", "format", "HH:mm:ss")), "combine", "AND"),
        rules.get("time"));
    assertEquals(Map.of("matchers", java.util.List.of(Map.of("match", "timestamp", "format", "yyyy-MM-dd HH:mm")),
        "combine", "AND"), rules.get("datetime"));
  }
}
