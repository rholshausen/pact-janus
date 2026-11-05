// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/src/main/kotlin/.../dsl/PactDslJsonBody.kt; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
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
import io.pact.janus.facade.core.model.matchingrules.EachKeyMatcher;
import io.pact.janus.facade.core.model.matchingrules.EqualsIgnoreOrderMatcher;
import io.pact.janus.facade.core.model.matchingrules.EqualsMatcher;
import io.pact.janus.facade.core.model.matchingrules.MatchingRule;
import io.pact.janus.facade.core.model.matchingrules.MatchingRuleGroup;
import io.pact.janus.facade.core.model.matchingrules.MaxEqualsIgnoreOrderMatcher;
import io.pact.janus.facade.core.model.matchingrules.MinEqualsIgnoreOrderMatcher;
import io.pact.janus.facade.core.model.matchingrules.MinMaxEqualsIgnoreOrderMatcher;
import io.pact.janus.facade.core.model.matchingrules.NumberTypeMatcher;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.model.matchingrules.RuleLogic;
import io.pact.janus.facade.core.model.matchingrules.TypeMatcher;
import io.pact.janus.facade.core.model.matchingrules.ValuesMatcher;
import io.pact.janus.facade.internal.Dates;
import io.pact.janus.facade.internal.Json;
import io.pact.janus.facade.internal.RegexExample;
import io.pact.janus.facade.core.model.generators.RegexGenerator;
import java.math.BigDecimal;
import java.time.Instant;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Date;
import java.util.List;
import java.util.TimeZone;
import java.util.UUID;
import java.util.function.Function;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** A JSON object in a body: its members' examples, and the rules and generators at them. */
public class PactDslJsonBody extends DslPart {
  private static final String ISO_DATETIME = "yyyy-MM-dd'T'HH:mm:ss";
  private static final String ISO_DATE = "yyyy-MM-dd";
  private static final String ISO_TIME = "'T'HH:mm:ss";
  private static final String ISO_TIME_NO_T = "HH:mm:ss";
  private static final Pattern QUOTED = Pattern.compile("\\['(.+)'\\]");

  private JsonNode body;

  public PactDslJsonBody() {
    super(".", "");
    body = Json.NODES.objectNode();
  }

  public PactDslJsonBody(String rootPath, String rootName, DslPart parent) {
    super(parent, rootPath, rootName);
    body = Json.NODES.objectNode();
  }

  public PactDslJsonBody(String rootPath, String rootName, DslPart parent, PactDslJsonBody body) {
    super(parent, rootPath, rootName);
    this.body = body.body;
    matchers = body.matchers.copyWithUpdatedMatcherRootPrefix(rootPath);
    generators = body.generators.copyWithUpdatedMatcherRootPrefix(rootPath);
  }

  /** The template of {@code eachLike(n)}: n objects, each member set on each of them. */
  public PactDslJsonBody(String rootPath, String rootName, PactDslJsonArray parent, int examples) {
    super(parent, rootPath, rootName);
    ArrayNode array = Json.NODES.arrayNode();
    for (int i = 0; i < examples; i++) {
      array.add(Json.NODES.objectNode());
    }
    body = array;
  }

  @Override
  public JsonNode getBody() {
    return body;
  }

  @Override
  public void setBody(JsonNode body) {
    this.body = body;
  }

  @Override
  protected void putObjectPrivate(DslPart obj) {
    obj.matchers.getMatchingRules().forEach((name, group) -> matchers.setRules(name, group));
    generators.addGenerators(obj.generators);
    String elementBase = difference(rootPath, obj.rootPath);
    String name;
    if (!obj.rootName.isEmpty()) {
      name = obj.rootName;
    } else {
      String stripped = strip(elementBase, ".");
      Matcher quoted = QUOTED.matcher(stripped);
      name = quoted.matches() ? quoted.group(1) : stripped;
    }
    putEach(name, obj.getBody());
  }

  @Override
  protected void putArrayPrivate(DslPart obj) {
    obj.matchers.getMatchingRules().forEach((name, group) -> matchers.setRules(name, group));
    generators.addGenerators(obj.generators);
    putEach(obj.rootName.isEmpty() ? difference(rootPath, obj.rootPath) : obj.rootName, obj.getBody());
  }

  // ----------------------------------------------------------------------------------------------
  // How a member's examples land: one on an object, or one on each of a template's objects.

  private void putEach(String name, JsonNode value) {
    if (body instanceof ObjectNode object) {
      object.set(name, value);
    } else if (body instanceof ArrayNode array) {
      array.forEach(element -> ((ObjectNode) element).set(name, value));
    }
  }

  private <T> PactDslJsonBody putExamples(String name, T[] examples, Function<T, JsonNode> json) {
    if (examples.length == 0) {
      throw new IllegalArgumentException("At least one example value is required");
    }
    if (body instanceof ObjectNode object) {
      if (examples.length != 1) {
        throw new IllegalArgumentException(
            "You provided multiple example values (" + examples.length + ") but only one was expected");
      }
      object.set(name, json.apply(examples[0]));
    } else if (body instanceof ArrayNode array) {
      if (array.size() < examples.length) {
        throw new IllegalArgumentException(
            "You provided " + examples.length + " example values but " + array.size() + " was expected");
      }
      List<T> padded = padTo(Arrays.asList(examples), array.size());
      for (int i = 0; i < padded.size(); i++) {
        ((ObjectNode) array.get(i)).set(name, json.apply(padded.get(i)));
      }
    }
    return this;
  }

