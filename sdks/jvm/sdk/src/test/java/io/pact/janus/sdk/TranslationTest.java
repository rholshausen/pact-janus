package io.pact.janus.sdk;

import static io.pact.janus.sdk.Shapes.anyOf;
import static io.pact.janus.sdk.Shapes.bool;
import static io.pact.janus.sdk.Shapes.date;
import static io.pact.janus.sdk.Shapes.datetime;
import static io.pact.janus.sdk.Shapes.decimal;
import static io.pact.janus.sdk.Shapes.eachLike;
import static io.pact.janus.sdk.Shapes.integer;
import static io.pact.janus.sdk.Shapes.json;
import static io.pact.janus.sdk.Shapes.map;
import static io.pact.janus.sdk.Shapes.nullable;
import static io.pact.janus.sdk.Shapes.number;
import static io.pact.janus.sdk.Shapes.oneOf;
import static io.pact.janus.sdk.Shapes.optional;
import static io.pact.janus.sdk.Shapes.regex;
import static io.pact.janus.sdk.Shapes.string;
import static io.pact.janus.sdk.Shapes.time;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.math.BigDecimal;
import java.util.ArrayList;
import java.time.Instant;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

/**
 * SDK spec §7 category 1 — DSL → interaction-spec translation. Each test names the
 * behavioural-spec conformance ids it covers. Documents are compared as JSON trees (member order
 * aside, which the ordering tests check separately).
 */
class TranslationTest {
  private static final ObjectMapper MAPPER = new ObjectMapper();
  private final Janus janus = Janus.of("web-app", "orders-api");

  /** The value as written to the wire and read back, so an int and a long holding 1 compare equal. */
  static JsonNode tree(Object value) {
    try {
      return MAPPER.readTree(MAPPER.writeValueAsString(value instanceof ShapeNode n ? n.toDocument() : value));
    } catch (Exception e) {
      throw new IllegalArgumentException(e);
    }
  }

  static JsonNode parse(String text) {
    try {
      return MAPPER.readTree(text.replace('\'', '"'));
    } catch (Exception e) {
      throw new IllegalArgumentException(text, e);
    }
  }

  static void assertShape(String expected, Object actual) {
    assertEquals(parse(expected), tree(actual));
  }

  @Nested
  @DisplayName("the RFC's consumer example")
  class RfcExample {
    @Test
    @DisplayName("compiles to the canonical order interaction [session.interaction.description-set, session.interaction.http-passive-transport, session.given.appends-state, session.request.bare-value-is-equality, session.response.bare-value-is-equality, shape.literal.helper-used-as-authored]")
    void theOrderInteraction() {
      Interaction getOrder = janus.interaction("get an order")
          .given("an order exists", Map.of("id", "42"))
          .request(r -> r.method("GET").path("/orders/42"))
          .response(r -> r
              .status(200)
              .body(json(map(
                  "id", integer(42),
                  "status", anyOf("PENDING", "SHIPPED", "DELIVERED"),
                  "shippedAt", optional(datetime("2026-07-30T10:00:00Z")),
                  "payment", oneOf("type", map(
                      "card", map("type", "card", "last4", regex("\\d{4}", "1234")),
                      "invoice", map("type", "invoice", "dueDate", date("2026-08-30")))),
                  "items", eachLike(map("sku", string("SKU-1"), "qty", integer(1)), Cardinality.min(1))))));

      assertEquals(parse("""
          { 'description': 'get an order',
            'transport': { 'kind': 'http', 'mode': 'passive' },
            'states': [ { 'name': 'an order exists', 'params': { 'id': '42' } } ],
            'parts': {
              'request': { 'method': { 'shape': 'equality', 'example': 'GET' },
                           'path': { 'shape': 'equality', 'example': '/orders/42' } },
              'response': {
                'status': { 'shape': 'equality', 'example': 200 },
                'body': { 'shape': 'object', 'members': {
                  'id': { 'shape': 'integer', 'example': 42 },
                  'status': { 'shape': 'any-of', 'options': ['PENDING', 'SHIPPED', 'DELIVERED'], 'example': 'PENDING' },
                  'shippedAt': { 'shape': 'optional', 'of': { 'shape': 'datetime', 'example': '2026-07-30T10:00:00Z' } },
                  'payment': { 'shape': 'one-of', 'discriminator': 'type', 'alternatives': {
                    'card': { 'shape': 'object', 'members': {
                      'type': { 'shape': 'equality', 'example': 'card' },
                      'last4': { 'shape': 'regex', 'pattern': '\\\\d{4}', 'example': '1234' } } },
                    'invoice': { 'shape': 'object', 'members': {
                      'type': { 'shape': 'equality', 'example': 'invoice' },
                      'dueDate': { 'shape': 'date', 'example': '2026-08-30' } } } } },
                  'items': { 'shape': 'each-like', 'min': 1, 'items': { 'shape': 'object', 'members': {
                    'sku': { 'shape': 'string', 'example': 'SKU-1' },
                    'qty': { 'shape': 'integer', 'example': 1 } } } } } } } } }"""),
          tree(getOrder.toDocument()));
    }

