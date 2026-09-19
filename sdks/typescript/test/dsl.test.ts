// DSL -> interaction-spec translation (SDK spec §7 category 1). No engine: what a chain builds is
// a document, and these tests read it. Each test names the behavioural-spec conformance ids it
// covers, so "does the suite exercise this primitive?" is answerable by grep.

import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import {
  Janus,
  anyOf,
  boolean,
  date,
  datetime,
  decimal,
  eachLike,
  integer,
  json,
  nullable,
  number,
  oneOf,
  optional,
  regex,
  string,
  time,
} from "../src/index.js";

const janus = new Janus({ consumer: "web-app", provider: "orders-api" });

/** The RFC's own consumer example, as written there (bar the object it hangs off). */
const getOrder = () =>
  janus
    .interaction("get an order")
    .given("an order exists", { id: "42" })
    .request({ method: "GET", path: "/orders/42" })
    .response({
      status: 200,
      body: json({
        id: integer(42),
        status: anyOf("PENDING", "SHIPPED", "DELIVERED"),
        shippedAt: optional(datetime("2026-07-30T10:00:00Z")),
        payment: oneOf("type", {
          card: { type: "card", last4: regex(/\d{4}/, "1234") },
          invoice: { type: "invoice", dueDate: date("2026-08-30") },
        }),
        items: eachLike({ sku: string("SKU-1"), qty: integer(1) }, { min: 1 }),
      }),
    });

describe("the RFC consumer example", () => {
  it("builds the shape-language example's canonical body, and nothing the engine decides", async () => {
    // shape-language examples/order-payload.md §2 is the authority for this body. Two members of
    // that example are not the DSL's to add, so the SDK does not: `datetime`'s 'format' (never
    // inferred from the example — behavioural spec 'datetime') and `one-of`'s 'default' (absent
    // means the first alternative — behavioural spec 'one-of').
    const corpus = JSON.parse(
      await readFile(new URL("../../../corpora/shapes/order-payload/case.json", import.meta.url), "utf8"),
    );
    const expected = structuredClone(corpus.input.spec.parts.response.body);
    delete expected.members.shippedAt.of.format;
    delete expected.members.payment.default;
    delete expected.members.payment.alternatives.invoice.members.dueDate.format;

    const spec = getOrder().build();
    expect(spec.parts.response?.body).toEqual(expected);
  });

  it("describes a passive HTTP interaction with its state and request (session.interaction.*, session.given.appends-state)", () => {
    expect(getOrder().build()).toEqual({
      description: "get an order",
      transport: { kind: "http", mode: "passive" },
      states: [{ name: "an order exists", params: { id: "42" } }],
      parts: {
        request: {
          method: { shape: "equality", example: "GET" },
          path: { shape: "equality", example: "/orders/42" },
        },
        response: expect.objectContaining({ status: { shape: "equality", example: 200 } }),
      },
    });
  });

  it("builds the same document twice from the same chain", () => {
    expect(getOrder().build()).toEqual(getOrder().build());
  });
});

describe("given", () => {
  it("appends states in call order, omitting params not given (session.given.multiple-states, session.given.params-omitted-when-absent)", () => {
    const spec = janus.interaction("x").given("first").given("second", { n: 1 }).build();
    expect(spec.states).toEqual([{ name: "first" }, { name: "second", params: { n: 1 } }]);
  });
});

