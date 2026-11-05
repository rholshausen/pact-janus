// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.core.model.generators.DateGenerator;
import io.pact.janus.facade.core.model.generators.DateTimeGenerator;
import io.pact.janus.facade.core.model.generators.ProviderStateGenerator;
import io.pact.janus.facade.core.model.generators.RandomBooleanGenerator;
import io.pact.janus.facade.core.model.generators.RandomDecimalGenerator;
import io.pact.janus.facade.core.model.generators.RandomHexadecimalGenerator;
import io.pact.janus.facade.core.model.generators.RandomIntGenerator;
import io.pact.janus.facade.core.model.generators.RandomStringGenerator;
import io.pact.janus.facade.core.model.generators.TimeGenerator;
import io.pact.janus.facade.core.model.generators.UuidGenerator;
import io.pact.janus.facade.core.model.matchingrules.DateMatcher;
import io.pact.janus.facade.core.model.matchingrules.EqualsMatcher;
import io.pact.janus.facade.core.model.matchingrules.IncludeMatcher;
import io.pact.janus.facade.core.model.matchingrules.NotEmptyMatcher;
import io.pact.janus.facade.core.model.matchingrules.NullMatcher;
import io.pact.janus.facade.core.model.matchingrules.NumberTypeMatcher;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.model.matchingrules.TimeMatcher;
import io.pact.janus.facade.core.model.matchingrules.TimestampMatcher;
import io.pact.janus.facade.core.model.matchingrules.TypeMatcher;
import io.pact.janus.facade.core.support.expressions.DataType;
import io.pact.janus.facade.internal.Dates;
import io.pact.janus.facade.internal.RegexExample;
import java.util.regex.Pattern;

/** Values with their rules, for headers, query parameters and message metadata. */
public final class Matchers {
  public static final Pattern HEXADECIMAL = Pattern.compile("[0-9a-fA-F]+");
  public static final Pattern IP_ADDRESS = Pattern.compile("(\\d{1,3}\\.)+\\d{1,3}");
  public static final Pattern UUID_REGEX =
      Pattern.compile("[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}");

  private static final String ISO_DATETIME = "yyyy-MM-dd'T'HH:mm:ss";
  private static final String ISO_TIME = "HH:mm:ss";
  private static final String ISO_DATE = "yyyy-MM-dd";

  private Matchers() {}

  public static Matcher regexp(Pattern re) {
    return regexp(re, null);
  }

  /**
   * A value matching {@code re}; given no example, one drawn from the pattern the same way every run,
   * with a generator, as pact-jvm draws one (ADR 0070 decision 3).
   */
  public static Matcher regexp(Pattern re, String value) {
    if (value == null || value.isEmpty()) {
      return new Matcher(RegexExample.of(re.toString()), new RegexMatcher(re.toString(), null),
          new io.pact.janus.facade.core.model.generators.RegexGenerator(re.toString()));
    }
    if (!re.matcher(value).matches()) {
      throw new InvalidMatcherException("Example \"" + value + "\" does not match regular expression \"" + re + "\"");
    }
    return new Matcher(value, new RegexMatcher(re.toString(), value), null);
  }

  public static Matcher regexp(String regexp) {
    return regexp(Pattern.compile(regexp), null);
  }

  public static Matcher regexp(String regexp, String value) {
    return regexp(Pattern.compile(regexp), value);
  }

  public static Matcher hexValue() {
    return hexValue(null);
  }

  /** A hexadecimal value; given none, {@code 1234a} with a generator, as pact-jvm's body DSL has. */
  public static Matcher hexValue(String value) {
    if (value != null && !value.isEmpty() && !HEXADECIMAL.matcher(value).matches()) {
      throw new InvalidMatcherException("Example \"" + value + "\" is not a hexadecimal value");
    }
    return new Matcher(value == null ? "1234a" : value, new RegexMatcher(HEXADECIMAL.toString(), value),
        value == null ? new RandomHexadecimalGenerator(10) : null);
  }

  public static Matcher identifier() {
    return identifier(null);
  }

  public static Matcher identifier(Object value) {
    return new Matcher(value == null ? 12345678 : value, new NumberTypeMatcher(NumberTypeMatcher.NumberType.INTEGER),
        value == null ? new RandomIntGenerator(0, Integer.MAX_VALUE) : null);
  }

  public static Matcher ipAddress() {
    return ipAddress(null);
  }