    @Test
    @DisplayName("building the same chain twice produces two equal documents [session.interaction.no-call-until-execute]")
    void buildingTwiceIsEqual() {
      Interaction a = janus.interaction("x").given("s").request(r -> r.method("GET").path("/a"));
      assertEquals(a.toDocument(), a.toDocument());
      assertEquals(a.toDocument(), janus.interaction("x").given("s").request(r -> r.method("GET").path("/a")).toDocument());
    }
  }

  @Nested
  @DisplayName("interaction and given")
  class InteractionAndGiven {
    @Test
    @DisplayName("an interaction is its description, an HTTP passive transport, and parts [session.interaction.description-set, session.interaction.http-passive-transport]")
    void bareInteraction() {
      assertEquals(parse("{ 'description': 'nothing yet', 'transport': { 'kind': 'http', 'mode': 'passive' }, 'parts': {} }"),
          tree(janus.interaction("nothing yet").toDocument()));
    }

    @Test
    @DisplayName("given appends states in call order, params omitted when not given [session.given.appends-state, session.given.multiple-states, session.given.params-omitted-when-absent]")
    void statesInOrder() {
      JsonNode states = tree(janus.interaction("x")
          .given("first")
          .given("second", map("b", 2, "a", List.of(1, "x"), "n", null))
          .given("third", Map.of())
          .toDocument()).get("states");
      assertEquals(parse("[ { 'name': 'first' }, { 'name': 'second', 'params': { 'b': 2, 'a': [1, 'x'], 'n': null } },"
          + " { 'name': 'third', 'params': {} } ]"), states);
      assertFalse(states.get(0).has("params"));
      // passed through as written: no key renamed, reordered or coerced
      assertEquals(List.of("b", "a", "n"), fieldNames(states.get(1).get("params")));
    }
  }

  @Nested
  @DisplayName("request and response")
  class RequestAndResponse {
    @Test
    @DisplayName("bare values are equality nodes; method and path are not repaired [session.request.bare-value-is-equality, session.response.bare-value-is-equality]")
    void bareValues() {
      JsonNode parts = tree(janus.interaction("x")
          .request(r -> r.method("get").path("/orders//42/").body(map("a", 1)))
          .response(r -> r.status(201).body("done"))
          .toDocument()).get("parts");
      assertEquals(parse("""
          { 'request': { 'method': { 'shape': 'equality', 'example': 'get' },
                         'path': { 'shape': 'equality', 'example': '/orders//42/' },
                         'body': { 'shape': 'object', 'members': { 'a': { 'shape': 'equality', 'example': 1 } } } },
            'response': { 'status': { 'shape': 'equality', 'example': 201 },
                          'body': { 'shape': 'equality', 'example': 'done' } } }"""), parts);
    }

    @Test
    @DisplayName("a member not given produces no slot, and a shape helper is used as authored [session.request.shape-used-as-authored]")
    void slotsOnlyWhenGiven() {
      JsonNode request = tree(janus.interaction("x")
          .request(r -> r.path(regex("^/orders/\\d+$", "/orders/1")))
          .toDocument()).at("/parts/request");
      assertEquals(parse("{ 'path': { 'shape': 'regex', 'pattern': '^/orders/\\\\d+$', 'example': '/orders/1' } }"), request);
    }

    @Test
    @DisplayName("header names are lower-cased; values follow the name-to-list rule [session.request.header-names-lower-cased, session.response.header-names-lower-cased, session.request.multi-value-slots]")
    void headers() {
      JsonNode parts = tree(janus.interaction("x")
          .request(r -> r.header("Accept", "application/json")
              .header("X-Many", List.of("a", "b"))
              .header("X-Trace-ID", regex("^[0-9a-f]+$", "abc123")))
          .response(r -> r.headers(map("Content-Type", "application/json")))
          .toDocument()).get("parts");
      assertEquals(parse("""
          { 'request': { 'headers': { 'shape': 'object', 'members': {
                'accept': { 'shape': 'equality', 'example': ['application/json'] },
                'x-many': { 'shape': 'equality', 'example': ['a', 'b'] },
                'x-trace-id': { 'shape': 'each-like', 'items': { 'shape': 'regex', 'pattern': '^[0-9a-f]+$', 'example': 'abc123' }, 'min': 1, 'max': 1 } } } },
            'response': { 'headers': { 'shape': 'object', 'members': {
                'content-type': { 'shape': 'equality', 'example': ['application/json'] } } } } }"""), parts);
    }