describe("request and response parts", () => {
  it("compiles a bare value to equality and uses a helper as authored (session.request.bare-value-is-equality, session.request.shape-used-as-authored)", () => {
    const spec = janus
      .interaction("x")
      .request({ method: "get", path: regex("^/orders/\\d+$", "/orders/1") })
      .build();
    expect(spec.parts.request).toEqual({
      // Not upper-cased: repairing a value is the engine's call to reject, never the SDK's to make.
      method: { shape: "equality", example: "get" },
      path: { shape: "regex", pattern: "^/orders/\\d+$", example: "/orders/1" },
    });
  });

  it("lower-cases header names, keeps query names, and carries every value as a list (session.request.header-names-lower-cased, session.request.query-names-as-written, session.request.multi-value-slots)", () => {
    const spec = janus
      .interaction("x")
      .request({
        query: { customerId: "7", status: ["new", "open"] },
        headers: { Accept: "application/json", "X-Trace": regex("^t-", "t-1") },
      })
      .response({ status: 200, headers: { "Content-Type": "application/json" } })
      .build();
    expect(spec.parts.request).toEqual({
      query: {
        shape: "object",
        members: {
          customerId: { shape: "equality", example: ["7"] },
          status: { shape: "equality", example: ["new", "open"] },
        },
      },
      headers: {
        shape: "object",
        members: {
          accept: { shape: "equality", example: ["application/json"] },
          "x-trace": { shape: "each-like", items: { shape: "regex", pattern: "^t-", example: "t-1" } },
        },
      },
    });
    expect(spec.parts.response).toEqual({
      status: { shape: "equality", example: 200 },
      headers: {
        shape: "object",
        members: { "content-type": { shape: "equality", example: ["application/json"] } },
      },
    });
  });

  it("produces no slot for a member not given", () => {
    expect(janus.interaction("x").response({ status: 204 }).build().parts).toEqual({
      response: { status: { shape: "equality", example: 204 } },
    });
  });
});

describe("the literal rule", () => {
  it("compiles scalars to equality, maps to object and lists to array (shape.literal.*)", () => {
    const body = janus
      .interaction("x")
      .response({ body: { a: 1, b: null, c: [true, "s"], d: { shape: "not a helper" } } })
      .build().parts.response?.body;
    expect(body).toEqual({
      shape: "object",
      members: {
        a: { shape: "equality", example: 1 },
        b: { shape: "equality", example: null },
        c: {
          shape: "array",
          entries: [
            { shape: "equality", example: true },
            { shape: "equality", example: "s" },
          ],
        },
        // A plain map with a 'shape' member is still a plain map.
        d: { shape: "object", members: { shape: { shape: "equality", example: "not a helper" } } },
      },
    });
  });

  it("json adds nothing to what it wraps (shape.json.compiles-as-literal)", () => {
    const withJson = janus.interaction("x").response({ body: json({ a: 1 }) }).build();
    const without = janus.interaction("x").response({ body: { a: 1 } }).build();
    expect(withJson).toEqual(without);
  });
});

describe("shape helpers", () => {
  it("map 1:1 to their operators", () => {
    const body = (template: Parameters<typeof json>[0]) =>
      janus.interaction("x").response({ body: template }).build().parts.response?.body;
    expect(body(integer(1))).toEqual({ shape: "integer", example: 1 });
    expect(body(number(1.5))).toEqual({ shape: "number", example: 1.5 });
    expect(body(decimal(1.5))).toEqual({ shape: "decimal", example: 1.5 });
    expect(body(string("s"))).toEqual({ shape: "string", example: "s" });
    expect(body(boolean(true))).toEqual({ shape: "boolean", example: true });
    expect(body(time("10:00:00", "HH:mm:ss"))).toEqual({ shape: "time", example: "10:00:00", format: "HH:mm:ss" });
    expect(body(datetime("2026-07-30T10:00:00Z"))).toEqual({ shape: "datetime", example: "2026-07-30T10:00:00Z" });
    expect(body(nullable(string("s")))).toEqual({ shape: "nullable", of: { shape: "string", example: "s" } });
    expect(body(anyOf(1, 2))).toEqual({ shape: "any-of", options: [1, 2], example: 1 });
    expect(body(eachLike(1, { max: 3 }))).toEqual({
      shape: "each-like",
      items: { shape: "equality", example: 1 },
      max: 3,
    });
  });

  it("keeps a regex unanchored and verbatim, and refuses flags it cannot carry (shape.regex.unanchored, shape.regex.pattern-verbatim)", () => {
    expect(regex(/\d{4}/, "1234").toJSON()).toEqual({ shape: "regex", pattern: "\\d{4}", example: "1234" });
    expect(() => regex(/abc/i, "ABC")).toThrow(/flags \('i'\) cannot be carried/);
  });

  it("writes each-like bounds only when given (shape.each-like.bounds-only-when-given)", () => {
    expect(eachLike(integer(1)).toJSON()).toEqual({ shape: "each-like", items: { shape: "integer", example: 1 } });
  });
});
