package io.pact.janus.sdk;

import io.pact.janus.bindings.shape.v1.Shape;
import io.pact.janus.bindings.shape.v1.Vocabulary.ShapeShape;
import java.math.BigDecimal;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.regex.Pattern;

/**
 * The shape helpers — one static method per shape primitive of the behavioural specification,
 * meant to be imported with {@code import static io.pact.janus.sdk.Shapes.*}. Each builds exactly
 * the shape-language node its primitive names and nothing else: no validation the engine already
 * does, no inference, no variant dimensions (the engine derives those from the nodes).
 */
public final class Shapes {
  private Shapes() {}

  // ---------------------------------------------------------------------------------------------
  // json

  /**
   * {@code json}: marks a body as a JSON document. Compiles {@code document} by the literal rules
   * and adds nothing — a body written without it compiles identically.
   */
  public static ShapeNode json(Object document) {
    return new ShapeNode(Literals.compile(document, "json(...)"));
  }

  /**
   * {@code content}: a body of {@code mediaType} — {@code text/csv}, {@code application/xml},
   * whatever a declared component handles. {@code document} is compiled by the literal rules as the
   * body's shape, and the interaction declares the type for that slot (contract spec §5.5). The type
   * is written as given; whether anything handles it is the engine's answer, at {@code execute}.
   * Accepted only as a request or response body.
   */
  public static Content content(String mediaType, Object document) {
    return new Content(mediaType, document);
  }

  // ---------------------------------------------------------------------------------------------
  // kind predicates

  /** {@code integer}: a whole-number value, with {@code example} as its example. */
  public static ShapeNode integer(long example) {
    return kind(ShapeShape.INTEGER, example);
  }

  /** {@code number}: any numeric value. */
  public static ShapeNode number(Number example) {
    return kind(ShapeShape.NUMBER, Objects.requireNonNull(example, "number(example)"));
  }

  /** {@code decimal}: a number written with a fractional part. */
  public static ShapeNode decimal(double example) {
    return kind(ShapeShape.DECIMAL, example);
  }

  /** {@code decimal}, with an example whose exact decimal spelling matters. */
  public static ShapeNode decimal(BigDecimal example) {
    return kind(ShapeShape.DECIMAL, Objects.requireNonNull(example, "decimal(example)"));
  }

  /** {@code string}: a string value. */
  public static ShapeNode string(String example) {
    return kind(ShapeShape.STRING, Objects.requireNonNull(example, "string(example)"));
  }

  /**
   * {@code boolean}: a boolean value. Spelled {@code bool} because {@code boolean} is a Java
   * keyword (STYLE.md, deviations).
   */
  public static ShapeNode bool(boolean example) {
    return kind(ShapeShape.BOOLEAN, example);
  }

  private static ShapeNode kind(String operator, Object example) {
    Shape shape = Literals.node(operator);
    Literals.example(shape, example, operator + "(...)");
    return new ShapeNode(shape);
  }

  // ---------------------------------------------------------------------------------------------
  // dates and times

  /** {@code datetime} with no {@code format}: ISO-8601. No format is inferred from the example. */
  public static ShapeNode datetime(String example) {
    return temporal(ShapeShape.DATETIME, example, null);
  }

  /** {@code datetime} with {@code format}, in shape spec §4.2's pattern language. */
  public static ShapeNode datetime(String example, String format) {
    return temporal(ShapeShape.DATETIME, example, Objects.requireNonNull(format, "datetime(example, format)"));
  }

  /** {@code date} with no {@code format}: ISO-8601. */
  public static ShapeNode date(String example) {
    return temporal(ShapeShape.DATE, example, null);
  }

  /** {@code date} with {@code format}. */
  public static ShapeNode date(String example, String format) {
    return temporal(ShapeShape.DATE, example, Objects.requireNonNull(format, "date(example, format)"));
  }

  /** {@code time} with no {@code format}: ISO-8601. */
  public static ShapeNode time(String example) {
    return temporal(ShapeShape.TIME, example, null);
  }

  /** {@code time} with {@code format}. */
  public static ShapeNode time(String example, String format) {
    return temporal(ShapeShape.TIME, example, Objects.requireNonNull(format, "time(example, format)"));
  }

  private static ShapeNode temporal(String operator, String example, String format) {
    Shape shape = Literals.node(operator);
    Literals.example(shape, Objects.requireNonNull(example, operator + "(example)"), operator + "(...)");
    shape.setFormat(format);
    return new ShapeNode(shape);
  }

  // ---------------------------------------------------------------------------------------------
  // regex

  /**
   * {@code regex}: a string the pattern matches — anywhere in it, since matching is unanchored
   * (shape spec §4.2). The pattern text is sent verbatim; write {@code ^...$} for a full match.
   */
  public static ShapeNode regex(String pattern, String example) {
    Shape shape = Literals.node(ShapeShape.REGEX);
    shape.setPattern(Objects.requireNonNull(pattern, "regex(pattern, example)"));
    Literals.example(shape, Objects.requireNonNull(example, "regex(pattern, example)"), "regex(...)");
    return new ShapeNode(shape);
  }

  /**
   * {@code regex} from a compiled {@link Pattern}. Its text is sent verbatim; a pattern compiled with
   * flags ({@code Pattern.CASE_INSENSITIVE} and the like) is refused, because the flags live outside
   * the pattern text and dropping them would change what it means — write them inline instead
   * ({@code (?i)}).
   */
  public static ShapeNode regex(Pattern pattern, String example) {
    Objects.requireNonNull(pattern, "regex(pattern, example)");
    // Pattern.flags() folds in flags written inline, so the out-of-band ones are those the text
    // alone does not produce.
    if (pattern.flags() != Pattern.compile(pattern.pattern()).flags()) {
      throw new IllegalArgumentException(
          "regex(" + pattern + ") was compiled with flags (" + pattern.flags() + ") the pattern text cannot"
              + " carry; write them inline, e.g. (?i), or pass the pattern as a string");
    }
    return regex(pattern.pattern(), example);
  }

