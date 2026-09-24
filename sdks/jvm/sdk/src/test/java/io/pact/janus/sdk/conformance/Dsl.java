package io.pact.janus.sdk.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import io.pact.janus.sdk.Cardinality;
import io.pact.janus.sdk.Interaction;
import io.pact.janus.sdk.Janus;
import io.pact.janus.sdk.Shapes;
import io.pact.janus.sdk.VariantBinding;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * A case's DSL usage, replayed through this SDK's own surface. This is the whole of what makes the
 * suite language-independent: a case names a behavioural-spec primitive and its arguments, and each
 * language's driver knows only how that primitive is spelled here — {@code each-like} is
 * {@code Shapes.eachLike} on the JVM and {@code eachLike} in TypeScript. Nothing in this file
 * decides what a primitive <em>means</em>.
 */
final class Dsl {

  private Dsl() {}

  /** A case's template: plain JSON as written, an object carrying {@code $} as the helper call it names. */
  static Object template(JsonNode node) {
    if (node == null || node.isNull()) {
      return null;
    }
    if (node.isArray()) {
      List<Object> items = new ArrayList<>();
      node.forEach(item -> items.add(template(item)));
      return items;
    }
    if (isCall(node)) {
      return call(node);
    }
    if (node.isObject()) {
      // Shapes.map is this SDK's ordered map literal (STYLE.md, deviations): the literal rule
      // writes members in the order the author wrote them, and Map.of would not keep it.
      List<Object> flattened = new ArrayList<>();
      node.properties().forEach(member -> {
        flattened.add(member.getKey());
        flattened.add(template(member.getValue()));
      });
      return Shapes.map(flattened.toArray());
    }
    if (node.isTextual()) {
      return node.asText();
    }
    if (node.isBoolean()) {
      return node.booleanValue();
    }
    if (node.isNumber()) {
      return node.numberValue();
    }
    throw new IllegalArgumentException("a template this driver does not read: " + node);
  }

  /** The shape helper a call names, spelled the JVM way. */
  static Object call(JsonNode node) {
    String primitive = node.path("$").asText();
    JsonNode args = node.path("args");
    JsonNode options = node.path("options");
    return switch (primitive) {
      case "literal" -> template(args.path(0));
      case "json" -> Shapes.json(template(args.path(0)));
      // Not a shape — it names a whole slot — but a case writes it where a body goes, as a call.
      // The SDK decides where it is accepted; this only spells it.
      case "content" -> Shapes.content(args.path(0).asText(), template(args.path(1)));
      case "integer" -> Shapes.integer(args.path(0).longValue());
      case "number" -> Shapes.number(args.path(0).numberValue());
      case "decimal" -> Shapes.decimal(args.path(0).decimalValue());
      case "string" -> Shapes.string(args.path(0).asText());
      case "boolean" -> Shapes.bool(args.path(0).booleanValue());
      case "datetime" -> args.size() > 1
          ? Shapes.datetime(args.path(0).asText(), args.path(1).asText())
          : Shapes.datetime(args.path(0).asText());
      case "date" -> args.size() > 1
          ? Shapes.date(args.path(0).asText(), args.path(1).asText())
          : Shapes.date(args.path(0).asText());
      case "time" -> args.size() > 1
          ? Shapes.time(args.path(0).asText(), args.path(1).asText())
          : Shapes.time(args.path(0).asText());
      case "regex" -> Shapes.regex(args.path(0).asText(), args.path(1).asText());
      case "any-of" -> Shapes.anyOf(literals(args));
      case "one-of" -> Shapes.oneOf(args.path(0).asText(), alternatives(args.path(1)));
      case "optional" -> Shapes.optional(template(args.path(0)));
      case "nullable" -> Shapes.nullable(template(args.path(0)));
      case "forbidden" -> Shapes.forbidden();
      case "each-like" -> Shapes.eachLike(template(args.path(0)), cardinality(options));
      default -> throw new IllegalArgumentException(
          "the case names a shape primitive this SDK does not implement: '" + primitive + "'");
    };
  }

