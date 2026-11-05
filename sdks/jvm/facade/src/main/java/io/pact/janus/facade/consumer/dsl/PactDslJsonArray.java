// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/src/main/kotlin/.../dsl/PactDslJsonArray.kt; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import io.pact.janus.facade.core.model.generators.Category;
import io.pact.janus.facade.core.model.generators.DateGenerator;
import io.pact.janus.facade.core.model.generators.DateTimeGenerator;
import io.pact.janus.facade.core.model.generators.Generators;
import io.pact.janus.facade.core.model.generators.MockServerURLGenerator;
import io.pact.janus.facade.core.model.generators.ProviderStateGenerator;
import io.pact.janus.facade.core.model.generators.RandomBooleanGenerator;
import io.pact.janus.facade.core.model.generators.RandomDecimalGenerator;
import io.pact.janus.facade.core.model.generators.RandomHexadecimalGenerator;
import io.pact.janus.facade.core.model.generators.RandomIntGenerator;
import io.pact.janus.facade.core.model.generators.RandomStringGenerator;
import io.pact.janus.facade.core.model.generators.TimeGenerator;
import io.pact.janus.facade.core.model.generators.UuidGenerator;
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
import io.pact.janus.facade.internal.Dates;
import io.pact.janus.facade.internal.Json;
import java.math.BigDecimal;
import java.time.Instant;
import java.time.format.DateTimeFormatter;
import java.util.Date;
import java.util.List;

/** A JSON array in a body: its items' examples, and the rules and generators at them. */
public class PactDslJsonArray extends DslPart {
  private static final String ISO_DATETIME = "yyyy-MM-dd'T'HH:mm:ss";
  private static final String ISO_DATE = "yyyy-MM-dd";
  private static final String ISO_TIME = "'T'HH:mm:ss";
  private static final String ISO_TIME_NO_T = "HH:mm:ss";

  private JsonNode body;
  private boolean wildCard;
  private int numberExamples = 1;

  public PactDslJsonArray(String rootPath, String rootName, DslPart parent, PactDslJsonArray array) {
    super(parent, rootPath, rootName);
    body = array.body;
    wildCard = array.wildCard;
    matchers = array.matchers.copyWithUpdatedMatcherRootPrefix(rootPath);
    generators = array.generators;
  }

  public PactDslJsonArray() {
    this("", "", null, false);
  }

  public PactDslJsonArray(String rootPath) {
    this(rootPath, "", null, false);
  }

  public PactDslJsonArray(String rootPath, String rootName) {
    this(rootPath, rootName, null, false);
  }

  public PactDslJsonArray(String rootPath, String rootName, DslPart parent) {
    this(rootPath, rootName, parent, false);
  }

  public PactDslJsonArray(String rootPath, String rootName, DslPart parent, boolean wildCard) {
    super(parent, rootPath, rootName);
    this.wildCard = wildCard;
    body = Json.NODES.arrayNode();
  }

  @Override
  public JsonNode getBody() {
    return body;
  }

  @Override
  public void setBody(JsonNode body) {
    this.body = body;
  }

  public int getNumberExamples() {
    return numberExamples;
  }

  public void setNumberExamples(int numberExamples) {
    this.numberExamples = numberExamples;
  }

  private ArrayNode items() {
    return (ArrayNode) body;
  }

  /** pact-jvm's {@code appendArrayIndex}: {@code [*]} for a wildcard array, else the index of the item at {@code offset}. */
  private String index(int offset) {
    return "[" + (wildCard ? "*" : Integer.toString(items().size() - 1 + offset)) + "]";
  }

  private String at(int offset) {
    return rootPath + index(offset);
  }

  private PactDslJsonArray add(JsonNode value, MatchingRule rule) {
    items().add(value);
    if (rule != null) {
      matchers.addRule(at(0), rule);
    }
    return this;
  }

  @Override
  public DslPart closeArray() {
    if (parent != null) {
      parent.putArrayPrivate(this);
    } else {
      matchers.applyMatcherRootPrefix("$");
      generators.applyRootPrefix("$");
    }
    closed = true;
    return parent;
  }

  @Override
  protected void putObjectPrivate(DslPart obj) {
    String prefix = at(1);
    obj.matchers.getMatchingRules().forEach((name, group) -> matchers.setRules(prefix + name, group));
    addGenerators(obj.generators, prefix);
    if (obj instanceof PactDslJsonBody && obj.getBody() instanceof ArrayNode array) {
      body = array;
    } else {
      for (int i = 0; i < numberExamples; i++) {
        items().add(obj.getBody());
      }
    }
  }

