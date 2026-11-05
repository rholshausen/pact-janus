// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/src/main/kotlin/.../dsl/MetadataBuilder.kt; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.core.model.generators.DateGenerator;
import io.pact.janus.facade.core.model.generators.DateTimeGenerator;
import io.pact.janus.facade.core.model.generators.Generator;
import io.pact.janus.facade.core.model.generators.ProviderStateGenerator;
import io.pact.janus.facade.core.model.generators.RandomHexadecimalGenerator;
import io.pact.janus.facade.core.model.generators.TimeGenerator;
import io.pact.janus.facade.core.model.generators.UuidGenerator;
import io.pact.janus.facade.core.model.matchingrules.DateMatcher;
import io.pact.janus.facade.core.model.matchingrules.IncludeMatcher;
import io.pact.janus.facade.core.model.matchingrules.MatchingRule;
import io.pact.janus.facade.core.model.matchingrules.MatchingRuleCategory;
import io.pact.janus.facade.core.model.matchingrules.NumberTypeMatcher;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.model.matchingrules.TimeMatcher;
import io.pact.janus.facade.core.model.matchingrules.TimestampMatcher;
import io.pact.janus.facade.core.model.matchingrules.TypeMatcher;
import io.pact.janus.facade.internal.Dates;
import java.math.BigDecimal;
import java.time.Instant;
import java.time.format.DateTimeFormatter;
import java.util.Date;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.TimeZone;
import java.util.UUID;

/** Builds a message's metadata, with rules and generators at its keys. */
public class MetadataBuilder {
  private static final String ISO_DATETIME = "yyyy-MM-dd'T'HH:mm:ss";
  private static final String ISO_DATE = "yyyy-MM-dd";
  private static final String ISO_TIME = "'T'HH:mm:ss";
  private static final String ISO_TIME_NO_T = "HH:mm:ss";

  private final Map<String, Object> values = new LinkedHashMap<>();
  private final MatchingRuleCategory matchers = new MatchingRuleCategory("metadata");
  private final Map<String, Generator> generators = new LinkedHashMap<>();

  public Map<String, Object> getValues() {
    return values;
  }

  public MatchingRuleCategory getMatchers() {
    return matchers;
  }

  public Map<String, Generator> getGenerators() {
    return generators;
  }

  private MetadataBuilder put(String name, Object value, MatchingRule rule, Generator generator) {
    values.put(name, value);
    if (rule != null) {
      matchers.addRule(name, rule);
    }
    if (generator != null) {
      generators.put(name, generator);
    }
    return this;
  }

  public MetadataBuilder add(String key, Object value) {
    values.put(key, value);
    return this;
  }

  public MetadataBuilder like(String name, Object example) {
    return put(name, example, TypeMatcher.INSTANCE, null);
  }

  public MetadataBuilder numberType(String name, Number number) {
    return put(name, number, new NumberTypeMatcher(NumberTypeMatcher.NumberType.NUMBER), null);
  }

  public MetadataBuilder integerType(String name, Long number) {
    return put(name, number, new NumberTypeMatcher(NumberTypeMatcher.NumberType.INTEGER), null);
  }

  public MetadataBuilder integerType(String name, Integer number) {
    return put(name, number, new NumberTypeMatcher(NumberTypeMatcher.NumberType.INTEGER), null);
  }

  public MetadataBuilder decimalType(String name, BigDecimal number) {
    return put(name, number, new NumberTypeMatcher(NumberTypeMatcher.NumberType.DECIMAL), null);
  }

  public MetadataBuilder decimalType(String name, Double number) {
    return put(name, number, new NumberTypeMatcher(NumberTypeMatcher.NumberType.DECIMAL), null);
  }

  public MetadataBuilder booleanType(String name, Boolean example) {
    return put(name, example, TypeMatcher.INSTANCE, null);
  }

  public MetadataBuilder matchRegex(String name, String regex, String value) {
    if (!value.matches(regex)) {
      throw new InvalidMatcherException("Example \"" + value + "\" does not match regular expression \"" + regex + "\"");
    }
    return put(name, value, new RegexMatcher(regex), null);
  }

  public MetadataBuilder datetime(String name) {
    return put(name, Dates.local2000(ISO_DATETIME), new TimestampMatcher(ISO_DATETIME), new DateTimeGenerator(ISO_DATETIME, null));
  }

  public MetadataBuilder datetime(String name, String format) {
    return put(name, Dates.localTime2000(format), new TimestampMatcher(format), new DateTimeGenerator(format, null));
  }

  public MetadataBuilder datetime(String name, String format, Date example) {
    return datetime(name, format, example.toInstant(), TimeZone.getDefault());
  }

