// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/src/main/kotlin/.../dsl/PactDslRootValue.kt; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import com.fasterxml.jackson.databind.JsonNode;
import io.pact.janus.facade.core.model.generators.Category;
import io.pact.janus.facade.core.model.generators.DateGenerator;
import io.pact.janus.facade.core.model.generators.DateTimeGenerator;
import io.pact.janus.facade.core.model.generators.MockServerURLGenerator;
import io.pact.janus.facade.core.model.generators.ProviderStateGenerator;
import io.pact.janus.facade.core.model.generators.RandomDecimalGenerator;
import io.pact.janus.facade.core.model.generators.RandomHexadecimalGenerator;
import io.pact.janus.facade.core.model.generators.RandomIntGenerator;
import io.pact.janus.facade.core.model.generators.RandomStringGenerator;
import io.pact.janus.facade.core.model.generators.TimeGenerator;
import io.pact.janus.facade.core.model.generators.UuidGenerator;
import io.pact.janus.facade.core.model.matchingrules.MatchingRule;
import io.pact.janus.facade.core.model.matchingrules.MatchingRuleGroup;
import io.pact.janus.facade.core.model.matchingrules.NumberTypeMatcher;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.model.matchingrules.RuleLogic;
import io.pact.janus.facade.core.model.matchingrules.TypeMatcher;
import io.pact.janus.facade.internal.Dates;
import io.pact.janus.facade.internal.RegexExample;
import io.pact.janus.facade.core.model.generators.RegexGenerator;
import java.math.BigDecimal;
import java.util.Date;
import java.util.List;
import java.util.UUID;

/** One value as a whole body, written as its text when it is a string: its example, rule and generator. */
public class PactDslRootValue extends DslPart {
  private static final String ISO_DATETIME = "yyyy-MM-dd'T'HH:mm:ss";
  private static final String ISO_DATE = "yyyy-MM-dd";
  private static final String ISO_TIME = "'T'HH:mm:ss";
  private static final String ISO_TIME_NO_T = "HH:mm:ss";

  private Object value;

  public PactDslRootValue() {
    super("", "");
  }

  /** The value as a body writes it: a string as its text, anything else as JSON. */
  public String getValue() {
    JsonNode body = getBody();
    return body.isTextual() ? body.asText() : body.toString();
  }

  public void setValue(Object value) {
    this.value = value;
  }

  @Override
  public String toString() {
    return getValue();
  }

  @Override
  public JsonNode getBody() {
    return toJson(value);
  }

  @Override
  public void setBody(JsonNode body) {
    value = body;
  }

  public void setMatcher(MatchingRule matcher) {
    matchers.addRule(matcher);
  }

  @Override
  protected void putObjectPrivate(DslPart obj) {
    throw new UnsupportedOperationException("Use PactDslJsonBody for objects");
  }

  @Override
  protected void putArrayPrivate(DslPart obj) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public DslPart close() {
    matchers.applyMatcherRootPrefix("$");
    generators.applyRootPrefix("$");
    return this;
  }

  @Override
  public DslPart matchUrl(String name, String basePath, Object... pathFragments) {
    throw new UnsupportedOperationException("matchUrl is not currently supported for PactDslRootValue");
  }

  @Override
  public DslPart matchUrl(String basePath, Object... pathFragments) {
    throw new UnsupportedOperationException("matchUrl is not currently supported for PactDslRootValue");
  }

  @Override
  public DslPart matchUrl2(String name, Object... pathFragments) {
    throw new UnsupportedOperationException("matchUrl2 is not currently supported for PactDslRootValue");
  }

  @Override
  public DslPart matchUrl2(Object... pathFragments) {
    throw new UnsupportedOperationException("matchUrl2 is not currently supported for PactDslRootValue");
  }