    @Test
    @DisplayName("query names are kept as written; values follow the name-to-list rule [session.request.query-names-as-written, session.request.multi-value-slots]")
    void query() {
      JsonNode query = tree(janus.interaction("x")
          .request(r -> r.query("Page", "1").query("tag", List.of("a", "b")).query("q", string("shoes")))
          .toDocument()).at("/parts/request/query");
      assertEquals(parse("""
          { 'shape': 'object', 'members': {
              'Page': { 'shape': 'equality', 'example': ['1'] },
              'tag': { 'shape': 'equality', 'example': ['a', 'b'] },
              'q': { 'shape': 'each-like', 'items': { 'shape': 'string', 'example': 'shoes' }, 'min': 1, 'max': 1 } } }"""), query);
    }

    @Test
    @DisplayName("a header declared twice under different case is refused rather than silently merged")
    void headerCollision() {
      assertThrows(IllegalArgumentException.class,
          () -> janus.interaction("x").request(r -> r.header("Accept", "a").header("accept", "b")));
    }

    @Test
    @DisplayName("a whole number or a boolean is written as the one string that spells it "
        + "[session.request.value-written-as-one-string]")
    void spellableValues() {
      JsonNode headers = tree(janus.interaction("x")
          .request(r -> r.header("X-Count", 3).header("X-Whole", 3.0d).header("X-Debug", false).query("page", 2))
          .toDocument()).at("/parts/request/headers");
      assertEquals(parse("""
          { 'shape': 'object', 'members': {
              'x-count': { 'shape': 'equality', 'example': ['3'] },
              'x-whole': { 'shape': 'equality', 'example': ['3'] },
              'x-debug': { 'shape': 'equality', 'example': ['false'] } } }"""), headers);
    }

    @Test
    @DisplayName("a value with no spelling every SDK agrees on is refused "
        + "[session.request.unspellable-value-refused]")
    void unspellableValues() {
      // A fractional number is spelled differently language by language, Long.MAX_VALUE is past the
      // largest integer JavaScript spells exactly, and an Instant has no format the SDK may choose
      // for the author (ADR 0019).
      assertThrows(IllegalArgumentException.class,
          () -> janus.interaction("x").request(r -> r.header("X-Ratio", 1.5)));
      assertThrows(IllegalArgumentException.class,
          () -> janus.interaction("x").request(r -> r.header("X-Big", Long.MAX_VALUE)));
      assertThrows(IllegalArgumentException.class,
          () -> janus.interaction("x").request(r -> r.header("X-When", Instant.EPOCH)));
      assertThrows(IllegalArgumentException.class,
          () -> janus.interaction("x").request(r -> r.header("X-Nothing", (Object) null)));
    }
  }

  @Nested
  @DisplayName("literal and json")
  class LiteralAndJson {
    @Test
    @DisplayName("scalars and null are equality nodes [shape.literal.scalar-is-equality]")
    void scalars() {
      assertShape("{ 'shape': 'object', 'members': {"
          + " 's': { 'shape': 'equality', 'example': 'x' },"
          + " 'i': { 'shape': 'equality', 'example': 7 },"
          + " 'd': { 'shape': 'equality', 'example': 1.5 },"
          + " 'b': { 'shape': 'equality', 'example': false },"
          + " 'n': { 'shape': 'equality', 'example': null } } }",
          json(map("s", "x", "i", 7, "d", 1.5, "b", false, "n", null)));
      // an equality node over null still carries its example: it is the operator's parameter
      assertTrue(json((Object) null).toDocument().containsKey("example"));
    }

    @Test
    @DisplayName("a map is an object node, members in the order written [shape.literal.map-is-object]")
    void maps() {
      ShapeNode node = json(map("z", 1, "a", map("y", "q")));
      assertShape("{ 'shape': 'object', 'members': { 'z': { 'shape': 'equality', 'example': 1 },"
          + " 'a': { 'shape': 'object', 'members': { 'y': { 'shape': 'equality', 'example': 'q' } } } } }", node);
      assertEquals(List.of("z", "a"), fieldNames(tree(node).get("members")));
    }