  public MetadataBuilder datetime(String name, String format, Date example, TimeZone timeZone) {
    return datetime(name, format, example.toInstant(), timeZone);
  }

  public MetadataBuilder datetime(String name, String format, Instant example) {
    return datetime(name, format, example, TimeZone.getDefault());
  }

  public MetadataBuilder datetime(String name, String format, Instant example, TimeZone timeZone) {
    return put(name, DateTimeFormatter.ofPattern(format).withZone(timeZone.toZoneId()).format(example),
        new TimestampMatcher(format), null);
  }

  public MetadataBuilder date(String name) {
    return put(name, Dates.local2000(ISO_DATE), new DateMatcher(ISO_DATE), new DateGenerator(ISO_DATE, null));
  }

  public MetadataBuilder date(String name, String format) {
    return put(name, Dates.local2000(format), new DateMatcher(format), new DateGenerator(format, null));
  }

  public MetadataBuilder date(String name, String format, Date example) {
    return date(name, format, example, TimeZone.getDefault());
  }

  public MetadataBuilder date(String name, String format, Date example, TimeZone timeZone) {
    return put(name, Dates.format(format, timeZone, example), new DateMatcher(format), null);
  }

  public MetadataBuilder time(String name) {
    return put(name, Dates.local2000(ISO_TIME), new TimeMatcher(ISO_TIME), new TimeGenerator(ISO_TIME, null));
  }

  public MetadataBuilder time(String name, String format) {
    return put(name, Dates.local2000(format), new TimeMatcher(format), new TimeGenerator(format, null));
  }

  public MetadataBuilder time(String name, String format, Date example) {
    return time(name, format, example, TimeZone.getDefault());
  }

  public MetadataBuilder time(String name, String format, Date example, TimeZone timeZone) {
    return put(name, Dates.format(format, timeZone, example), new TimeMatcher(format), null);
  }

  public MetadataBuilder dateExpression(String name, String expression) {
    return dateExpression(name, expression, ISO_DATE);
  }

  public MetadataBuilder dateExpression(String name, String expression, String format) {
    return put(name, Dates.local2000(format), new DateMatcher(format), new DateGenerator(format, expression));
  }

  public MetadataBuilder timeExpression(String name, String expression) {
    return timeExpression(name, expression, ISO_TIME_NO_T);
  }

  public MetadataBuilder timeExpression(String name, String expression, String format) {
    return put(name, Dates.local2000(format), new TimeMatcher(format), new TimeGenerator(format, expression));
  }

  public MetadataBuilder datetimeExpression(String name, String expression) {
    return datetimeExpression(name, expression, ISO_DATETIME);
  }

  public MetadataBuilder datetimeExpression(String name, String expression, String format) {
    return put(name, Dates.local2000(format), new TimestampMatcher(format), new DateTimeGenerator(format, expression));
  }

  public MetadataBuilder ipAddress(String name) {
    return put(name, "127.0.0.1", new RegexMatcher("(\\d{1,3}\\.)+\\d{1,3}"), null);
  }

  public MetadataBuilder hexValue(String name) {
    generators.put(name, new RandomHexadecimalGenerator(10));
    return hexValue(name, "1234a");
  }

  public MetadataBuilder hexValue(String name, String hexValue) {
    if (!DslPart.HEXADECIMAL.matcher(hexValue).matches()) {
      throw new InvalidMatcherException("Example \"" + hexValue + "\" is not a valid hexadecimal value");
    }
    return put(name, hexValue, new RegexMatcher(DslPart.HEXADECIMAL.pattern()), null);
  }

  public MetadataBuilder uuid(String name) {
    generators.put(name, new UuidGenerator(null));
    return uuid(name, "e2490de5-5bd3-43d5-b7c4-526e33f71304");
  }

  public MetadataBuilder uuid(String name, UUID uuid) {
    return uuid(name, uuid.toString());
  }

  public MetadataBuilder uuid(String name, String uuid) {
    if (!DslPart.UUID_REGEX.matcher(uuid).matches()) {
      throw new InvalidMatcherException("Example \"" + uuid + "\" is not a valid UUID");
    }
    return put(name, uuid, new RegexMatcher(DslPart.UUID_REGEX.pattern()), null);
  }

  public MetadataBuilder includesStr(String name, String value) {
    return put(name, value, new IncludeMatcher(value), null);
  }

  public MetadataBuilder matchUrl(String name, String basePath, Object... pathFragments) {
    String[] url = DslPart.urlMatcher(basePath, pathFragments);
    return put(name, url[0], new RegexMatcher(url[1]), null);
  }

  public MetadataBuilder valueFromProviderState(String name, String expression, Object example) {
    return put(name, example, null, new ProviderStateGenerator(expression, DslPart.dataType(example)));
  }
}