  private static <T> void requireNoNulls(T[] values, String message) {
    if (Arrays.stream(values).anyMatch(java.util.Objects::isNull)) {
      throw new IllegalArgumentException(message);
    }
  }

  private String[] examples(String example) {
    int size = body instanceof ArrayNode array ? array.size() : 1;
    String[] examples = new String[size];
    Arrays.fill(examples, example);
    return examples;
  }

  private String path(String name) {
    return Json.constructValidPath(name, rootPath);
  }

  private String pathByName(String name) {
    return Json.constructValidPath(name, rootPath, false);
  }

  private String rootWithoutDot() {
    return rootPath.endsWith(".") ? rootPath.substring(0, rootPath.length() - 1) : rootPath;
  }

  // ----------------------------------------------------------------------------------------------
  // Values

  public PactDslJsonBody stringValue(String name, String... values) {
    return putExamples(name, values, DslPart::jsonString);
  }

  public PactDslJsonBody numberValue(String name, Number... values) {
    return putExamples(name, values, DslPart::jsonNumber);
  }

  public PactDslJsonBody booleanValue(String name, Boolean... values) {
    requireNoNulls(values, "Example values can not be null");
    return putExamples(name, values, DslPart::jsonBool);
  }

  public PactDslJsonBody like(String name, Object... examples) {
    putExamples(name, examples, DslPart::toJson);
    matchers.addRule(path(name), TypeMatcher.INSTANCE);
    return this;
  }

  public PactDslJsonBody stringType(String name) {
    generators.addGenerator(Category.BODY, path(name), new RandomStringGenerator(20));
    return stringType(name, examples("string"));
  }

  public PactDslJsonBody stringTypes(String... names) {
    requireNoNulls(names, "Attribute names can not be null");
    for (String name : names) {
      stringType(name);
    }
    return this;
  }

  public PactDslJsonBody stringType(String name, String... examples) {
    requireNoNulls(examples, "Example values can not be null");
    putExamples(name, examples, DslPart::jsonString);
    matchers.addRule(path(name), TypeMatcher.INSTANCE);
    return this;
  }

  public PactDslJsonBody numberType(String name) {
    generators.addGenerator(Category.BODY, path(name), new RandomIntGenerator(0, Integer.MAX_VALUE));
    return numberType(name, 100);
  }

  public PactDslJsonBody numberTypes(String... names) {
    requireNoNulls(names, "Attribute names can not be null");
    for (String name : names) {
      numberType(name);
    }
    return this;
  }

  public PactDslJsonBody numberType(String name, Number... numbers) {
    requireNoNulls(numbers, "Example values can not be null");
    putExamples(name, numbers, DslPart::jsonNumber);
    matchers.addRule(path(name), new NumberTypeMatcher(NumberTypeMatcher.NumberType.NUMBER));
    return this;
  }

  public PactDslJsonBody integerType(String name) {
    generators.addGenerator(Category.BODY, path(name), new RandomIntGenerator(0, Integer.MAX_VALUE));
    return integerType(name, 100);
  }

  public PactDslJsonBody integerTypes(String... names) {
    requireNoNulls(names, "Attribute names can not be null");
    for (String name : names) {
      integerType(name);
    }
    return this;
  }

  public PactDslJsonBody integerType(String name, Long... numbers) {
    requireNoNulls(numbers, "Example values can not be null");
    putExamples(name, numbers, DslPart::jsonNumber);
    matchers.addRule(path(name), new NumberTypeMatcher(NumberTypeMatcher.NumberType.INTEGER));
    return this;
  }

  public PactDslJsonBody integerType(String name, Integer... numbers) {
    requireNoNulls(numbers, "Example values can not be null");
    putExamples(name, numbers, DslPart::jsonNumber);
    matchers.addRule(path(name), new NumberTypeMatcher(NumberTypeMatcher.NumberType.INTEGER));
    return this;
  }

  public PactDslJsonBody decimalType(String name) {
    generators.addGenerator(Category.BODY, path(name), new RandomDecimalGenerator(10));
    return decimalType(name, 100.0);
  }

  public PactDslJsonBody decimalTypes(String... names) {
    requireNoNulls(names, "Attribute names can not be null");
    for (String name : names) {
      decimalType(name);
    }
    return this;
  }

  public PactDslJsonBody decimalType(String name, BigDecimal... numbers) {
    requireNoNulls(numbers, "Example values can not be null");
    putExamples(name, numbers, DslPart::jsonNumber);
    matchers.addRule(path(name), new NumberTypeMatcher(NumberTypeMatcher.NumberType.DECIMAL));
    return this;
  }

  public PactDslJsonBody decimalType(String name, Double... numbers) {
    requireNoNulls(numbers, "Example values can not be null");
    putExamples(name, numbers, DslPart::jsonNumber);
    matchers.addRule(path(name), new NumberTypeMatcher(NumberTypeMatcher.NumberType.DECIMAL));
    return this;
  }