  @Override
  protected void putArrayPrivate(DslPart obj) {
    String prefix = at(1);
    obj.matchers.getMatchingRules().forEach((name, group) -> matchers.setRules(prefix + name, group));
    addGenerators(obj.generators, prefix);
    for (int i = 0; i < numberExamples; i++) {
      items().add(obj.getBody());
    }
  }

  private void addGenerators(Generators other, String prefix) {
    other.getCategories().forEach((category, map) ->
        map.forEach((key, generator) -> generators.addGenerator(category, prefix + key, generator)));
  }

  // ----------------------------------------------------------------------------------------------
  // Values

  public PactDslJsonArray stringValue(String value) {
    return add(jsonString(value), null);
  }

  public PactDslJsonArray string(String value) {
    return stringValue(value);
  }

  public PactDslJsonArray numberValue(Number value) {
    return add(jsonNumber(value), null);
  }

  public PactDslJsonArray number(Number value) {
    return numberValue(value);
  }

  public PactDslJsonArray booleanValue(Boolean value) {
    return add(jsonBool(value), null);
  }

  public PactDslJsonArray like(Object example) {
    return add(toJson(example), TypeMatcher.INSTANCE);
  }

  public PactDslJsonArray stringType() {
    items().add(jsonString("string"));
    generators.addGenerator(Category.BODY, at(0), new RandomStringGenerator(20));
    matchers.addRule(at(0), TypeMatcher.INSTANCE);
    return this;
  }

  public PactDslJsonArray stringType(String example) {
    return add(jsonString(example), TypeMatcher.INSTANCE);
  }

  public PactDslJsonArray numberType() {
    generators.addGenerator(Category.BODY, at(1), new RandomIntGenerator(0, Integer.MAX_VALUE));
    return numberType(100);
  }

  public PactDslJsonArray numberType(Number number) {
    return add(jsonNumber(number), new NumberTypeMatcher(NumberTypeMatcher.NumberType.NUMBER));
  }

  public PactDslJsonArray integerType() {
    generators.addGenerator(Category.BODY, at(1), new RandomIntGenerator(0, Integer.MAX_VALUE));
    return integerType(100L);
  }

  public PactDslJsonArray integerType(Long number) {
    return add(jsonNumber(number), new NumberTypeMatcher(NumberTypeMatcher.NumberType.INTEGER));
  }

  public PactDslJsonArray decimalType() {
    generators.addGenerator(Category.BODY, at(1), new RandomDecimalGenerator(10));
    return decimalType(new BigDecimal("100"));
  }

  public PactDslJsonArray decimalType(BigDecimal number) {
    return add(jsonNumber(number), new NumberTypeMatcher(NumberTypeMatcher.NumberType.DECIMAL));
  }

  public PactDslJsonArray decimalType(Double number) {
    return add(jsonNumber(number), new NumberTypeMatcher(NumberTypeMatcher.NumberType.DECIMAL));
  }

  private PactDslJsonArray numberMatching(String regex, Number example, NumberTypeMatcher.NumberType type) {
    if (!example.toString().matches(regex)) {
      throw new IllegalArgumentException(
          "Example value " + example + " does not match the provided regular expression '" + regex + "'");
    }
    items().add(jsonNumber(example));
    matchers.addRules(at(0), List.of(new NumberTypeMatcher(type), new RegexMatcher(regex, example.toString())));
    return this;
  }

  public PactDslJsonArray numberMatching(String regex, Number example) {
    return numberMatching(regex, example, NumberTypeMatcher.NumberType.NUMBER);
  }

  public PactDslJsonArray decimalMatching(String regex, Double example) {
    return numberMatching(regex, example, NumberTypeMatcher.NumberType.DECIMAL);
  }

  public PactDslJsonArray integerMatching(String regex, Integer example) {
    return numberMatching(regex, example, NumberTypeMatcher.NumberType.INTEGER);
  }

  public PactDslJsonArray booleanType() {
    generators.addGenerator(Category.BODY, at(1), RandomBooleanGenerator.INSTANCE);
    return add(jsonBool(true), TypeMatcher.INSTANCE);
  }

  public PactDslJsonArray booleanType(Boolean example) {
    return add(jsonBool(example), TypeMatcher.INSTANCE);
  }

