// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/src/main/kotlin/.../dsl/FormPostBuilder.kt; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.core.model.generators.Category;
import io.pact.janus.facade.core.model.generators.DateGenerator;
import io.pact.janus.facade.core.model.generators.DateTimeGenerator;
import io.pact.janus.facade.core.model.generators.Generator;
import io.pact.janus.facade.core.model.generators.Generators;
import io.pact.janus.facade.core.model.generators.ProviderStateGenerator;
import io.pact.janus.facade.core.model.generators.RandomHexadecimalGenerator;
import io.pact.janus.facade.core.model.generators.TimeGenerator;
import io.pact.janus.facade.core.model.generators.UuidGenerator;
import io.pact.janus.facade.core.model.matchingrules.DateMatcher;
import io.pact.janus.facade.core.model.matchingrules.IncludeMatcher;
import io.pact.janus.facade.core.model.matchingrules.MatchingRule;
import io.pact.janus.facade.core.model.matchingrules.MatchingRuleCategory;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.model.matchingrules.TimeMatcher;
import io.pact.janus.facade.core.model.matchingrules.TimestampMatcher;
import io.pact.janus.facade.core.support.expressions.DataType;
import io.pact.janus.facade.internal.ContentTypes;
import io.pact.janus.facade.internal.Dates;
import io.pact.janus.facade.internal.RegexExample;
import java.net.URLEncoder;
import java.nio.charset.Charset;
import java.time.Instant;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.Date;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.TimeZone;
import java.util.UUID;

/** Builds an {@code application/x-www-form-urlencoded} body, with rules at its parameters. */
public class FormPostBuilder implements BodyBuilder {
  /** Apache HttpCore's form type, which pact-jvm uses, with its charset. */
  public static final String APPLICATION_FORM_URLENCODED = "application/x-www-form-urlencoded; charset=ISO-8859-1";
  public static final String ROOT = "$.";
  private static final String ISO_DATETIME = "yyyy-MM-dd'T'HH:mm:ss";
  private static final String ISO_DATE = "yyyy-MM-dd";
  private static final String ISO_TIME = "'T'HH:mm:ss";
  private static final String ISO_TIME_NO_T = "HH:mm:ss";

  private final Map<String, List<String>> body = new LinkedHashMap<>();
  private String contentType = APPLICATION_FORM_URLENCODED;
  private final MatchingRuleCategory matchers = new MatchingRuleCategory("body");
  private final Generators generators = new Generators();

  public FormPostBuilder() {}

  public FormPostBuilder(String contentType) {
    this.contentType = contentType;
  }

  public Map<String, List<String>> getBody() {
    return body;
  }

  private FormPostBuilder put(String name, String value, MatchingRule rule, Generator generator) {
    body.put(name, List.of(value));
    if (rule != null) {
      matchers.addRule(ROOT + name, rule);
    }
    if (generator != null) {
      generators.addGenerator(Category.BODY, ROOT + name, generator);
    }
    return this;
  }

  public FormPostBuilder stringValue(String name, String value) {
    body.put(name, new ArrayList<>(List.of(value)));
    return this;
  }

  public FormPostBuilder stringValue(String name, String... values) {
    body.put(name, new ArrayList<>(List.of(values)));
    return this;
  }

  public FormPostBuilder stringMatcher(String name, String regex, String... values) {
    for (String value : values) {
      if (!value.matches(regex)) {
        throw new InvalidMatcherException("Example \"" + value + "\" does not match regular expression \"" + regex + "\"");
      }
    }
    body.put(name, List.of(values));
    matchers.addRule(ROOT + name, new RegexMatcher(regex, null));
    return this;
  }

  /** A parameter matching {@code regex}, its example drawn from the pattern the same way every run (ADR 0070 decision 3). */
  public FormPostBuilder stringMatcher(String name, String regex) {
    generators.addGenerator(Category.BODY, name, new io.pact.janus.facade.core.model.generators.RegexGenerator(regex));
    return stringMatcher(name, regex, RegexExample.of(regex));
  }

  public FormPostBuilder datetime(String name) {
    return put(name, Dates.local2000(ISO_DATETIME), new TimestampMatcher(ISO_DATETIME), new DateTimeGenerator(ISO_DATETIME, null));
  }

  public FormPostBuilder datetime(String name, String format) {
    return put(name, Dates.localTime2000(format), new TimestampMatcher(format), new DateTimeGenerator(format, null));
  }

  public FormPostBuilder datetime(String name, String format, Date example) {
    return datetime(name, format, example, TimeZone.getDefault());
  }

  public FormPostBuilder datetime(String name, String format, Date example, TimeZone timeZone) {
    return datetime(name, format, example.toInstant(), timeZone);
  }