  private PactDslJsonBody numberMatching(String name, String regex, Number example, NumberTypeMatcher.NumberType type) {
    if (!example.toString().matches(regex)) {
      throw new IllegalArgumentException(
          "Example value " + example + " does not match the provided regular expression '" + regex + "'");
    }
    putEach(name, jsonNumber(example));
    matchers.addRules(pathByName(name), List.of(new NumberTypeMatcher(type), new RegexMatcher(regex, example.toString())));
    return this;
  }

  public PactDslJsonBody numberMatching(String name, String regex, Number example) {
    return numberMatching(name, regex, example, NumberTypeMatcher.NumberType.NUMBER);
  }

  public PactDslJsonBody decimalMatching(String name, String regex, Double example) {
    return numberMatching(name, regex, example, NumberTypeMatcher.NumberType.DECIMAL);
  }

  public PactDslJsonBody integerMatching(String name, String regex, Integer example) {
    return numberMatching(name, regex, example, NumberTypeMatcher.NumberType.INTEGER);
  }

  public PactDslJsonBody booleanTypes(String... names) {
    requireNoNulls(names, "Attribute names can not be null");
    for (String name : names) {
      booleanType(name);
    }
    return this;
  }

  public PactDslJsonBody booleanType(String name) {
    return booleanType(name, true);
  }

  public PactDslJsonBody booleanType(String name, Boolean... examples) {
    requireNoNulls(examples, "Example values can not be null");
    putExamples(name, examples, DslPart::jsonBool);
    matchers.addRule(path(name), TypeMatcher.INSTANCE);
    return this;
  }

  public PactDslJsonBody stringMatcher(String name, String regex, String... values) {
    Pattern re = Pattern.compile(regex);
    putExamples(name, values, value -> {
      if (value == null || !re.matcher(value).matches()) {
        throw new InvalidMatcherException("Example \"" + value + "\" does not match regular expression \"" + regex + "\"");
      }
      return jsonString(value);
    });
    matchers.addRule(path(name), regexp(regex));
    return this;
  }

  /** A value matching {@code regex}, its example drawn from the pattern the same way every run (ADR 0070 decision 3). */
  public PactDslJsonBody stringMatcher(String name, String regex) {
    generators.addGenerator(Category.BODY, path(name), new RegexGenerator(regex));
    return stringMatcher(name, regex, examples(RegexExample.of(regex)));
  }

  // ----------------------------------------------------------------------------------------------
  // Dates and times

  public PactDslJsonBody datetime(String name) {
    generators.addGenerator(Category.BODY, path(name), new DateTimeGenerator(ISO_DATETIME, null));
    matchers.addRule(path(name), matchTimestamp(ISO_DATETIME));
    putEach(name, jsonString(Dates.local2000(ISO_DATETIME)));
    return this;
  }

  public PactDslJsonBody datetime(String name, String format) {
    String path = pathByName(name);
    generators.addGenerator(Category.BODY, path, new DateTimeGenerator(format, null));
    matchers.addRule(path, matchTimestamp(format));
    putEach(name, jsonString(Dates.localTime2000(format)));
    return this;
  }

  public PactDslJsonBody datetime(String name, String format, Date example) {
    return datetime(name, format, example, TimeZone.getDefault());
  }

  public PactDslJsonBody datetime(String name, String format, Date example, TimeZone timeZone) {
    return datetime(name, format, timeZone, example);
  }

  public PactDslJsonBody datetime(String name, String format, Date... examples) {
    return datetime(name, format, TimeZone.getDefault(), examples);
  }

  public PactDslJsonBody datetime(String name, String format, TimeZone timeZone, Date... examples) {
    DateTimeFormatter formatter = DateTimeFormatter.ofPattern(format).withZone(timeZone.toZoneId());
    matchers.addRule(pathByName(name), matchTimestamp(format));
    return putExamples(name, examples, example -> jsonString(formatter.format(example.toInstant())));
  }

  public PactDslJsonBody datetime(String name, String format, Instant example) {
    return datetime(name, format, example, TimeZone.getDefault());
  }

  public PactDslJsonBody datetime(String name, String format, Instant example, TimeZone timeZone) {
    return datetime(name, format, timeZone, example);
  }

  public PactDslJsonBody datetime(String name, String format, Instant... examples) {
    return datetime(name, format, TimeZone.getDefault(), examples);
  }

  public PactDslJsonBody datetime(String name, String format, TimeZone timeZone, Instant... examples) {
    DateTimeFormatter formatter = DateTimeFormatter.ofPattern(format).withZone(timeZone.toZoneId());
    matchers.addRule(pathByName(name), matchTimestamp(format));
    return putExamples(name, examples, example -> jsonString(formatter.format(example)));
  }

  public PactDslJsonBody date() {
    return date("date");
  }

  public PactDslJsonBody date(String name) {
    String path = pathByName(name);
    generators.addGenerator(Category.BODY, path, new DateGenerator(ISO_DATE, null));
    matchers.addRule(path, matchDate(ISO_DATE));
    putEach(name, jsonString(Dates.local2000(ISO_DATE)));
    return this;
  }

  public PactDslJsonBody date(String name, String format) {
    String path = pathByName(name);
    generators.addGenerator(Category.BODY, path, new DateGenerator(format, null));
    matchers.addRule(path, matchDate(format));
    putEach(name, jsonString(Dates.local2000(format)));
    return this;
  }

  public PactDslJsonBody date(String name, String format, Date example) {
    return date(name, format, example, TimeZone.getDefault());
  }