  @Override
  public PactDslJsonArray array(String name) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray array() {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray unorderedArray(String name) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray unorderedArray() {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray unorderedMinArray(String name, int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray unorderedMinArray(int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray unorderedMaxArray(String name, int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray unorderedMaxArray(int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray unorderedMinMaxArray(String name, int minSize, int maxSize) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray unorderedMinMaxArray(int minSize, int maxSize) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public DslPart closeArray() {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody eachLike(String name) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody eachLike(String name, DslPart obj) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody eachLike() {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachLike(DslPart obj) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody eachLike(String name, int numberExamples) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody eachLike(int numberExamples) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody minArrayLike(String name, int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody minArrayLike(int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody minArrayLike(String name, int size, DslPart obj) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray minArrayLike(int size, DslPart obj) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody minArrayLike(String name, int size, int numberExamples) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody minArrayLike(int size, int numberExamples) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody maxArrayLike(String name, int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody maxArrayLike(int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody maxArrayLike(String name, int size, DslPart obj) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray maxArrayLike(int size, DslPart obj) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody maxArrayLike(String name, int size, int numberExamples) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody maxArrayLike(int size, int numberExamples) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize, DslPart obj) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(int minSize, int maxSize) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray minMaxArrayLike(int minSize, int maxSize, DslPart obj) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize, int numberExamples) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(int minSize, int maxSize, int numberExamples) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayLike(String name) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayLike() {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayLike(String name, int numberExamples) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayLike(int numberExamples) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(String name, int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(String name, int numberExamples, int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(int numberExamples, int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(String name, int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(String name, int numberExamples, int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(int numberExamples, int size) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(String name, int minSize, int maxSize) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(int minSize, int maxSize) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(String name, int numberExamples, int minSize, int maxSize) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(int numberExamples, int minSize, int maxSize) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  @Override
  public PactDslJsonBody object(String name) {
    throw new UnsupportedOperationException("Use PactDslJsonBody for objects");
  }

  @Override
  public PactDslJsonBody object() {
    throw new UnsupportedOperationException("Use PactDslJsonBody for objects");
  }

  @Override
  public DslPart closeObject() {
    throw new UnsupportedOperationException("Use PactDslJsonBody for objects");
  }

  @Override
  public DslPart arrayContaining(String name) {
    throw new UnsupportedOperationException("Use PactDslJsonArray for arrays");
  }

  // ----------------------------------------------------------------------------------------------

  private static PactDslRootValue of(Object example, MatchingRule rule) {
    PactDslRootValue value = new PactDslRootValue();
    value.value = example;
    if (rule != null) {
      value.setMatcher(rule);
    }
    return value;
  }

  private PactDslRootValue generated(io.pact.janus.facade.core.model.generators.Generator generator) {
    generators.addGenerator(Category.BODY, "", generator);
    return this;
  }

  public static PactDslRootValue stringType() {
    return of("string", TypeMatcher.INSTANCE).generated(new RandomStringGenerator(20));
  }

  public static PactDslRootValue stringType(String example) {
    return of(example, TypeMatcher.INSTANCE);
  }

  public static PactDslRootValue numberType() {
    return of(100, TypeMatcher.INSTANCE).generated(new RandomIntGenerator(0, Integer.MAX_VALUE));
  }

  public static PactDslRootValue numberType(Number number) {
    return of(number, TypeMatcher.INSTANCE);
  }

  public static PactDslRootValue integerType() {
    return of(100, new NumberTypeMatcher(NumberTypeMatcher.NumberType.INTEGER))
        .generated(new RandomIntGenerator(0, Integer.MAX_VALUE));
  }

  public static PactDslRootValue integerType(Long number) {
    return of(number, new NumberTypeMatcher(NumberTypeMatcher.NumberType.INTEGER));
  }

  public static PactDslRootValue integerType(Integer number) {
    return of(number, new NumberTypeMatcher(NumberTypeMatcher.NumberType.INTEGER));
  }

  public static PactDslRootValue decimalType() {
    return of(100, new NumberTypeMatcher(NumberTypeMatcher.NumberType.DECIMAL)).generated(new RandomDecimalGenerator(10));
  }

  public static PactDslRootValue decimalType(BigDecimal number) {
    return of(number, new NumberTypeMatcher(NumberTypeMatcher.NumberType.DECIMAL));
  }

  public static PactDslRootValue decimalType(Double number) {
    return of(number, new NumberTypeMatcher(NumberTypeMatcher.NumberType.DECIMAL));
  }

  private static PactDslRootValue numberMatching(String regex, Number example, NumberTypeMatcher.NumberType type) {
    if (!example.toString().matches(regex)) {
      throw new IllegalArgumentException(
          "Example value " + example + " does not match the provided regular expression '" + regex + "'");
    }
    PactDslRootValue value = of(example, null);
    value.matchers.addRules("", List.of(new NumberTypeMatcher(type), new RegexMatcher(regex, example.toString())));
    return value;
  }

  public static PactDslRootValue numberMatching(String regex, Number example) {
    return numberMatching(regex, example, NumberTypeMatcher.NumberType.NUMBER);
  }

  public static PactDslRootValue decimalMatching(String regex, Double example) {
    return numberMatching(regex, example, NumberTypeMatcher.NumberType.DECIMAL);
  }

  public static PactDslRootValue integerMatching(String regex, Integer example) {
    return numberMatching(regex, example, NumberTypeMatcher.NumberType.INTEGER);
  }

  public static PactDslRootValue booleanType() {
    return booleanType(true);
  }

  public static PactDslRootValue booleanType(Boolean example) {
    return of(example, TypeMatcher.INSTANCE);
  }

  public static PactDslRootValue stringMatcher(String regex, String value) {
    if (!value.matches(regex)) {
      throw new InvalidMatcherException("Example \"" + value + "\" does not match regular expression \"" + regex + "\"");
    }
    return of(value, new RegexMatcher(regex, value));
  }

  /** A value matching {@code regex}, its example drawn from the pattern the same way every run (ADR 0070 decision 3). */
  public static PactDslRootValue stringMatcher(String regex) {
    return of(RegexExample.of(regex), new RegexMatcher(regex, null)).generated(new RegexGenerator(regex));
  }

  private static PactDslRootValue dated(String format, MatchingRule rule,
      io.pact.janus.facade.core.model.generators.Generator generator) {
    return of(Dates.local2000(format), rule).generated(generator);
  }

  public static PactDslRootValue timestamp() {
    return timestamp(ISO_DATETIME);
  }

  public static PactDslRootValue timestamp(String format) {
    return dated(format, new io.pact.janus.facade.core.model.matchingrules.TimestampMatcher(format),
        new DateTimeGenerator(format, null));
  }

  public static PactDslRootValue timestamp(String format, Date example) {
    return of(Dates.format(format, java.util.TimeZone.getDefault(), example),
        new io.pact.janus.facade.core.model.matchingrules.TimestampMatcher(format));
  }

  public static PactDslRootValue datetime() {
    return datetime(ISO_DATETIME);
  }

  public static PactDslRootValue datetime(String format) {
    return timestamp(format);
  }

  public static PactDslRootValue datetime(String format, Date example) {
    return timestamp(format, example);
  }

  public static PactDslRootValue date() {
    return date(ISO_DATE);
  }

  public static PactDslRootValue date(String format) {
    return dated(format, new io.pact.janus.facade.core.model.matchingrules.DateMatcher(format),
        new DateGenerator(format, null));
  }

  public static PactDslRootValue date(String format, Date example) {
    return of(Dates.format(format, java.util.TimeZone.getDefault(), example),
        new io.pact.janus.facade.core.model.matchingrules.DateMatcher(format));
  }

  public static PactDslRootValue time() {
    return time(ISO_TIME);
  }

  public static PactDslRootValue time(String format) {
    return dated(format, new io.pact.janus.facade.core.model.matchingrules.TimeMatcher(format),
        new TimeGenerator(format, null));
  }

  public static PactDslRootValue time(String format, Date example) {
    return of(Dates.format(format, java.util.TimeZone.getDefault(), example),
        new io.pact.janus.facade.core.model.matchingrules.TimeMatcher(format));
  }

  public static PactDslRootValue ipAddress() {
    return of("127.0.0.1", new RegexMatcher("(\\d{1,3}\\.)+\\d{1,3}", null));
  }

  public static PactDslRootValue id() {
    return numberType();
  }

  public static PactDslRootValue id(Long id) {
    return numberType(id);
  }

  public static PactDslRootValue hexValue() {
    return of("1234a", new RegexMatcher("[0-9a-fA-F]+", null)).generated(new RandomHexadecimalGenerator(10));
  }

  public static PactDslRootValue hexValue(String hexValue) {
    if (!HEXADECIMAL.matcher(hexValue).matches()) {
      throw new InvalidMatcherException("Example \"" + hexValue + "\" is not a hexadecimal value");
    }
    return of(hexValue, new RegexMatcher("[0-9a-fA-F]+", null));
  }

  public static PactDslRootValue uuid() {
    return of("e2490de5-5bd3-43d5-b7c4-526e33f71304", new RegexMatcher(UUID_REGEX.pattern(), null))
        .generated(new UuidGenerator(null));
  }

  public static PactDslRootValue uuid(UUID uuid) {
    return uuid(uuid.toString());
  }

  public static PactDslRootValue uuid(String uuid) {
    if (!UUID_REGEX.matcher(uuid).matches()) {
      throw new InvalidMatcherException("Example \"" + uuid + "\" is not an UUID");
    }
    return of(uuid, new RegexMatcher(UUID_REGEX.pattern(), null));
  }

  public static PactDslRootValue and(Object example, MatchingRule... rules) {
    PactDslRootValue value = of(example, null);
    value.matchers.setRules("", new MatchingRuleGroup(List.of(rules), RuleLogic.AND));
    return value;
  }

  public static PactDslRootValue or(Object example, MatchingRule... rules) {
    PactDslRootValue value = of(example, null);
    value.matchers.setRules("", new MatchingRuleGroup(List.of(rules), RuleLogic.OR));
    return value;
  }

  public static PactDslRootValue valueFromProviderState(String expression, Object example) {
    return of(example, TypeMatcher.INSTANCE).generated(new ProviderStateGenerator(expression, dataType(example)));
  }

  public static PactDslRootValue dateExpression(String expression) {
    return dateExpression(expression, ISO_DATE);
  }

  public static PactDslRootValue dateExpression(String expression, String format) {
    return dated(format, new io.pact.janus.facade.core.model.matchingrules.DateMatcher(format),
        new DateGenerator(format, expression));
  }

  public static PactDslRootValue timeExpression(String expression) {
    return timeExpression(expression, ISO_TIME_NO_T);
  }

  public static PactDslRootValue timeExpression(String expression, String format) {
    return dated(format, new io.pact.janus.facade.core.model.matchingrules.TimeMatcher(format),
        new TimeGenerator(format, expression));
  }

  public static PactDslRootValue datetimeExpression(String expression) {
    return datetimeExpression(expression, ISO_DATETIME);
  }

  public static PactDslRootValue datetimeExpression(String expression, String format) {
    return dated(format, new io.pact.janus.facade.core.model.matchingrules.TimestampMatcher(format),
        new DateTimeGenerator(format, expression));
  }
}
