// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/src/main/kotlin/.../dsl/DslPart.kt; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.util.RawValue;
import io.pact.janus.facade.core.model.generators.Generators;
import io.pact.janus.facade.core.model.matchingrules.DateMatcher;
import io.pact.janus.facade.core.model.matchingrules.EqualsIgnoreOrderMatcher;
import io.pact.janus.facade.core.model.matchingrules.IncludeMatcher;
import io.pact.janus.facade.core.model.matchingrules.MatchingRuleCategory;
import io.pact.janus.facade.core.model.matchingrules.MaxEqualsIgnoreOrderMatcher;
import io.pact.janus.facade.core.model.matchingrules.MaxTypeMatcher;
import io.pact.janus.facade.core.model.matchingrules.MinEqualsIgnoreOrderMatcher;
import io.pact.janus.facade.core.model.matchingrules.MinMaxEqualsIgnoreOrderMatcher;
import io.pact.janus.facade.core.model.matchingrules.MinMaxTypeMatcher;
import io.pact.janus.facade.core.model.matchingrules.MinTypeMatcher;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.model.matchingrules.TimeMatcher;
import io.pact.janus.facade.core.model.matchingrules.TimestampMatcher;
import io.pact.janus.facade.core.support.expressions.DataType;
import io.pact.janus.facade.internal.Json;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.List;
import java.util.regex.Pattern;

/**
 * A part of a JSON body being built: its example, and the rules and generators at the paths under it.
 * Each builder writes them as pact-jvm's does, so a pact-jvm suite records the same rules (ADR 0070).
 */
public abstract class DslPart {
  public static final Pattern HEXADECIMAL = Pattern.compile("[0-9a-fA-F]+");
  public static final Pattern IP_ADDRESS = Pattern.compile("(\\d{1,3}\\.)+\\d{1,3}");
  public static final Pattern UUID_REGEX =
      Pattern.compile("[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}");
  public static final long DATE_2000 = 949323600000L;

  protected final DslPart parent;
  protected final String rootPath;
  protected final String rootName;
  protected MatchingRuleCategory matchers = new MatchingRuleCategory("body");
  protected Generators generators = new Generators();
  protected boolean closed = false;

  protected DslPart(DslPart parent, String rootPath, String rootName) {
    this.parent = parent;
    this.rootPath = rootPath;
    this.rootName = rootName;
  }

  protected DslPart(String rootPath, String rootName) {
    this(null, rootPath, rootName);
  }

  public DslPart getParent() {
    return parent;
  }

  public String getRootPath() {
    return rootPath;
  }

  public String getRootName() {
    return rootName;
  }

  public MatchingRuleCategory getMatchers() {
    return matchers;
  }

  public void setMatchers(MatchingRuleCategory matchers) {
    this.matchers = matchers;
  }

  public Generators getGenerators() {
    return generators;
  }

  public void setGenerators(Generators generators) {
    this.generators = generators;
  }

  /** The example: an object, an array, or for the template of {@code eachLike(n)}, n objects. */
  public abstract JsonNode getBody();

  public abstract void setBody(JsonNode body);

  protected abstract void putObjectPrivate(DslPart obj);

  protected abstract void putArrayPrivate(DslPart obj);

  public abstract PactDslJsonArray array(String name);

  public abstract PactDslJsonArray array();

  public abstract PactDslJsonArray unorderedArray(String name);

  public abstract PactDslJsonArray unorderedArray();

  public abstract PactDslJsonArray unorderedMinArray(String name, int size);

  public abstract PactDslJsonArray unorderedMinArray(int size);

  public abstract PactDslJsonArray unorderedMaxArray(String name, int size);

  public abstract PactDslJsonArray unorderedMaxArray(int size);

  public abstract PactDslJsonArray unorderedMinMaxArray(String name, int minSize, int maxSize);

  public abstract PactDslJsonArray unorderedMinMaxArray(int minSize, int maxSize);

  public abstract DslPart closeArray();

  public abstract PactDslJsonBody eachLike(String name);

  public abstract PactDslJsonBody eachLike(String name, DslPart obj);

  public abstract PactDslJsonBody eachLike();

  public abstract PactDslJsonArray eachLike(DslPart obj);

  public abstract PactDslJsonBody eachLike(String name, int numberExamples);