  public PactDslJsonBody date(String name, String format, Date example, TimeZone timeZone) {
    return date(name, format, timeZone, example);
  }

  public PactDslJsonBody date(String name, String format, Date... examples) {
    return date(name, format, TimeZone.getDefault(), examples);
  }

  public PactDslJsonBody date(String name, String format, TimeZone timeZone, Date... examples) {
    matchers.addRule(pathByName(name), matchDate(format));
    return putExamples(name, examples, example -> jsonString(Dates.format(format, timeZone, example)));
  }

  public PactDslJsonBody localDate(String name, String format, LocalDate... examples) {
    DateTimeFormatter formatter = DateTimeFormatter.ofPattern(format);
    matchers.addRule(pathByName(name), matchDate(format));
    return putExamples(name, examples, example -> jsonString(formatter.format(example)));
  }

  public PactDslJsonBody time() {
    return time("time");
  }

  public PactDslJsonBody time(String name) {
    String path = pathByName(name);
    generators.addGenerator(Category.BODY, path, new TimeGenerator(ISO_TIME, null));
    matchers.addRule(path, matchTime(ISO_TIME));
    putEach(name, jsonString(Dates.local2000(ISO_TIME)));
    return this;
  }

  public PactDslJsonBody time(String name, String format) {
    String path = pathByName(name);
    generators.addGenerator(Category.BODY, path, new TimeGenerator(format, null));
    matchers.addRule(path, matchTime(format));
    putEach(name, jsonString(Dates.local2000(format)));
    return this;
  }

  public PactDslJsonBody time(String name, String format, Date example) {
    return time(name, format, example, TimeZone.getDefault());
  }

  public PactDslJsonBody time(String name, String format, Date example, TimeZone timeZone) {
    return time(name, format, timeZone, example);
  }

  public PactDslJsonBody time(String name, String format, Date... examples) {
    return time(name, format, TimeZone.getDefault(), examples);
  }

  public PactDslJsonBody time(String name, String format, TimeZone timeZone, Date... examples) {
    matchers.addRule(pathByName(name), matchTime(format));
    return putExamples(name, examples, example -> jsonString(Dates.format(format, timeZone, example)));
  }

  public PactDslJsonBody dateExpression(String name, String expression) {
    return dateExpression(name, expression, ISO_DATE);
  }

  public PactDslJsonBody dateExpression(String name, String expression, String format) {
    generators.addGenerator(Category.BODY, path(name), new DateGenerator(format, expression));
    putEach(name, jsonString(Dates.local2000(format)));
    matchers.addRule(path(name), matchDate(format));
    return this;
  }

  public PactDslJsonBody timeExpression(String name, String expression) {
    return timeExpression(name, expression, ISO_TIME_NO_T);
  }

  public PactDslJsonBody timeExpression(String name, String expression, String format) {
    generators.addGenerator(Category.BODY, path(name), new TimeGenerator(format, expression));
    putEach(name, jsonString(Dates.local2000(format)));
    matchers.addRule(path(name), matchTime(format));
    return this;
  }

  public PactDslJsonBody datetimeExpression(String name, String expression) {
    return datetimeExpression(name, expression, ISO_DATETIME);
  }

  public PactDslJsonBody datetimeExpression(String name, String expression, String format) {
    generators.addGenerator(Category.BODY, path(name), new DateTimeGenerator(format, expression));
    putEach(name, jsonString(Dates.local2000(format)));
    matchers.addRule(path(name), matchTimestamp(format));
    return this;
  }

  // ----------------------------------------------------------------------------------------------
  // Formats

  public PactDslJsonBody ipAddress(String name) {
    matchers.addRule(pathByName(name), regexp("(\\d{1,3}\\.)+\\d{1,3}"));
    putEach(name, jsonString("127.0.0.1"));
    return this;
  }

  public PactDslJsonBody id() {
    return id("id");
  }

  public PactDslJsonBody id(String name) {
    String path = path(name);
    generators.addGenerator(Category.BODY, path, new RandomIntGenerator(0, Integer.MAX_VALUE));
    matchers.addRule(path, TypeMatcher.INSTANCE);
    putEach(name, jsonNumber(1234567890L));
    return this;
  }

  public PactDslJsonBody id(String name, Long... examples) {
    putExamples(name, examples, DslPart::jsonNumber);
    matchers.addRule(path(name), TypeMatcher.INSTANCE);
    return this;
  }

  public PactDslJsonBody hexValue(String name) {
    generators.addGenerator(Category.BODY, path(name), new RandomHexadecimalGenerator(10));
    return hexValue(name, "1234a");
  }

  public PactDslJsonBody hexValue(String name, String... examples) {
    putExamples(name, examples, example -> {
      if (!HEXADECIMAL.matcher(example).matches()) {
        throw new InvalidMatcherException("Example \"" + example + "\" is not a hexadecimal value");
      }
      return jsonString(example);
    });
    matchers.addRule(path(name), regexp("[0-9a-fA-F]+"));
    return this;
  }

  public PactDslJsonBody uuid(String name) {
    generators.addGenerator(Category.BODY, path(name), new UuidGenerator(null));
    return uuid(name, "e2490de5-5bd3-43d5-b7c4-526e33f71304");
  }

  public PactDslJsonBody uuid(String name, UUID... uuids) {
    return uuid(name, Arrays.stream(uuids).map(UUID::toString).toArray(String[]::new));
  }