  // ---------------------------------------------------------------------------------------------
  // any-of, one-of

  /**
   * {@code anyOf}: the literal values a field may take, in the order given; the first is the
   * example. The options are values, never shapes.
   */
  public static ShapeNode anyOf(Object... options) {
    // anyOf(null) arrives as a null array: one option, null.
    return anyOf(options == null ? Arrays.asList((Object) null) : Arrays.asList(options));
  }

  /** {@code anyOf} over a list of options. */
  public static ShapeNode anyOf(List<?> options) {
    Objects.requireNonNull(options, "anyOf(options)");
    List<Object> values = new ArrayList<>(options.size());
    for (int i = 0; i < options.size(); i++) {
      values.add(Json.value(options.get(i), "anyOf option " + i));
    }
    Shape shape = Literals.node(ShapeShape.ANY_OF);
    shape.setOptions(values);
    if (!values.isEmpty()) {
      Literals.example(shape, values.get(0), "anyOf(...)");
    }
    return new ShapeNode(shape);
  }

  /**
   * {@code oneOf}: a discriminated union. Each alternative — a plain map binding the discriminator to
   * a literal — is compiled by the literal rules, in the order given. No {@code default} is written,
   * and the alternatives are not checked: the engine validates them when the interaction is added.
   */
  public static ShapeNode oneOf(String discriminator, Map<String, ?> alternatives) {
    Objects.requireNonNull(discriminator, "oneOf(discriminator, alternatives)");
    Objects.requireNonNull(alternatives, "oneOf(discriminator, alternatives)");
    Map<String, Shape> compiled = new LinkedHashMap<>();
    for (Map.Entry<String, Object> e : Json.entries(alternatives, "oneOf alternatives")) {
      compiled.put(e.getKey(), Literals.compile(e.getValue(), "oneOf alternative '" + e.getKey() + "'"));
    }
    Shape shape = Literals.node(ShapeShape.ONE_OF);
    shape.setDiscriminator(discriminator);
    shape.setAlternatives(compiled);
    return new ShapeNode(shape);
  }

  // ---------------------------------------------------------------------------------------------
  // optional, nullable, forbidden, each-like

  /** {@code optional}: may be absent. Wraps {@code of} compiled by the literal rules. */
  public static ShapeNode optional(Object of) {
    return wrapper(ShapeShape.OPTIONAL, of);
  }

  /** {@code nullable}: may be null. Wraps {@code of} compiled by the literal rules. */
  public static ShapeNode nullable(Object of) {
    return wrapper(ShapeShape.NULLABLE, of);
  }

  /**
   * {@code forbidden}: the member must be absent. It takes no nested shape and no example — a
   * constraint on a value that must not exist is not a thing to write — so it takes no arguments.
   *
   * <p>Where it may be written is not checked here: a {@code forbidden} node belongs in a slot and
   * may not be wrapped by {@code optional} or {@code nullable} (shape spec §5.1, §5.2), and the
   * engine refuses a misplaced one with {@code interaction-invalid} when the interaction is added.
   */
  public static ShapeNode forbidden() {
    return new ShapeNode(Literals.node(ShapeShape.FORBIDDEN));
  }

  private static ShapeNode wrapper(String operator, Object of) {
    Shape shape = Literals.node(operator);
    shape.setOf(Literals.compile(of, operator + "(...)"));
    return new ShapeNode(shape);
  }

  /** {@code eachLike} with no bounds given. */
  public static ShapeNode eachLike(Object items) {
    return eachLike(items, Cardinality.unspecified());
  }

  /**
   * {@code eachLike}: an array of like elements; {@code min} and {@code max} are written only when
   * given.
   */
  public static ShapeNode eachLike(Object items, Cardinality cardinality) {
    Objects.requireNonNull(cardinality, "eachLike(items, cardinality)");
    Shape shape = Literals.node(ShapeShape.EACH_LIKE);
    shape.setItems(Literals.compile(items, "eachLike(...)"));
    shape.setMin(cardinality.minOrNull());
    shape.setMax(cardinality.maxOrNull());
    return new ShapeNode(shape);
  }

  // ---------------------------------------------------------------------------------------------
  // plain data

  /**
   * A plain, ordered map — Java's missing map literal. Not a shape helper: its result is an
   * ordinary {@code Map} and is compiled by the literal rules like any other, but it keeps the order
   * the members are written in (which {@code Map.of} does not) and admits {@code null} values
   * (which {@code Map.of} rejects). Arguments alternate name, value.
   */
  public static Map<String, Object> map(Object... namesAndValues) {
    if (namesAndValues.length % 2 != 0) {
      throw new IllegalArgumentException("map(...) takes name, value pairs; got an odd number of arguments");
    }
    Map<String, Object> map = new LinkedHashMap<>();
    for (int i = 0; i < namesAndValues.length; i += 2) {
      if (!(namesAndValues[i] instanceof String name)) {
        throw new IllegalArgumentException("map(...) argument " + i + " must be a member name (a String)");
      }
      if (map.containsKey(name)) {
        throw new IllegalArgumentException("map(...) names '" + name + "' twice");
      }
      map.put(name, namesAndValues[i + 1]);
    }
    return map;
  }
}