  public abstract PactDslJsonBody eachLike(int numberExamples);

  public abstract PactDslJsonBody minArrayLike(String name, int size);

  public abstract PactDslJsonBody minArrayLike(int size);

  public abstract PactDslJsonBody minArrayLike(String name, int size, DslPart obj);

  public abstract PactDslJsonArray minArrayLike(int size, DslPart obj);

  public abstract PactDslJsonBody minArrayLike(String name, int size, int numberExamples);

  public abstract PactDslJsonBody minArrayLike(int size, int numberExamples);

  public abstract PactDslJsonBody maxArrayLike(String name, int size);

  public abstract PactDslJsonBody maxArrayLike(int size);

  public abstract PactDslJsonBody maxArrayLike(String name, int size, DslPart obj);

  public abstract PactDslJsonArray maxArrayLike(int size, DslPart obj);

  public abstract PactDslJsonBody maxArrayLike(String name, int size, int numberExamples);

  public abstract PactDslJsonBody maxArrayLike(int size, int numberExamples);

  public abstract PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize);

  public abstract PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize, DslPart obj);

  public abstract PactDslJsonBody minMaxArrayLike(int minSize, int maxSize);

  public abstract PactDslJsonArray minMaxArrayLike(int minSize, int maxSize, DslPart obj);

  public abstract PactDslJsonBody minMaxArrayLike(String name, int minSize, int maxSize, int numberExamples);

  public abstract PactDslJsonBody minMaxArrayLike(int minSize, int maxSize, int numberExamples);

  public abstract PactDslJsonArray eachArrayLike(String name);

  public abstract PactDslJsonArray eachArrayLike();

  public abstract PactDslJsonArray eachArrayLike(String name, int numberExamples);

  public abstract PactDslJsonArray eachArrayLike(int numberExamples);

  public abstract PactDslJsonArray eachArrayWithMaxLike(String name, int size);

  public abstract PactDslJsonArray eachArrayWithMaxLike(int size);

  public abstract PactDslJsonArray eachArrayWithMaxLike(String name, int numberExamples, int size);

  public abstract PactDslJsonArray eachArrayWithMaxLike(int numberExamples, int size);

  public abstract PactDslJsonArray eachArrayWithMinLike(String name, int size);

  public abstract PactDslJsonArray eachArrayWithMinLike(int size);

  public abstract PactDslJsonArray eachArrayWithMinLike(String name, int numberExamples, int size);

  public abstract PactDslJsonArray eachArrayWithMinLike(int numberExamples, int size);

  public abstract PactDslJsonArray eachArrayWithMinMaxLike(String name, int minSize, int maxSize);

  public abstract PactDslJsonArray eachArrayWithMinMaxLike(int minSize, int maxSize);

  public abstract PactDslJsonArray eachArrayWithMinMaxLike(String name, int numberExamples, int minSize, int maxSize);

  public abstract PactDslJsonArray eachArrayWithMinMaxLike(int numberExamples, int minSize, int maxSize);

  public abstract PactDslJsonBody object(String name);

  public abstract PactDslJsonBody object();

  public abstract DslPart closeObject();

  public abstract DslPart close();

  public abstract DslPart matchUrl(String name, String basePath, Object... pathFragments);

  public abstract DslPart matchUrl(String basePath, Object... pathFragments);

  public abstract DslPart matchUrl2(String name, Object... pathFragments);

  public abstract DslPart matchUrl2(Object... pathFragments);

  public abstract DslPart arrayContaining(String name);

  public PactDslJsonBody asBody() {
    return (PactDslJsonBody) this;
  }

  public PactDslJsonArray asArray() {
    return (PactDslJsonArray) this;
  }

  /** The example as pact-jvm writes it: each object's members in key order. */
  @Override
  public String toString() {
    return Json.sortedString(getBody());
  }

  protected RegexMatcher regexp(String regex) {
    return new RegexMatcher(regex, null);
  }

  protected TimestampMatcher matchTimestamp(String format) {
    return new TimestampMatcher(format);
  }

  protected DateMatcher matchDate(String format) {
    return new DateMatcher(format);
  }

  protected TimeMatcher matchTime(String format) {
    return new TimeMatcher(format);
  }

  protected MinTypeMatcher matchMin(int min) {
    return new MinTypeMatcher(min);
  }

  protected MaxTypeMatcher matchMax(int max) {
    return new MaxTypeMatcher(max);
  }

  protected MinMaxTypeMatcher matchMinMax(int minSize, int maxSize) {
    return new MinMaxTypeMatcher(minSize, maxSize);
  }

  protected EqualsIgnoreOrderMatcher matchIgnoreOrder() {
    return EqualsIgnoreOrderMatcher.INSTANCE;
  }

  protected MinEqualsIgnoreOrderMatcher matchMinIgnoreOrder(int min) {
    return new MinEqualsIgnoreOrderMatcher(min);
  }

  protected MaxEqualsIgnoreOrderMatcher matchMaxIgnoreOrder(int max) {
    return new MaxEqualsIgnoreOrderMatcher(max);
  }

  protected MinMaxEqualsIgnoreOrderMatcher matchMinMaxIgnoreOrder(int minSize, int maxSize) {
    return new MinMaxEqualsIgnoreOrderMatcher(minSize, maxSize);
  }

  protected IncludeMatcher includesMatcher(Object value) {
    return new IncludeMatcher(value.toString());
  }

  public static RegexMatcher regex(String regex, String example) {
    return new RegexMatcher(regex, example);
  }

  public static RegexMatcher regex(String regex) {
    return new RegexMatcher(regex, null);
  }

  // ----------------------------------------------------------------------------------------------
  // What pact-jvm's JsonValue does, over Jackson's tree.

  /** A number written as pact-jvm writes it: its own text. */
  static JsonNode jsonNumber(Object value) {
    return Json.NODES.rawValueNode(new RawValue(value.toString()));
  }

  static JsonNode jsonString(String value) {
    return value == null ? Json.NODES.nullNode() : Json.NODES.textNode(value);
  }

  static JsonNode jsonBool(boolean value) {
    return Json.NODES.booleanNode(value);
  }

  /** A value as pact-jvm's {@code Json.toJson} makes it a JSON value. */
  static JsonNode toJson(Object value) {
    if (value == null) {
      return Json.NODES.nullNode();
    }
    if (value instanceof DslPart part) {
      return part.getBody();
    }
    if (value instanceof BigDecimal || value instanceof BigInteger || value instanceof Number) {
      return jsonNumber(value);
    }
    return Json.toJson(value);
  }

  /** {@code values} repeated until there are {@code size} of them, as pact-jvm's {@code padTo} does. */
  static <T> List<T> padTo(List<T> values, int size) {
    if (values.isEmpty() || values.size() >= size) {
      return values;
    }
    List<T> padded = new ArrayList<>();
    while (padded.size() < size) {
      padded.addAll(values);
    }
    return padded.subList(0, size);
  }

  /** The type a provider-state expression's example is, as pact-jvm's {@code DataType.from} reads it. */
  static DataType dataType(Object example) {
    if (example instanceof Integer || example instanceof Long || example instanceof BigInteger) {
      return DataType.INTEGER;
    }
    if (example instanceof Float || example instanceof Double) {
      return DataType.FLOAT;
    }
    if (example instanceof BigDecimal) {
      return DataType.DECIMAL;
    }
    if (example instanceof String) {
      return DataType.STRING;
    }
    if (example instanceof Boolean) {
      return DataType.BOOLEAN;
    }
    return DataType.RAW;
  }

  /** pact-jvm's {@code UrlMatcherSupport}: the example URL and the pattern a URL with these fragments has. */
  static String[] urlMatcher(String basePath, Object... pathFragments) {
    StringBuilder example = new StringBuilder(basePath == null ? "http://localhost:8080" : basePath);
    StringBuilder regex = new StringBuilder(".*\\/(");
    for (int i = 0; i < pathFragments.length; i++) {
      Object fragment = pathFragments[i];
      example.append('/').append(fragment instanceof RegexMatcher matcher ? matcher.getExample() : fragment.toString());
      if (i > 0) {
        regex.append("\\/");
      }
      regex.append(fragment instanceof RegexMatcher matcher ? matcher.getRegex() : Pattern.quote(fragment.toString()));
    }
    regex.append(")$");
    return new String[] {example.toString(), regex.toString()};
  }
}