  public PactDslJsonBody uuid(String name, String... examples) {
    putExamples(name, examples, example -> {
      if (!UUID_REGEX.matcher(example).matches()) {
        throw new InvalidMatcherException("Example \"" + example + "\" is not an UUID");
      }
      return jsonString(example);
    });
    matchers.addRule(path(name), regexp(UUID_REGEX.pattern()));
    return this;
  }

  public PactDslJsonBody nullValue(String fieldName) {
    putEach(fieldName, Json.NODES.nullNode());
    return this;
  }

  public PactDslJsonBody includesStr(String name, String value) {
    putEach(name, jsonString(value));
    matchers.addRule(path(name), includesMatcher(value));
    return this;
  }

  public PactDslJsonBody equalTo(String name, Object... examples) {
    putExamples(name, examples, DslPart::toJson);
    matchers.addRule(path(name), EqualsMatcher.INSTANCE);
    return this;
  }

  public PactDslJsonBody and(String name, Object value, MatchingRule... rules) {
    putEach(name, toJson(value));
    matchers.setRules(path(name), new MatchingRuleGroup(List.of(rules), RuleLogic.AND));
    return this;
  }

  public PactDslJsonBody or(String name, Object value, MatchingRule... rules) {
    putEach(name, toJson(value));
    matchers.setRules(path(name), new MatchingRuleGroup(List.of(rules), RuleLogic.OR));
    return this;
  }

  public PactDslJsonBody valueFromProviderState(String name, String expression, Object example) {
    generators.addGenerator(Category.BODY, path(name), new ProviderStateGenerator(expression, dataType(example)));
    putEach(name, toJson(example));
    matchers.addRule(path(name), TypeMatcher.INSTANCE);
    return this;
  }

  @Override
  public PactDslJsonBody matchUrl(String name, String basePath, Object... pathFragments) {
    String[] url = urlMatcher(basePath, pathFragments);
    putEach(name, jsonString(url[0]));
    matchers.addRule(path(name), regexp(url[1]));
    if (basePath == null || basePath.isEmpty()) {
      generators.addGenerator(Category.BODY, path(name), new MockServerURLGenerator(url[0], url[1]));
    }
    return this;
  }

  @Override
  public DslPart matchUrl(String basePath, Object... pathFragments) {
    throw new UnsupportedOperationException("URL matcher without an attribute name is not supported for objects. "
        + "Use matchUrl(String name, String basePath, Object... pathFragments)");
  }

  @Override
  public PactDslJsonBody matchUrl2(String name, Object... pathFragments) {
    return matchUrl(name, null, pathFragments);
  }

  @Override
  public DslPart matchUrl2(Object... pathFragments) {
    throw new UnsupportedOperationException("URL matcher without an attribute name is not supported for objects. "
        + "Use matchUrl2(Object... pathFragments)");
  }

  // ----------------------------------------------------------------------------------------------
  // Objects

  @Override
  public PactDslJsonBody object(String name) {
    return new PactDslJsonBody(pathByName(name) + ".", name, this);
  }

  @Override
  public PactDslJsonBody object() {
    throw new UnsupportedOperationException("use the object(String name) form");
  }

  public PactDslJsonBody object(String name, DslPart value) {
    String base = pathByName(name);
    if (value instanceof PactDslJsonBody body) {
      putObjectPrivate(new PactDslJsonBody(base, name, this, body));
    } else if (value instanceof PactDslJsonArray array) {
      putArrayPrivate(new PactDslJsonArray(base, name, this, array));
    }
    return this;
  }

  @Override
  public DslPart closeObject() {
    if (parent != null) {
      parent.putObjectPrivate(this);
    } else {
      matchers.applyMatcherRootPrefix("$");
      generators.applyRootPrefix("$");
    }
    closed = true;
    return parent;
  }

  @Override
  public DslPart close() {
    DslPart parentToReturn = this;
    if (!closed) {
      DslPart next = closeObject();
      while (next != null) {
        parentToReturn = next;
        next = next instanceof PactDslJsonArray ? next.closeArray() : next.closeObject();
      }
    }
    return parentToReturn;
  }

  @Override
  public DslPart closeArray() {
    if (parent instanceof PactDslJsonArray) {
      closeObject();
      return parent.closeArray();
    }
    throw new UnsupportedOperationException("can't call closeArray on an Object");
  }

  public void extendFrom(PactDslJsonBody baseTemplate) {
    body = baseTemplate.body.deepCopy();
    matchers = baseTemplate.matchers.copyWithUpdatedMatcherRootPrefix("");
    generators = baseTemplate.generators.copyWithUpdatedMatcherRootPrefix("");
  }

  // ----------------------------------------------------------------------------------------------
  // Maps

  public PactDslJsonBody eachKeyMappedToAnArrayLike(String exampleKey) {
    matchers.addRule(rootWithoutDot(), ValuesMatcher.INSTANCE);
    PactDslJsonArray parent = new PactDslJsonArray(rootPath + "*", exampleKey, this, true);
    return new PactDslJsonBody(".", "", parent);
  }

  public PactDslJsonBody eachKeyLike(String exampleKey) {
    return eachValueLike(exampleKey);
  }

  public PactDslJsonBody eachKeyLike(String exampleKey, PactDslJsonRootValue value) {
    return eachValueLike(exampleKey, value);
  }