    @Test
    @DisplayName("a map with no encounter order (Map.of) is written in key order, so the same DSL always gives the same document")
    void unorderedMaps() {
      assertEquals(List.of("a", "m", "z"), fieldNames(tree(json(Map.of("z", 1, "a", 2, "m", 3))).get("members")));
    }

    @Test
    @DisplayName("a list is a positional array node [shape.literal.list-is-array]")
    void lists() {
      assertShape("{ 'shape': 'array', 'entries': [ { 'shape': 'equality', 'example': 1 },"
          + " { 'shape': 'integer', 'example': 2 }, { 'shape': 'array', 'entries': [] } ] }",
          json(List.of(1, integer(2), List.of())));
    }

    @Test
    @DisplayName("a helper's result is used as authored, and a plain map with a 'shape' member is still a map [shape.literal.helper-used-as-authored]")
    void helperVersusLookalike() {
      assertShape("{ 'shape': 'object', 'members': {"
          + " 'shape': { 'shape': 'equality', 'example': 'integer' },"
          + " 'example': { 'shape': 'equality', 'example': 42 } } }",
          json(map("shape", "integer", "example", 42)));
      assertShape("{ 'shape': 'integer', 'example': 42 }", json(integer(42)));
    }

    @Test
    @DisplayName("json(...) compiles its document by the literal rules and adds nothing [shape.json.compiles-as-literal]")
    void jsonIsSugar() {
      Interaction with = janus.interaction("x").response(r -> r.body(json(map("a", 1))));
      Interaction without = janus.interaction("x").response(r -> r.body(map("a", 1)));
      assertEquals(with.toDocument(), without.toDocument());
    }

    @Test
    @DisplayName("nothing else is inferred: a value with no JSON reading is refused, not guessed")
    void nothingElseInferred() {
      assertThrows(IllegalArgumentException.class, () -> json(map("when", java.time.Instant.EPOCH)));
      assertThrows(IllegalArgumentException.class, () -> json(Double.NaN));
    }
  }

  @Nested
  @DisplayName("kind predicates, dates and regex")
  class ValueOperators {
    @Test
    @DisplayName("integer, number, decimal, string and boolean are kind predicates with their example [shape.integer.kind-predicate, shape.number.kind-predicate, shape.decimal.kind-predicate, shape.string.kind-predicate, shape.boolean.kind-predicate]")
    void kinds() {
      assertShape("{ 'shape': 'integer', 'example': 42 }", integer(42));
      assertShape("{ 'shape': 'number', 'example': 2.5 }", number(2.5));
      assertShape("{ 'shape': 'decimal', 'example': 12.5 }", decimal(12.5));
      // a BigDecimal example keeps its written scale on the wire
      assertEquals("{\"example\":12.50,\"shape\":\"decimal\"}", decimal(new BigDecimal("12.50")).toString());
      assertShape("{ 'shape': 'string', 'example': 'x' }", string("x"));
      assertShape("{ 'shape': 'boolean', 'example': true }", bool(true));
    }

    @Test
    @DisplayName("datetime carries format only when given, and infers none [shape.datetime.format-when-given, shape.datetime.default-is-iso8601]")
    void datetimes() {
      assertShape("{ 'shape': 'datetime', 'example': '2026-07-30T10:00:00Z' }", datetime("2026-07-30T10:00:00Z"));
      assertShape("{ 'shape': 'datetime', 'example': '30/07/2026 10:00', 'format': 'dd/MM/yyyy HH:mm' }",
          datetime("30/07/2026 10:00", "dd/MM/yyyy HH:mm"));
    }

    @Test
    @DisplayName("date and time are treated like datetime [shape.date.format-when-given, shape.time.format-when-given]")
    void datesAndTimes() {
      assertShape("{ 'shape': 'date', 'example': '2026-08-30' }", date("2026-08-30"));
      assertShape("{ 'shape': 'date', 'example': '30.08.2026', 'format': 'dd.MM.yyyy' }", date("30.08.2026", "dd.MM.yyyy"));
      assertShape("{ 'shape': 'time', 'example': '10:00:00' }", time("10:00:00"));
      assertShape("{ 'shape': 'time', 'example': '10:00', 'format': 'HH:mm' }", time("10:00", "HH:mm"));
    }