  public PactDslJsonArray stringMatcher(String regex, String value) {
    if (!value.matches(regex)) {
      throw new InvalidMatcherException("Example \"" + value + "\" does not match regular expression \"" + regex + "\"");
    }
    return add(jsonString(value), regexp(regex));
  }

  public PactDslJsonArray datetime() {
    items().add(jsonString(Dates.local2000(ISO_DATETIME)));
    generators.addGenerator(Category.BODY, at(0), new DateTimeGenerator(ISO_DATETIME, null));
    matchers.addRule(at(0), matchTimestamp(ISO_DATETIME));
    return this;
  }

  public PactDslJsonArray datetime(String format) {
    items().add(jsonString(Dates.local2000(format)));
    generators.addGenerator(Category.BODY, at(0), new DateTimeGenerator(format, null));
    matchers.addRule(at(0), matchTimestamp(format));
    return this;
  }

  public PactDslJsonArray datetime(String format, Date example) {
    return add(jsonString(Dates.format(format, java.util.TimeZone.getDefault(), example)), matchTimestamp(format));
  }

  public PactDslJsonArray datetime(String format, Instant example) {
    return add(jsonString(DateTimeFormatter.ofPattern(format).format(example)), matchTimestamp(format));
  }

  public PactDslJsonArray date() {
    items().add(jsonString(Dates.local2000(ISO_DATE)));
    generators.addGenerator(Category.BODY, at(0), new DateGenerator(ISO_DATE, null));
    matchers.addRule(at(0), matchDate(ISO_DATE));
    return this;
  }

  public PactDslJsonArray date(String format) {
    items().add(jsonString(Dates.local2000(format)));
    // pact-jvm adds a date-time generator here.
    generators.addGenerator(Category.BODY, at(0), new DateTimeGenerator(format, null));
    matchers.addRule(at(0), matchDate(format));
    return this;
  }

  public PactDslJsonArray date(String format, Date example) {
    return add(jsonString(Dates.format(format, java.util.TimeZone.getDefault(), example)), matchDate(format));
  }

  public PactDslJsonArray time() {
    items().add(jsonString(Dates.local2000(ISO_TIME)));
    generators.addGenerator(Category.BODY, at(0), new TimeGenerator(ISO_TIME, null));
    matchers.addRule(at(0), matchTime(ISO_TIME));
    return this;
  }

  public PactDslJsonArray time(String format) {
    items().add(jsonString(Dates.local2000(format)));
    generators.addGenerator(Category.BODY, at(0), new TimeGenerator(format, null));
    matchers.addRule(at(0), matchTime(format));
    return this;
  }

  public PactDslJsonArray time(String format, Date example) {
    return add(jsonString(Dates.format(format, java.util.TimeZone.getDefault(), example)), matchTime(format));
  }

  public PactDslJsonArray dateExpression(String expression) {
    return dateExpression(expression, ISO_DATE);
  }

  public PactDslJsonArray dateExpression(String expression, String format) {
    items().add(jsonString(Dates.local2000(format)));
    generators.addGenerator(Category.BODY, at(0), new DateGenerator(format, expression));
    matchers.addRule(at(0), matchDate(format));
    return this;
  }

  public PactDslJsonArray timeExpression(String expression) {
    return timeExpression(expression, ISO_TIME_NO_T);
  }

  public PactDslJsonArray timeExpression(String expression, String format) {
    items().add(jsonString(Dates.local2000(format)));
    generators.addGenerator(Category.BODY, at(0), new TimeGenerator(format, expression));
    matchers.addRule(at(0), matchTime(format));
    return this;
  }

  public PactDslJsonArray datetimeExpression(String expression) {
    return datetimeExpression(expression, ISO_DATETIME);
  }

  public PactDslJsonArray datetimeExpression(String expression, String format) {
    items().add(jsonString(Dates.local2000(format)));
    generators.addGenerator(Category.BODY, at(0), new DateTimeGenerator(format, expression));
    matchers.addRule(at(0), matchTimestamp(format));
    return this;
  }

  public PactDslJsonArray ipAddress() {
    return add(jsonString("127.0.0.1"), regexp("(\\d{1,3}\\.)+\\d{1,3}"));
  }

  public PactDslJsonArray id() {
    items().add(jsonNumber(100));
    generators.addGenerator(Category.BODY, at(0), new RandomIntGenerator(0, Integer.MAX_VALUE));
    matchers.addRule(at(0), TypeMatcher.INSTANCE);
    return this;
  }