  public PactDslJsonBody eachValueLike(String exampleKey) {
    matchers.addRule(rootWithoutDot(), ValuesMatcher.INSTANCE);
    return new PactDslJsonBody(rootPath + "*.", exampleKey, this);
  }

  public PactDslJsonBody eachValueLike(String exampleKey, PactDslJsonRootValue value) {
    putEach(exampleKey, value.getBody());
    matchers.addRule(rootWithoutDot(), ValuesMatcher.INSTANCE);
    value.matchers.getMatchingRules().forEach((name, group) -> matchers.addRules(rootPath + "*" + name, group.getRules()));
    return this;
  }

  public PactDslJsonBody eachKeyMatching(io.pact.janus.facade.consumer.dsl.Matcher matcher) {
    String path = rootWithoutDot();
    String value = String.valueOf(matcher.getValue());
    if (matcher.getMatcher() != null) {
      matchers.addRule(path, new EachKeyMatcher(List.of(matcher.getMatcher()), value));
    }
    if (body instanceof ObjectNode object && !object.has(value)) {
      object.set(value, Json.NODES.nullNode());
    }
    return this;
  }

  public PactDslJsonBody eachValueMatching(String exampleKey) {
    return new PactDslJsonBody(Json.constructValidPath("*", rootPath) + ".", exampleKey, this);
  }

  // ----------------------------------------------------------------------------------------------
  // Arrays

  @Override
  public PactDslJsonArray array(String name) {
    return new PactDslJsonArray(path(name), name, this);
  }

  @Override
  public PactDslJsonArray array() {
    throw new UnsupportedOperationException("use the array(String name) form");
  }

  @Override
  public PactDslJsonArray unorderedArray(String name) {
    matchers.addRule(path(name), EqualsIgnoreOrderMatcher.INSTANCE);
    return array(name);
  }

  @Override
  public PactDslJsonArray unorderedArray() {
    throw new UnsupportedOperationException("use the unorderedArray(String name) form");
  }

  @Override
  public PactDslJsonArray unorderedMinArray(String name, int size) {
    matchers.addRule(path(name), new MinEqualsIgnoreOrderMatcher(size));
    return array(name);
  }

  @Override
  public PactDslJsonArray unorderedMinArray(int size) {
    throw new UnsupportedOperationException("use the unorderedMinArray(String name, int size) form");
  }

  @Override
  public PactDslJsonArray unorderedMaxArray(String name, int size) {
    matchers.addRule(path(name), new MaxEqualsIgnoreOrderMatcher(size));
    return array(name);
  }

  @Override
  public PactDslJsonArray unorderedMaxArray(int size) {
    throw new UnsupportedOperationException("use the unorderedMaxArray(String name, int size) form");
  }

  @Override
  public PactDslJsonArray unorderedMinMaxArray(String name, int minSize, int maxSize) {
    if (minSize > maxSize) {
      throw new IllegalArgumentException(
          String.format("The minimum size of %d is greater than the maximum of %d", minSize, maxSize));
    }
    matchers.addRule(path(name), new MinMaxEqualsIgnoreOrderMatcher(minSize, maxSize));
    return array(name);
  }

  @Override
  public PactDslJsonArray unorderedMinMaxArray(int minSize, int maxSize) {
    throw new UnsupportedOperationException("use the unorderedMinMaxArray(String name, int minSize, int maxSize) form");
  }

  @Override
  public DslPart arrayContaining(String name) {
    return new PactDslJsonArrayContaining(rootPath, name, this);
  }

  /** An array at {@code name} whose items are like {@code obj}, with {@code rule} at the array. */
  private PactDslJsonBody arrayOf(String name, MatchingRule rule, DslPart obj, Integer numberExamples) {
    String base = pathByName(name);
    matchers.addRule(base, rule);
    PactDslJsonArray array = new PactDslJsonArray(base, name, this, true);
    if (numberExamples != null) {
      array.setNumberExamples(numberExamples);
    }
    if (obj instanceof PactDslJsonBody) {
      array.putObjectPrivate(obj);
    } else if (obj instanceof PactDslJsonArray) {
      array.putArrayPrivate(obj);
    } else {
      array.putObjectPrivate(obj);
    }
    return (PactDslJsonBody) array.closeArray();
  }

  /** The template of an array at {@code name}, with {@code rule} at the array. */
  private PactDslJsonBody templateOf(String name, MatchingRule rule, int numberExamples, String templateName) {
    String path = pathByName(name);
    matchers.addRule(path, rule);
    PactDslJsonArray array = new PactDslJsonArray(path, name, this, true);
    array.setNumberExamples(numberExamples);
    return new PactDslJsonBody(".", templateName, array, numberExamples);
  }

  @Override
  public PactDslJsonBody eachLike(String name) {
    return eachLike(name, 1);
  }

  @Override
  public PactDslJsonBody eachLike(String name, DslPart obj) {
    return arrayOf(name, TypeMatcher.INSTANCE, obj, null);
  }

  @Override
  public PactDslJsonBody eachLike() {
    throw new UnsupportedOperationException("use the eachLike(String name) form");
  }

  @Override
  public PactDslJsonArray eachLike(DslPart obj) {
    throw new UnsupportedOperationException("use the eachLike(String name, DslPart object) form");
  }