  public FormPostBuilder datetime(String name, String format, Instant example) {
    return datetime(name, format, example, TimeZone.getDefault());
  }

  public FormPostBuilder datetime(String name, String format, Instant example, TimeZone timeZone) {
    String value = DateTimeFormatter.ofPattern(format).withZone(timeZone.toZoneId()).format(example);
    return put(name, value, new TimestampMatcher(format), null);
  }

  public FormPostBuilder date(String name) {
    return put(name, Dates.local2000(ISO_DATE), new DateMatcher(ISO_DATE), new DateGenerator(ISO_DATE, null));
  }

  public FormPostBuilder date(String name, String format) {
    return put(name, Dates.local2000(format), new DateMatcher(format), new DateGenerator(format, null));
  }

  public FormPostBuilder date(String name, String format, Date example) {
    return date(name, format, example, TimeZone.getDefault());
  }

  public FormPostBuilder date(String name, String format, Date example, TimeZone timeZone) {
    return put(name, Dates.format(format, timeZone, example), new DateMatcher(format), null);
  }

  public FormPostBuilder time(String name) {
    return put(name, Dates.local2000(ISO_TIME), new TimeMatcher(ISO_TIME), new TimeGenerator(ISO_TIME, null));
  }

  public FormPostBuilder time(String name, String format) {
    return put(name, Dates.local2000(format), new TimeMatcher(format), new TimeGenerator(format, null));
  }

  public FormPostBuilder time(String name, String format, Date example) {
    return time(name, format, example, TimeZone.getDefault());
  }

  public FormPostBuilder time(String name, String format, Date example, TimeZone timeZone) {
    return put(name, Dates.format(format, timeZone, example), new TimeMatcher(format), null);
  }

  public FormPostBuilder hexValue(String name) {
    return put(name, "1234a", new RegexMatcher("[0-9a-fA-F]+", null), new RandomHexadecimalGenerator(10));
  }

  public FormPostBuilder hexValue(String name, String hexValue) {
    if (!DslPart.HEXADECIMAL.matcher(hexValue).matches()) {
      throw new InvalidMatcherException("Example \"" + hexValue + "\" is not a hexadecimal value");
    }
    return put(name, hexValue, new RegexMatcher("[0-9a-fA-F]+", null), null);
  }

  public FormPostBuilder uuid(String name) {
    generators.addGenerator(Category.BODY, ROOT + name, new UuidGenerator(null));
    return uuid(name, "e2490de5-5bd3-43d5-b7c4-526e33f71304");
  }

  public FormPostBuilder uuid(String name, UUID uuid) {
    return uuid(name, uuid.toString());
  }

  public FormPostBuilder uuid(String name, String uuid) {
    if (!DslPart.UUID_REGEX.matcher(uuid).matches()) {
      throw new InvalidMatcherException("Example \"" + uuid + "\" is not an UUID");
    }
    return put(name, uuid, new RegexMatcher(DslPart.UUID_REGEX.pattern(), null), null);
  }

  public FormPostBuilder includesString(String name, String value) {
    return put(name, value, new IncludeMatcher(value), null);
  }

  public FormPostBuilder parameterFromProviderState(String name, String expression, String example) {
    return put(name, example, null, new ProviderStateGenerator(expression, DataType.STRING));
  }

  public FormPostBuilder dateExpression(String name, String expression) {
    return dateExpression(name, expression, ISO_DATE);
  }

  public FormPostBuilder dateExpression(String name, String expression, String format) {
    return put(name, Dates.local2000(format), new DateMatcher(format), new DateGenerator(format, expression));
  }

  public FormPostBuilder timeExpression(String name, String expression) {
    return timeExpression(name, expression, ISO_TIME_NO_T);
  }

  public FormPostBuilder timeExpression(String name, String expression, String format) {
    return put(name, Dates.local2000(format), new TimeMatcher(format), new TimeGenerator(format, expression));
  }

  public FormPostBuilder datetimeExpression(String name, String expression) {
    return datetimeExpression(name, expression, ISO_DATETIME);
  }

  public FormPostBuilder datetimeExpression(String name, String expression, String format) {
    return put(name, Dates.local2000(format), new TimestampMatcher(format), new DateTimeGenerator(format, expression));
  }

  @Override
  public MatchingRuleCategory getMatchers() {
    return matchers;
  }

  @Override
  public Generators getGenerators() {
    return generators;
  }

  @Override
  public String getContentType() {
    return contentType;
  }

  @Override
  public byte[] buildBody() {
    Charset charset = ContentTypes.charset(contentType);
    List<String> pairs = new ArrayList<>();
    body.forEach((name, values) -> values.forEach(value ->
        pairs.add(URLEncoder.encode(name, charset) + "=" + URLEncoder.encode(value, charset))));
    return String.join("&", pairs).getBytes(charset);
  }
}