  public PactDslJsonArray id(Long id) {
    return add(jsonNumber(id), TypeMatcher.INSTANCE);
  }

  public PactDslJsonArray hexValue() {
    generators.addGenerator(Category.BODY, at(1), new RandomHexadecimalGenerator(10));
    return hexValue("1234a");
  }

  public PactDslJsonArray hexValue(String hexValue) {
    if (!HEXADECIMAL.matcher(hexValue).matches()) {
      throw new InvalidMatcherException("Example \"" + hexValue + "\" is not a hexadecimal value");
    }
    return add(jsonString(hexValue), regexp("[0-9a-fA-F]+"));
  }

  public PactDslJsonArray uuid() {
    generators.addGenerator(Category.BODY, at(1), new UuidGenerator(null));
    return uuid("e2490de5-5bd3-43d5-b7c4-526e33f71304");
  }

  public PactDslJsonArray uuid(String uuid) {
    if (!UUID_REGEX.matcher(uuid).matches()) {
      throw new InvalidMatcherException("Example \"" + uuid + "\" is not an UUID");
    }
    return add(jsonString(uuid), regexp(UUID_REGEX.pattern()));
  }

  public PactDslJsonArray nullValue() {
    return add(Json.NODES.nullNode(), null);
  }

  public PactDslJsonArray includesStr(String value) {
    return add(jsonString(value), includesMatcher(value));
  }

  public PactDslJsonArray equalsTo(Object value) {
    return add(toJson(value), EqualsMatcher.INSTANCE);
  }

  public PactDslJsonArray and(Object value, MatchingRule... rules) {
    items().add(toJson(value));
    matchers.setRules(at(0), new MatchingRuleGroup(List.of(rules), RuleLogic.AND));
    return this;
  }

  public PactDslJsonArray or(Object value, MatchingRule... rules) {
    items().add(toJson(value));
    matchers.setRules(at(0), new MatchingRuleGroup(List.of(rules), RuleLogic.OR));
    return this;
  }

  public PactDslJsonArray valueFromProviderState(String expression, Object example) {
    items().add(toJson(example));
    generators.addGenerator(Category.BODY, at(0), new ProviderStateGenerator(expression, dataType(example)));
    matchers.addRule(at(0), TypeMatcher.INSTANCE);
    return this;
  }

  @Override
  public PactDslJsonArray matchUrl(String basePath, Object... pathFragments) {
    String[] url = urlMatcher(basePath, pathFragments);
    items().add(jsonString(url[0]));
    matchers.addRule(at(0), regexp(url[1]));
    if (basePath == null || basePath.isEmpty()) {
      generators.addGenerator(Category.BODY, at(0), new MockServerURLGenerator(url[0], url[1]));
    }
    return this;
  }

  @Override
  public DslPart matchUrl(String name, String basePath, Object... pathFragments) {
    throw new UnsupportedOperationException("URL matcher with an attribute name is not supported for arrays. "
        + "Use matchUrl(String base, Object... fragments)");
  }

  @Override
  public PactDslJsonBody matchUrl2(String name, Object... pathFragments) {
    throw new UnsupportedOperationException("URL matcher with an attribute name is not supported for arrays. "
        + "Use matchUrl2(Object... pathFragments)");
  }

  @Override
  public DslPart matchUrl2(Object... pathFragments) {
    return matchUrl(null, pathFragments);
  }

  public PactDslJsonArray template(DslPart template) {
    putObjectPrivate(template);
    return this;
  }

  public PactDslJsonArray template(DslPart template, int occurrences) {
    for (int i = 0; i < occurrences; i++) {
      template(template);
    }
    return this;
  }

  public PactDslJsonArray wildcardArrayMatcher(MatchingRule rule) {
    wildCard = true;
    matchers.addRule(at(1), rule);
    return this;
  }

  // ----------------------------------------------------------------------------------------------
  // Objects and arrays in it

  @Override
  public PactDslJsonBody object(String name) {
    throw new UnsupportedOperationException("use the object() form");
  }

  @Override
  public PactDslJsonBody object() {
    return new PactDslJsonBody(".", "", this);
  }

  @Override
  public DslPart closeObject() {
    throw new UnsupportedOperationException("can't call closeObject on an Array");
  }