  @Override
  public PactDslJsonBody eachLike(String name, int numberExamples) {
    return templateOf(name, TypeMatcher.INSTANCE, numberExamples, ".");
  }

  @Override
  public PactDslJsonBody eachLike(int numberExamples) {
    throw new UnsupportedOperationException("use the eachLike(String name, int numberExamples) form");
  }

  public PactDslJsonBody eachLike(String name, PactDslJsonRootValue value) {
    return eachLike(name, value, 1);
  }

  public PactDslJsonBody eachLike(String name, PactDslJsonRootValue value, int numberExamples) {
    return arrayOf(name, TypeMatcher.INSTANCE, value, numberExamples);
  }

  @Override
  public PactDslJsonBody minArrayLike(String name, int size) {
    return minArrayLike(name, size, size);
  }

  @Override
  public PactDslJsonBody minArrayLike(int size) {
    throw new UnsupportedOperationException("use the minArrayLike(String name, Integer size) form");
  }

  @Override
  public PactDslJsonBody minArrayLike(String name, int size, DslPart obj) {
    return arrayOf(name, matchMin(size), obj, size);
  }

  @Override
  public PactDslJsonArray minArrayLike(int size, DslPart obj) {
    throw new UnsupportedOperationException("use the minArrayLike(String name, Integer size, DslPart object) form");
  }

  @Override
  public PactDslJsonBody minArrayLike(String name, int size, int numberExamples) {
    if (numberExamples < size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is less than the minimum size of %d", numberExamples, size));
    }
    return templateOf(name, matchMin(size), numberExamples, "");
  }

  @Override
  public PactDslJsonBody minArrayLike(int size, int numberExamples) {
    throw new UnsupportedOperationException("use the minArrayLike(String name, Integer size, int numberExamples) form");
  }

  public PactDslJsonBody minArrayLike(String name, int size, PactDslJsonRootValue value) {
    return minArrayLike(name, size, value, 2);
  }

  public PactDslJsonBody minArrayLike(String name, int size, PactDslJsonRootValue value, int numberExamples) {
    return minArrayLike(name, size, (DslPart) value, numberExamples);
  }

  public PactDslJsonBody minArrayLike(String name, int size, DslPart value, int numberExamples) {
    if (numberExamples < size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is less than the minimum size of %d", numberExamples, size));
    }
    String path = pathByName(name);
    matchers.addRule(path, matchMin(size));
    PactDslJsonArray array = new PactDslJsonArray(path, name, this, true);
    array.setNumberExamples(numberExamples);
    array.putObjectPrivate(value);
    return (PactDslJsonBody) array.closeArray();
  }

  @Override
  public PactDslJsonBody maxArrayLike(String name, int size) {
    return maxArrayLike(name, size, 1);
  }

  @Override
  public PactDslJsonBody maxArrayLike(int size) {
    throw new UnsupportedOperationException("use the maxArrayLike(String name, Integer size) form");
  }

  @Override
  public PactDslJsonBody maxArrayLike(String name, int size, DslPart obj) {
    return arrayOf(name, matchMax(size), obj, null);
  }

  @Override
  public PactDslJsonArray maxArrayLike(int size, DslPart obj) {
    throw new UnsupportedOperationException("use the maxArrayLike(String name, Integer size, DslPart object) form");
  }

  @Override
  public PactDslJsonBody maxArrayLike(String name, int size, int numberExamples) {
    if (numberExamples > size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is more than the maximum size of %d", numberExamples, size));
    }
    return templateOf(name, matchMax(size), numberExamples, "");
  }

  @Override
  public PactDslJsonBody maxArrayLike(int size, int numberExamples) {
    throw new UnsupportedOperationException("use the maxArrayLike(String name, Integer size, int numberExamples) form");
  }

  public PactDslJsonBody maxArrayLike(String name, int size, PactDslJsonRootValue value) {
    return maxArrayLike(name, size, value, 1);
  }

  public PactDslJsonBody maxArrayLike(String name, int size, PactDslJsonRootValue value, int numberExamples) {
    return maxArrayLike(name, size, (DslPart) value, numberExamples);
  }

  public PactDslJsonBody maxArrayLike(String name, int size, DslPart value, int numberExamples) {
    if (numberExamples > size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is more than the maximum size of %d", numberExamples, size));
    }
    String path = pathByName(name);
    matchers.addRule(path, matchMax(size));
    PactDslJsonArray array = new PactDslJsonArray(path, name, this, true);
    array.setNumberExamples(numberExamples);
    array.putObjectPrivate(value);
    return (PactDslJsonBody) array.closeArray();
  }