  public static Matcher ipAddress(String value) {
    if (value != null && !value.isEmpty() && !IP_ADDRESS.matcher(value).matches()) {
      throw new InvalidMatcherException("Example \"" + value + "\" is not an ip adress");
    }
    return new Matcher("127.0.0.1", new RegexMatcher(IP_ADDRESS.toString(), "127.0.0.1"), null);
  }

  public static Matcher numeric() {
    return numeric(null);
  }

  public static Matcher numeric(Number value) {
    return new Matcher(value == null ? 100 : value, new NumberTypeMatcher(NumberTypeMatcher.NumberType.NUMBER),
        value == null ? new RandomDecimalGenerator(6) : null);
  }

  public static Matcher decimal() {
    return decimal(null);
  }

  public static Matcher decimal(Number value) {
    return new Matcher(value == null ? 100.0 : value, new NumberTypeMatcher(NumberTypeMatcher.NumberType.DECIMAL),
        value == null ? new RandomDecimalGenerator(6) : null);
  }

  public static Matcher integer() {
    return integer(null);
  }

  public static Matcher integer(Long value) {
    return new Matcher(value == null ? 100 : value, new NumberTypeMatcher(NumberTypeMatcher.NumberType.INTEGER),
        value == null ? new RandomIntGenerator(0, Integer.MAX_VALUE) : null);
  }

  public static Matcher timestamp() {
    return timestamp(null, null);
  }

  public static Matcher timestamp(String pattern) {
    return timestamp(pattern, null);
  }

  public static Matcher timestamp(String pattern, String value) {
    String format = pattern == null ? ISO_DATETIME : pattern;
    Dates.validate(value, format);
    return new Matcher(value == null ? Dates.utc2000(format) : value, new TimestampMatcher(format),
        value == null ? new DateTimeGenerator(format, null) : null);
  }

  public static Matcher time() {
    return time(null, null);
  }

  public static Matcher time(String pattern) {
    return time(pattern, null);
  }

  public static Matcher time(String pattern, String value) {
    String format = pattern == null ? ISO_TIME : pattern;
    Dates.validate(value, format);
    return new Matcher(value == null ? Dates.utc2000(format) : value, new TimeMatcher(format),
        value == null ? new TimeGenerator(format, null) : null);
  }

  public static Matcher date() {
    return date(null, null);
  }

  public static Matcher date(String pattern) {
    return date(pattern, null);
  }

  public static Matcher date(String pattern, String value) {
    String format = pattern == null ? ISO_DATE : pattern;
    Dates.validate(value, format);
    return new Matcher(value == null ? Dates.utc2000(format) : value, new DateMatcher(format),
        value == null ? new DateGenerator(format, null) : null);
  }

  public static Matcher uuid() {
    return uuid(null);
  }

  /** A UUID; given none, a fixed one with a generator, as pact-jvm's body DSL has. */
  public static Matcher uuid(String value) {
    if (value != null && !value.isEmpty() && !UUID_REGEX.matcher(value).matches()) {
      throw new InvalidMatcherException("Example \"" + value + "\" is not a UUID");
    }
    return new Matcher(value == null ? "e2490de5-5bd3-43d5-b7c4-526e33f71304" : value,
        new RegexMatcher(UUID_REGEX.toString(), value), value == null ? new UuidGenerator(null) : null);
  }

  public static Matcher string() {
    return string(null);
  }

  public static Matcher string(String value) {
    return value != null
        ? new Matcher(value, TypeMatcher.INSTANCE, null)
        : new Matcher("string", TypeMatcher.INSTANCE, new RandomStringGenerator(10));
  }

  public static Matcher bool() {
    return bool(null);
  }

  public static Matcher bool(Boolean value) {
    return value != null
        ? new Matcher(value, TypeMatcher.INSTANCE, null)
        : new Matcher(true, TypeMatcher.INSTANCE, RandomBooleanGenerator.INSTANCE);
  }

  public static Matcher equalTo(Object value) {
    return new Matcher(value, EqualsMatcher.INSTANCE, null);
  }

  public static Matcher includesStr(String value) {
    return new Matcher(value, new IncludeMatcher(value), null);
  }

  public static Matcher nullValue() {
    return new Matcher(null, NullMatcher.INSTANCE, null);
  }

  public static Matcher fromProviderState(String expression, String value) {
    return new Matcher(value, null, new ProviderStateGenerator(expression, DataType.RAW));
  }

  public static Matcher notEmpty(String example) {
    return new Matcher(example, NotEmptyMatcher.INSTANCE, null);
  }
}