  @Override
  public DslPart close() {
    DslPart parentToReturn = this;
    if (!closed) {
      DslPart next = closeArray();
      while (next != null) {
        parentToReturn = next;
        next = next instanceof PactDslJsonArray ? next.closeArray() : next.closeObject();
      }
    }
    return parentToReturn;
  }

  @Override
  public DslPart arrayContaining(String name) {
    throw new UnsupportedOperationException("arrayContaining is not currently supported for arrays");
  }

  @Override
  public PactDslJsonArray array(String name) {
    throw new UnsupportedOperationException("use the array() form");
  }

  @Override
  public PactDslJsonArray array() {
    return new PactDslJsonArray("", "", this);
  }

  @Override
  public PactDslJsonArray unorderedArray(String name) {
    throw new UnsupportedOperationException("use the unorderedArray() form");
  }

  @Override
  public PactDslJsonArray unorderedArray() {
    matchers.addRule(at(1), EqualsIgnoreOrderMatcher.INSTANCE);
    return array();
  }

  @Override
  public PactDslJsonArray unorderedMinArray(String name, int size) {
    throw new UnsupportedOperationException("use the unorderedMinArray(int size) form");
  }

  @Override
  public PactDslJsonArray unorderedMinArray(int size) {
    matchers.addRule(at(1), new MinEqualsIgnoreOrderMatcher(size));
    return array();
  }

  @Override
  public PactDslJsonArray unorderedMaxArray(String name, int size) {
    throw new UnsupportedOperationException("use the unorderedMaxArray(int size) form");
  }

  @Override
  public PactDslJsonArray unorderedMaxArray(int size) {
    matchers.addRule(at(1), new MaxEqualsIgnoreOrderMatcher(size));
    return array();
  }

  @Override
  public PactDslJsonArray unorderedMinMaxArray(String name, int minSize, int maxSize) {
    throw new UnsupportedOperationException("use the unorderedMinMaxArray(int minSize, int maxSize) form");
  }

  @Override
  public PactDslJsonArray unorderedMinMaxArray(int minSize, int maxSize) {
    if (minSize > maxSize) {
      throw new IllegalArgumentException(
          String.format("The minimum size of %d is greater than the maximum of %d", minSize, maxSize));
    }
    matchers.addRule(at(1), new MinMaxEqualsIgnoreOrderMatcher(minSize, maxSize));
    return array();
  }

  /** Items like {@code obj}, with {@code rule} at the item. */
  private PactDslJsonArray itemsLike(MatchingRule rule, DslPart obj, Integer numberExamples) {
    matchers.addRule(at(1), rule);
    PactDslJsonArray array = new PactDslJsonArray(rootPath, "", this, true);
    array.numberExamples = numberExamples == null ? this.numberExamples : numberExamples;
    if (obj instanceof PactDslJsonArray) {
      array.putArrayPrivate(obj);
    } else {
      array.putObjectPrivate(obj);
    }
    return (PactDslJsonArray) array.closeArray();
  }

  /** The template of the items, with {@code rule} at the item. */
  private PactDslJsonBody itemTemplate(MatchingRule rule, int numberExamples, boolean rooted) {
    matchers.addRule(at(1), rule);
    PactDslJsonArray array = new PactDslJsonArray(rooted ? rootPath : "", "", this, true);
    array.numberExamples = numberExamples;
    return new PactDslJsonBody(".", "", array);
  }

  @Override
  public PactDslJsonBody eachLike(String name) {
    throw new UnsupportedOperationException("use the eachLike() form");
  }

  @Override
  public PactDslJsonBody eachLike(String name, DslPart obj) {
    throw new UnsupportedOperationException("use the eachLike(DslPart object) form");
  }

  @Override
  public PactDslJsonBody eachLike(String name, int numberExamples) {
    throw new UnsupportedOperationException("use the eachLike(numberExamples) form");
  }

  @Override
  public PactDslJsonBody eachLike() {
    return eachLike(1);
  }

  @Override
  public PactDslJsonArray eachLike(DslPart obj) {
    return itemsLike(TypeMatcher.INSTANCE, obj, null);
  }

  @Override
  public PactDslJsonBody eachLike(int numberExamples) {
    return itemTemplate(TypeMatcher.INSTANCE, numberExamples, true);
  }

  public PactDslJsonArray eachLike(PactDslJsonRootValue value) {
    return eachLike(value, 1);
  }