  private static void validateMinAndMaxAndExamples(int minSize, int maxSize, int numberExamples) {
    if (minSize > maxSize) {
      throw new IllegalArgumentException(
          String.format("The minimum size %d is more than the maximum size of %d", minSize, maxSize));
    }
    if (numberExamples < minSize) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is less than the minimum size of %d", numberExamples, minSize));
    }
    if (numberExamples > maxSize) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is greater than the maximum size of %d", numberExamples, maxSize));
    }
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize) {
    return minMaxArrayLike(name, minSize, maxSize, minSize);
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize, DslPart obj) {
    validateMinAndMaxAndExamples(minSize, maxSize, minSize);
    return arrayOf(name, matchMinMax(minSize, maxSize), obj, minSize);
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(int minSize, int maxSize) {
    throw new UnsupportedOperationException("use the minMaxArrayLike(String name, Integer minSize, Integer maxSize) form");
  }

  @Override
  public PactDslJsonArray minMaxArrayLike(int minSize, int maxSize, DslPart obj) {
    throw new UnsupportedOperationException(
        "use the minMaxArrayLike(String name, Integer minSize, Integer maxSize, DslPart object) form");
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize, int numberExamples) {
    validateMinAndMaxAndExamples(minSize, maxSize, numberExamples);
    return templateOf(name, matchMinMax(minSize, maxSize), numberExamples, "");
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(int minSize, int maxSize, int numberExamples) {
    throw new UnsupportedOperationException(
        "use the minMaxArrayLike(String name, Integer minSize, Integer maxSize, int numberExamples) form");
  }

  public PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize, PactDslJsonRootValue value) {
    return minMaxArrayLike(name, minSize, maxSize, value, 1);
  }

  public PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize, PactDslJsonRootValue value,
      int numberExamples) {
    return minMaxArrayLike(name, minSize, maxSize, (DslPart) value, numberExamples);
  }

  public PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize, DslPart value, int numberExamples) {
    validateMinAndMaxAndExamples(minSize, maxSize, numberExamples);
    String path = pathByName(name);
    matchers.addRule(path, matchMinMax(minSize, maxSize));
    PactDslJsonArray array = new PactDslJsonArray(path, name, this, true);
    array.setNumberExamples(numberExamples);
    array.putObjectPrivate(value);
    return (PactDslJsonBody) array.closeArray();
  }

  /** An array of arrays at {@code name}, with {@code rule} at it. */
  private PactDslJsonArray arrayOfArrays(String name, MatchingRule rule, int numberExamples) {
    String path = pathByName(name);
    matchers.addRule(path, rule);
    PactDslJsonArray array = new PactDslJsonArray(path, name, this, true);
    array.setNumberExamples(numberExamples);
    return new PactDslJsonArray("", "", array);
  }

  @Override
  public PactDslJsonArray eachArrayLike(String name) {
    return eachArrayLike(name, 1);
  }

  @Override
  public PactDslJsonArray eachArrayLike() {
    throw new UnsupportedOperationException("use the eachArrayLike(String name) form");
  }

  @Override
  public PactDslJsonArray eachArrayLike(String name, int numberExamples) {
    return arrayOfArrays(name, TypeMatcher.INSTANCE, numberExamples);
  }

  @Override
  public PactDslJsonArray eachArrayLike(int numberExamples) {
    throw new UnsupportedOperationException("use the eachArrayLike(String name, int numberExamples) form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(String name, int size) {
    return eachArrayWithMaxLike(name, 1, size);
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(int size) {
    throw new UnsupportedOperationException("use the eachArrayWithMaxLike(String name, Integer size) form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(String name, int numberExamples, int size) {
    if (numberExamples > size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is more than the maximum size of %d", numberExamples, size));
    }
    return arrayOfArrays(name, matchMax(size), numberExamples);
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(int numberExamples, int size) {
    throw new UnsupportedOperationException(
        "use the eachArrayWithMaxLike(String name, int numberExamples, Integer size) form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(String name, int size) {
    return eachArrayWithMinLike(name, size, size);
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(int size) {
    throw new UnsupportedOperationException("use the eachArrayWithMinLike(String name, Integer size) form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(String name, int numberExamples, int size) {
    if (numberExamples < size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is less than the minimum size of %d", numberExamples, size));
    }
    return arrayOfArrays(name, matchMin(size), numberExamples);
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(int numberExamples, int size) {
    throw new UnsupportedOperationException(
        "use the eachArrayWithMinLike(String name, int numberExamples, Integer size) form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(String name, int minSize, int maxSize) {
    return eachArrayWithMinMaxLike(name, minSize, minSize, maxSize);
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(int minSize, int maxSize) {
    throw new UnsupportedOperationException(
        "use the eachArrayWithMinMaxLike(String name, Integer minSize, Integer maxSize) form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(String name, int numberExamples, int minSize, int maxSize) {
    validateMinAndMaxAndExamples(minSize, maxSize, numberExamples);
    return arrayOfArrays(name, matchMinMax(minSize, maxSize), numberExamples);
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(int numberExamples, int minSize, int maxSize) {
    throw new UnsupportedOperationException(
        "use the eachArrayWithMinMaxLike(String name, int numberExamples, Integer minSize, Integer maxSize) form");
  }

  // ----------------------------------------------------------------------------------------------
  // What StringUtils does for pact-jvm.

  /** The rest of {@code b} from where it first differs from {@code a}. */
  static String difference(String a, String b) {
    int i = 0;
    while (i < a.length() && i < b.length() && a.charAt(i) == b.charAt(i)) {
      i++;
    }
    return i == a.length() && i == b.length() ? "" : b.substring(i);
  }

  static String strip(String value, String chars) {
    int start = 0;
    int end = value.length();
    while (start < end && chars.indexOf(value.charAt(start)) >= 0) {
      start++;
    }
    while (end > start && chars.indexOf(value.charAt(end - 1)) >= 0) {
      end--;
    }
    return value.substring(start, end);
  }

  static List<String> list(String... values) {
    return new ArrayList<>(Arrays.asList(values));
  }
}