  /** The chain a case scripts, built with this SDK's builder, one call per member. */
  static Interaction interaction(Janus janus, JsonNode script) {
    Interaction interaction = janus.interaction(script.path("description").asText());
    for (JsonNode state : script.path("given")) {
      interaction = state.has("params")
          ? interaction.given(state.path("name").asText(), literalMap(state.path("params")))
          : interaction.given(state.path("name").asText());
    }
    JsonNode request = script.path("request");
    if (request.isObject()) {
      interaction = interaction.request(parts -> {
        if (request.has("method")) {
          parts.method(template(request.get("method")));
        }
        if (request.has("path")) {
          parts.path(template(request.get("path")));
        }
        request.path("query").properties().forEach(q -> parts.query(q.getKey(), multiValue(q.getValue())));
        request.path("headers").properties().forEach(h -> parts.header(h.getKey(), multiValue(h.getValue())));
        if (request.has("body")) {
          parts.body(template(request.get("body")));
        }
      });
    }
    JsonNode response = script.path("response");
    if (response.isObject()) {
      interaction = interaction.response(parts -> {
        if (response.has("status")) {
          parts.status(template(response.get("status")));
        }
        response.path("headers").properties().forEach(h -> parts.header(h.getKey(), multiValue(h.getValue())));
        if (response.has("body")) {
          parts.body(template(response.get("body")));
        }
      });
    }
    return interaction;
  }

  /**
   * A header or query value. Only a helper call is resolved here; everything else is handed to the
   * SDK exactly as the case wrote it — including a value the rule refuses, because deciding that is
   * the SDK's job and a driver that converted first would be answering the case on its behalf.
   */
  private static Object multiValue(JsonNode node) {
    return isCall(node) ? call(node) : plain(node);
  }

  private static Cardinality cardinality(JsonNode options) {
    Cardinality bounds = Cardinality.unspecified();
    if (options.has("min")) {
      bounds = bounds.withMin(options.path("min").longValue());
    }
    if (options.has("max")) {
      bounds = bounds.withMax(options.path("max").longValue());
    }
    return bounds;
  }

  /** {@code any-of}'s options: literal values, never templates. */
  private static List<Object> literals(JsonNode args) {
    List<Object> options = new ArrayList<>();
    args.forEach(option -> options.add(option.isNull() ? null
        : option.isTextual() ? option.asText()
        : option.isBoolean() ? option.booleanValue()
        : option.numberValue()));
    return options;
  }

  /** {@code one-of}'s alternatives: name -> a plain map, each member compiled by the literal rules. */
  private static Map<String, Object> alternatives(JsonNode node) {
    Map<String, Object> alternatives = new LinkedHashMap<>();
    node.properties().forEach(member -> alternatives.put(member.getKey(), template(member.getValue())));
    return alternatives;
  }

  /**
   * A state's params: plain data, passed through as written, with every binding call resolved
   * wherever it appears — a nested one too, because refusing it is the SDK's job, not this driver's.
   */
  private static Map<String, Object> literalMap(JsonNode node) {
    Map<String, Object> params = new LinkedHashMap<>();
    node.properties().forEach(member -> params.put(member.getKey(), plain(member.getValue())));
    return params;
  }

  /** A binding call, spelled the JVM way: {@code when-variant} and {@code variant-cases}. */
  private static VariantBinding binding(JsonNode node) {
    JsonNode args = node.path("args");
    String primitive = node.path("$").asText();
    return switch (primitive) {
      case "when-variant" -> VariantBinding.whenVariant(args.path(0).asText(), args.path(1).asText());
      case "variant-cases" -> args.size() > 2
          ? VariantBinding.variantCases(args.path(0).asText(), literalMap(args.path(1)), plain(args.path(2)))
          : VariantBinding.variantCases(args.path(0).asText(), literalMap(args.path(1)));
      default -> throw new IllegalArgumentException(
          "the case names a binding primitive this SDK does not implement: '" + primitive + "'");
    };
  }

  private static Object plain(JsonNode node) {
    if (isCall(node)) {
      return binding(node);
    }
    if (node.isObject()) {
      return literalMap(node);
    }
    if (node.isArray()) {
      List<Object> items = new ArrayList<>();
      node.forEach(item -> items.add(plain(item)));
      return items;
    }
    return node.isNull() ? null
        : node.isTextual() ? node.asText()
        : node.isBoolean() ? node.booleanValue()
        : (Object) node.numberValue();
  }

  private static boolean isCall(JsonNode node) {
    return node.isObject() && node.path("$").isTextual();
  }
}