  public PactDslJsonArray eachLike(PactDslJsonRootValue value, int numberExamples) {
    if (numberExamples == 0) {
      throw new IllegalArgumentException("Testing Zero examples is unsafe. Please make sure to provide at least one "
          + "example in the Pact provider implementation. See https://github.com/DiUS/pact-jvm/issues/546");
    }
    return itemsLike(TypeMatcher.INSTANCE, value, numberExamples);
  }

  @Override
  public PactDslJsonBody minArrayLike(String name, int size) {
    throw new UnsupportedOperationException("use the minArrayLike(Integer size) form");
  }

  @Override
  public PactDslJsonBody minArrayLike(int size) {
    return minArrayLike(size, size);
  }

  @Override
  public PactDslJsonBody minArrayLike(String name, int size, DslPart obj) {
    throw new UnsupportedOperationException("use the minArrayLike(Integer size, DslPart object) form");
  }

  @Override
  public PactDslJsonArray minArrayLike(int size, DslPart obj) {
    return itemsLike(matchMin(size), obj, size);
  }

  @Override
  public PactDslJsonBody minArrayLike(String name, int size, int numberExamples) {
    throw new UnsupportedOperationException("use the minArrayLike(Integer size, int numberExamples) form");
  }

  @Override
  public PactDslJsonBody minArrayLike(int size, int numberExamples) {
    if (numberExamples < size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is less than the minimum size of %d", numberExamples, size));
    }
    return itemTemplate(matchMin(size), numberExamples, false);
  }

  public PactDslJsonArray minArrayLike(int size, PactDslJsonRootValue value) {
    return minArrayLike(size, value, size);
  }

  public PactDslJsonArray minArrayLike(int size, PactDslJsonRootValue value, int numberExamples) {
    if (numberExamples < size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is less than the minimum size of %d", numberExamples, size));
    }
    return itemsLike(matchMin(size), value, numberExamples);
  }

  @Override
  public PactDslJsonBody maxArrayLike(String name, int size) {
    throw new UnsupportedOperationException("use the maxArrayLike(Integer size) form");
  }

  @Override
  public PactDslJsonBody maxArrayLike(int size) {
    return maxArrayLike(size, 1);
  }

  @Override
  public PactDslJsonBody maxArrayLike(String name, int size, DslPart obj) {
    throw new UnsupportedOperationException("use the maxArrayLike(Integer size, DslPart object) form");
  }

  @Override
  public PactDslJsonArray maxArrayLike(int size, DslPart obj) {
    return itemsLike(matchMax(size), obj, 1);
  }

  @Override
  public PactDslJsonBody maxArrayLike(String name, int size, int numberExamples) {
    throw new UnsupportedOperationException("use the maxArrayLike(Integer size, int numberExamples) form");
  }

  @Override
  public PactDslJsonBody maxArrayLike(int size, int numberExamples) {
    if (numberExamples > size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is more than the maximum size of %d", numberExamples, size));
    }
    return itemTemplate(matchMax(size), numberExamples, false);
  }

  public PactDslJsonArray maxArrayLike(int size, PactDslJsonRootValue value) {
    return maxArrayLike(size, value, 1);
  }

  public PactDslJsonArray maxArrayLike(int size, PactDslJsonRootValue value, int numberExamples) {
    if (numberExamples > size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is more than the maximum size of %d", numberExamples, size));
    }
    return itemsLike(matchMax(size), value, numberExamples);
  }