    @Test
    @DisplayName("regex sends its pattern verbatim, never anchored [shape.regex.unanchored, shape.regex.pattern-verbatim]")
    void regexes() {
      assertShape("{ 'shape': 'regex', 'pattern': '\\\\d{4}', 'example': '1234' }", regex("\\d{4}", "1234"));
      assertShape("{ 'shape': 'regex', 'pattern': '(?i)^abc$', 'example': 'ABC' }", regex(Pattern.compile("(?i)^abc$"), "ABC"));
    }

    @Test
    @DisplayName("a Pattern compiled with out-of-band flags is refused at the call")
    void regexFlagsRefused() {
      assertThrows(IllegalArgumentException.class,
          () -> regex(Pattern.compile("abc", Pattern.CASE_INSENSITIVE), "ABC"));
    }
  }

  @Nested
  @DisplayName("any-of, one-of, optional, nullable, each-like")
  class Structure {
    @Test
    @DisplayName("any-of lists its options in order, the first as example [shape.any-of.literal-containment]")
    void anyOfOptions() {
      assertShape("{ 'shape': 'any-of', 'options': ['B', 'A', 3, null], 'example': 'B' }", anyOf("B", "A", 3, null));
      assertShape("{ 'shape': 'any-of', 'options': [null], 'example': null }", anyOf((Object) null));
      assertShape("{ 'shape': 'any-of', 'options': [ { 'k': 1 } ], 'example': { 'k': 1 } }", anyOf(List.of(map("k", 1))));
    }

    @Test
    @DisplayName("any-of options are values, not shapes")
    void anyOfRejectsShapes() {
      assertThrows(IllegalArgumentException.class, () -> anyOf(integer(1), 2));
    }

    @Test
    @DisplayName("one-of compiles each alternative by the literal rules, in order, with no default and no checks [shape.one-of.discriminator-binding]")
    void oneOfAlternatives() {
      ShapeNode node = oneOf("kind", map(
          "b", map("kind", "b", "x", integer(1)),
          "a", map("kind", "a")));
      assertShape("{ 'shape': 'one-of', 'discriminator': 'kind', 'alternatives': {"
          + " 'b': { 'shape': 'object', 'members': { 'kind': { 'shape': 'equality', 'example': 'b' }, 'x': { 'shape': 'integer', 'example': 1 } } },"
          + " 'a': { 'shape': 'object', 'members': { 'kind': { 'shape': 'equality', 'example': 'a' } } } } }", node);
      assertEquals(List.of("b", "a"), fieldNames(tree(node).get("alternatives")));
      assertFalse(tree(node).has("default"));
      // duplicate and missing discriminators are the engine's to reject, not the SDK's
      oneOf("kind", map("a", map("kind", "x"), "b", map("kind", "x"), "c", map()));
    }

    @Test
    @DisplayName("optional and nullable wrap any operator compiled by the literal rules [shape.optional.wraps-any-operator, shape.nullable.wraps-any-operator]")
    void wrappers() {
      assertShape("{ 'shape': 'optional', 'of': { 'shape': 'equality', 'example': 'x' } }", optional("x"));
      assertShape("{ 'shape': 'optional', 'of': { 'shape': 'nullable', 'of': { 'shape': 'integer', 'example': 1 } } }",
          optional(nullable(integer(1))));
      assertShape("{ 'shape': 'nullable', 'of': { 'shape': 'object', 'members': { 'a': { 'shape': 'equality', 'example': 1 } } } }",
          nullable(map("a", 1)));
      // the SDK does not police composition (nullable(optional(...)) is the engine's to reject)
      assertShape("{ 'shape': 'nullable', 'of': { 'shape': 'optional', 'of': { 'shape': 'equality', 'example': 1 } } }",
          nullable(optional(1)));
    }

    @Test
    @DisplayName("each-like writes min and max only when given [shape.each-like.bounds-only-when-given]")
    void eachLikeBounds() {
      assertShape("{ 'shape': 'each-like', 'items': { 'shape': 'equality', 'example': 1 } }", eachLike(1));
      assertShape("{ 'shape': 'each-like', 'min': 0, 'items': { 'shape': 'equality', 'example': 1 } }", eachLike(1, Cardinality.min(0)));
      assertShape("{ 'shape': 'each-like', 'max': 3, 'items': { 'shape': 'equality', 'example': 1 } }", eachLike(1, Cardinality.max(3)));
      assertShape("{ 'shape': 'each-like', 'min': 2, 'max': 5, 'items': { 'shape': 'string', 'example': 's' } }",
          eachLike(string("s"), Cardinality.between(2, 5)));
    }
  }

  static List<String> fieldNames(JsonNode node) {
    List<String> names = new ArrayList<>();
    node.fieldNames().forEachRemaining(names::add);
    return names;
  }
}