  private static void validate(int minSize, int maxSize, int numberExamples) {
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
    throw new UnsupportedOperationException("use the minMaxArrayLike(Integer minSize, Integer maxSize) form");
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize, DslPart obj) {
    throw new UnsupportedOperationException("use the minMaxArrayLike(Integer minSize, Integer maxSize, DslPart obj) form");
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(int minSize, int maxSize) {
    return minMaxArrayLike(minSize, maxSize, minSize);
  }

  @Override
  public PactDslJsonArray minMaxArrayLike(int minSize, int maxSize, DslPart obj) {
    validate(minSize, maxSize, minSize);
    return itemsLike(matchMinMax(minSize, maxSize), obj, minSize);
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize, int numberExamples) {
    throw new UnsupportedOperationException(
        "use the minMaxArrayLike(Integer minSize, Integer maxSize, int numberExamples) form");
  }

  @Override
  public PactDslJsonBody minMaxArrayLike(int minSize, int maxSize, int numberExamples) {
    validate(minSize, maxSize, numberExamples);
    return itemTemplate(matchMinMax(minSize, maxSize), numberExamples, false);
  }

  /** Items that are arrays, with {@code rule} at the item. */
  private PactDslJsonArray arraysLike(MatchingRule rule, int numberExamples) {
    matchers.addRule(at(1), rule);
    PactDslJsonArray array = new PactDslJsonArray(rootPath, "", this, true);
    array.numberExamples = numberExamples;
    return new PactDslJsonArray("", "", array);
  }

  @Override
  public PactDslJsonArray eachArrayLike(String name) {
    throw new UnsupportedOperationException("use the eachArrayLike() form");
  }

  @Override
  public PactDslJsonArray eachArrayLike(String name, int numberExamples) {
    throw new UnsupportedOperationException("use the eachArrayLike(numberExamples) form");
  }

  @Override
  public PactDslJsonArray eachArrayLike() {
    return eachArrayLike(1);
  }

  @Override
  public PactDslJsonArray eachArrayLike(int numberExamples) {
    return arraysLike(TypeMatcher.INSTANCE, numberExamples);
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(String name, int size) {
    throw new UnsupportedOperationException("use the eachArrayWithMaxLike() form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(String name, int numberExamples, int size) {
    throw new UnsupportedOperationException("use the eachArrayWithMaxLike(numberExamples) form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(int size) {
    return eachArrayWithMaxLike(1, size);
  }

  @Override
  public PactDslJsonArray eachArrayWithMaxLike(int numberExamples, int size) {
    if (numberExamples > size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is more than the maximum size of %d", numberExamples, size));
    }
    return arraysLike(matchMax(size), numberExamples);
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(String name, int size) {
    throw new UnsupportedOperationException("use the eachArrayWithMinLike() form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(String name, int numberExamples, int size) {
    throw new UnsupportedOperationException("use the eachArrayWithMinLike(numberExamples) form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(int size) {
    return eachArrayWithMinLike(size, size);
  }

  @Override
  public PactDslJsonArray eachArrayWithMinLike(int numberExamples, int size) {
    if (numberExamples < size) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is less than the minimum size of %d", numberExamples, size));
    }
    return arraysLike(matchMin(size), numberExamples);
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(String name, int minSize, int maxSize) {
    throw new UnsupportedOperationException("use the eachArrayWithMinMaxLike() form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(int minSize, int maxSize) {
    return eachArrayWithMinMaxLike(minSize, minSize, maxSize);
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(String name, int numberExamples, int minSize, int maxSize) {
    throw new UnsupportedOperationException("use the eachArrayWithMinMaxLike(numberExamples, minSize, maxSize) form");
  }

  @Override
  public PactDslJsonArray eachArrayWithMinMaxLike(int numberExamples, int minSize, int maxSize) {
    validate(minSize, maxSize, numberExamples);
    return arraysLike(matchMinMax(minSize, maxSize), numberExamples);
  }

  // ----------------------------------------------------------------------------------------------
  // Root arrays

  public static PactDslJsonBody arrayEachLike() {
    return arrayEachLike(1);
  }

  public static PactDslJsonBody arrayEachLike(int numberExamples) {
    PactDslJsonArray parent = new PactDslJsonArray("", "", null, true);
    parent.numberExamples = numberExamples;
    parent.matchers.addRule("", TypeMatcher.INSTANCE);
    return new PactDslJsonBody(".", "", parent);
  }

  public static PactDslJsonArray arrayEachLike(PactDslJsonRootValue rootValue) {
    return arrayEachLike(1, rootValue);
  }

  public static PactDslJsonArray arrayEachLike(int numberExamples, PactDslJsonRootValue value) {
    PactDslJsonArray parent = new PactDslJsonArray("", "", null, true);
    parent.numberExamples = numberExamples;
    parent.matchers.addRule("", TypeMatcher.INSTANCE);
    parent.putObjectPrivate(value);
    return parent;
  }

  public static PactDslJsonBody arrayMinLike(int minSize) {
    return arrayMinLike(minSize, minSize);
  }

  public static PactDslJsonBody arrayMinLike(int minSize, int numberExamples) {
    if (numberExamples < minSize) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is less than the minimum size of %d", numberExamples, minSize));
    }
    PactDslJsonArray parent = new PactDslJsonArray("", "", null, true);
    parent.numberExamples = numberExamples;
    parent.matchers.addRule("", parent.matchMin(minSize));
    return new PactDslJsonBody(".", "", parent);
  }

  public static PactDslJsonArray arrayMinLike(int minSize, PactDslJsonRootValue value) {
    return arrayMinLike(minSize, minSize, value);
  }

  public static PactDslJsonArray arrayMinLike(int minSize, int numberExamples, PactDslJsonRootValue value) {
    if (numberExamples < minSize) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is less than the minimum size of %d", numberExamples, minSize));
    }
    PactDslJsonArray parent = new PactDslJsonArray("", "", null, true);
    parent.numberExamples = numberExamples;
    parent.matchers.addRule("", parent.matchMin(minSize));
    parent.putObjectPrivate(value);
    return parent;
  }

  public static PactDslJsonBody arrayMaxLike(int maxSize) {
    return arrayMaxLike(maxSize, 1);
  }

  public static PactDslJsonBody arrayMaxLike(int maxSize, int numberExamples) {
    if (numberExamples > maxSize) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is more than the maximum size of %d", numberExamples, maxSize));
    }
    PactDslJsonArray parent = new PactDslJsonArray("", "", null, true);
    parent.numberExamples = numberExamples;
    parent.matchers.addRule("", parent.matchMax(maxSize));
    return new PactDslJsonBody(".", "", parent);
  }

  public static PactDslJsonArray arrayMaxLike(int maxSize, PactDslJsonRootValue value) {
    return arrayMaxLike(maxSize, 1, value);
  }

  public static PactDslJsonArray arrayMaxLike(int maxSize, int numberExamples, PactDslJsonRootValue value) {
    if (numberExamples > maxSize) {
      throw new IllegalArgumentException(
          String.format("Number of example %d is more than the maximum size of %d", numberExamples, maxSize));
    }
    PactDslJsonArray parent = new PactDslJsonArray("", "", null, true);
    parent.numberExamples = numberExamples;
    parent.matchers.addRule("", parent.matchMax(maxSize));
    parent.putObjectPrivate(value);
    return parent;
  }

  public static PactDslJsonBody arrayMinMaxLike(int minSize, int maxSize) {
    return arrayMinMaxLike(minSize, maxSize, minSize);
  }

  public static PactDslJsonBody arrayMinMaxLike(int minSize, int maxSize, int numberExamples) {
    validate(minSize, maxSize, numberExamples);
    PactDslJsonArray parent = new PactDslJsonArray("", "", null, true);
    parent.numberExamples = numberExamples;
    parent.matchers.addRule("", parent.matchMinMax(minSize, maxSize));
    return new PactDslJsonBody(".", "", parent);
  }

  public static PactDslJsonArray arrayMinMaxLike(int minSize, int maxSize, PactDslJsonRootValue value) {
    return arrayMinMaxLike(minSize, maxSize, minSize, value);
  }

  public static PactDslJsonArray arrayMinMaxLike(int minSize, int maxSize, int numberExamples,
      PactDslJsonRootValue value) {
    validate(minSize, maxSize, numberExamples);
    PactDslJsonArray parent = new PactDslJsonArray("", "", null, true);
    parent.numberExamples = numberExamples;
    parent.matchers.addRule("", parent.matchMinMax(minSize, maxSize));
    parent.putObjectPrivate(value);
    return parent;
  }

  public static PactDslJsonArray newUnorderedArray() {
    PactDslJsonArray root = new PactDslJsonArray();
    root.matchers.addRule(root.rootPath, EqualsIgnoreOrderMatcher.INSTANCE);
    return root;
  }

  public static PactDslJsonArray newUnorderedMinArray(int size) {
    PactDslJsonArray root = new PactDslJsonArray();
    root.matchers.addRule(root.rootPath, new MinEqualsIgnoreOrderMatcher(size));
    return root;
  }

  public static PactDslJsonArray newUnorderedMaxArray(int size) {
    PactDslJsonArray root = new PactDslJsonArray();
    root.matchers.addRule(root.rootPath, new MaxEqualsIgnoreOrderMatcher(size));
    return root;
  }

  public static PactDslJsonArray newUnorderedMinMaxArray(int minSize, int maxSize) {
    PactDslJsonArray root = new PactDslJsonArray();
    root.matchers.addRule(root.rootPath, new MinMaxEqualsIgnoreOrderMatcher(minSize, maxSize));
    return root;
  }
}
