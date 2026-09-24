// The interaction builder (behavioural spec 'interaction', 'given', 'request', 'response'): builds
// an interaction-spec document (contract.schema.json#/$defs/InteractionSpec) and makes no protocol
// call — `execute` submits it.

import type { contract, shape } from "./generated/index.js";
import { compile, Content, Shape, type Template } from "./shapes.js";

/** A header or query value the rule can spell: a string, a whole number, or a boolean (ADR 0019). */
export type Spellable = string | number | boolean;

/** A header or query parameter: one value, several values, or a shape its one value must match. */
export type MultiValue = Spellable | readonly Spellable[] | Shape;

export interface RequestParts {
  method?: Template;
  path?: Template;
  query?: { readonly [name: string]: MultiValue };
  headers?: { readonly [name: string]: MultiValue };
  body?: Template | Content;
}

export interface ResponseParts {
  status?: Template;
  headers?: { readonly [name: string]: MultiValue };
  body?: Template | Content;
}

/**
 * A provider-state parameter bound to the running variant (behavioural spec 'when-variant',
 * 'variant-cases'; variant semantics spec §6). It has no name of its own: the `given` params member
 * it is written as names it, and `given` moves it out of `params` into `variant-params`, because a
 * binding written in-band as a parameter value would be indistinguishable from user data.
 */
export class VariantBinding {
  readonly #dimension: string;
  readonly #cases: readonly { point: string; value: unknown }[];
  readonly #fallback: readonly unknown[];

  constructor(dimension: string, cases: readonly { point: string; value: unknown }[], fallback: readonly unknown[]) {
    this.#dimension = dimension;
    this.#cases = cases;
    this.#fallback = fallback;
  }

  /** The binding document (variant-params.schema.json's `Binding`) for the parameter `name`. */
  binding(name: string): { [k: string]: unknown } {
    return {
      name,
      dimension: this.#dimension,
      cases: this.#cases.map((c) => ({ ...c })),
      ...(this.#fallback.length > 0 ? { default: this.#fallback[0] } : {}),
    };
  }
}

/** True exactly when the running variant is at `point` of `dimension` — the RFC's `whenVariant`. */
export function whenVariant(dimension: string, point: string): VariantBinding {
  return new VariantBinding(dimension, [{ point, value: true }], [false]);
}

/**
 * The value the running variant's point for `dimension` selects: `cases` maps point names to values,
 * in the order written. With no `fallback`, a variant at a point no case names leaves the parameter
 * absent (variant semantics spec §6.4) — which is why the argument is optional rather than defaulted.
 */
export function variantCases(
  dimension: string,
  cases: { readonly [point: string]: unknown },
  ...fallback: [] | [unknown]
): VariantBinding {
  const entries = Object.entries(cases);
  if (entries.length === 0) {
    throw new TypeError(`variantCases('${dimension}'): at least one case is needed — with none, it is only ever its default`);
  }
  return new VariantBinding(
    dimension,
    entries.map(([point, value]) => ({ point, value })),
    fallback,
  );
}

export class InteractionBuilder {
  readonly #description: string;
  readonly #states: contract.State[] = [];
  #request: Record<string, shape.Shape> | undefined;
  #response: Record<string, shape.Shape> | undefined;
  /** Part -> slot -> media type, for every body written with `content` (contract spec §5.5). */
  readonly #contentTypes = new Map<string, Record<string, string>>();

  constructor(description: string) {
    this.#description = description;
  }

  get description(): string {
    return this.#description;
  }

  /**
   * Appends a provider state. A param written as `whenVariant`/`variantCases` becomes a
   * `variant-params` binding under that name; every other param is passed through as written.
   */
  given(name: string, params?: { readonly [k: string]: unknown }): this {
    const literal: Record<string, unknown> = {};
    const bindings: { [k: string]: unknown }[] = [];
    for (const [key, value] of Object.entries(params ?? {})) {
      if (value instanceof VariantBinding) {
        bindings.push(value.binding(key));
      } else {
        refuseNestedBinding(value, `given('${name}').${key}`);
        literal[key] = value;
      }
    }
    this.#states.push({
      name,
      ...(Object.keys(literal).length > 0 ? { params: literal } : {}),
      ...(bindings.length > 0 ? { "variant-params": bindings } : {}),
    });
    return this;
  }

  request(parts: RequestParts): this {
    this.#request = {
      ...slots({ method: parts.method, path: parts.path }),
      ...multiValueSlot("query", parts.query, (name) => name),
      ...multiValueSlot("headers", parts.headers, (name) => name.toLowerCase()),
      ...this.#body("request", parts.body),
    };
    return this;
  }

  response(parts: ResponseParts): this {
    this.#response = {
      ...slots({ status: parts.status }),
      ...multiValueSlot("headers", parts.headers, (name) => name.toLowerCase()),
      ...this.#body("response", parts.body),
    };
    return this;
  }

  /** The body slot: its shape by the 'literal' rules, and its declared media type if `content` named one. */
  #body(part: string, body: Template | Content | undefined): Record<string, shape.Shape> {
    this.#contentTypes.delete(part);
    if (body instanceof Content) {
      this.#contentTypes.set(part, { body: body.mediaType });
      return slots({ body: body.document });
    }
    return slots({ body });
  }

  /** The interaction-spec document this chain describes. */
  build(): contract.InteractionSpec {
    const parts: contract.InteractionSpec["parts"] = {};
    if (this.#request) {
      parts.request = this.#request;
    }
    if (this.#response) {
      parts.response = this.#response;
    }
    return {
      description: this.#description,
      transport: { kind: "http", mode: "passive" },
      ...(this.#states.length > 0 ? { states: this.#states.map((s) => ({ ...s })) } : {}),
      parts,
      ...(this.#contentTypes.size > 0
        ? { "content-types": Object.fromEntries([...this.#contentTypes].map(([part, slots]) => [part, { ...slots }])) }
        : {}),
    };
  }
}

/**
 * A binding inside a literal param would be written as data that only looks like a binding — the
 * in-band ambiguity variant semantics spec §6.2 exists to prevent — so it is refused at the call.
 */
function refuseNestedBinding(value: unknown, where: string): void {
  if (value instanceof VariantBinding) {
    throw new TypeError(`${where}: a variant binding must be a given() param's own value, not nested inside one`);
  }
  if (value !== null && typeof value === "object") {
    for (const [key, inner] of Object.entries(value)) {
      refuseNestedBinding(inner, `${where}.${key}`);
    }
  }
}

/** Slots compiled by the 'literal' rule; a member not given is no slot. */
function slots(members: Record<string, Template | undefined>): Record<string, shape.Shape> {
  const out: Record<string, shape.Shape> = {};
  for (const [name, template] of Object.entries(members)) {
    if (template !== undefined) {
      out[name] = compile(template);
    }
  }
  return out;
}

/**
 * A `{ name: [values…] }` slot (shape spec §3.6): a value is the one-element list, a list is itself,
 * a shape describes the one value — bounded at exactly one, since an unbounded `each-like` would add
 * a request variant sending the header twice. A shape that admits absence is the exception: absence
 * is a fact about the member, not about the value in it, so the list goes *inside* the modifier —
 * `forbidden` is the member's shape as written, and `optional(of)` keeps the `each-like` as its `of`.
 * Wrapping those instead would put a node admitting absence in an `each-like`'s `items`, which shape
 * spec §5.1 refuses. Header names are lower-cased, the only spelling the HTTP transport presents them
 * under; query names are kept as written. Two names that collide once spelled are refused, rather
 * than one of them being dropped where nobody can see it (ADR 0019).
 */
function multiValueSlot(
  slot: string,
  members: { readonly [name: string]: MultiValue } | undefined,
  spell: (name: string) => string,
): Record<string, shape.Shape> {
  if (members === undefined) {
    return {};
  }
  const compiled: Record<string, shape.Shape> = {};
  for (const [name, value] of Object.entries(members)) {
    const spelled = spell(name);
    if (spelled in compiled) {
      throw new TypeError(
        `${slot}: '${name}' is already declared as '${spelled}' — HTTP header names are ` +
          "case-insensitive, so the SDK writes them lower-cased; give one name a list of values " +
          "instead of declaring it twice",
      );
    }
    if ((value as unknown) instanceof Content) {
      throw new TypeError(
        `${slot}.${spelled}: content() declares a body's media type; a ${slot === "query" ? "query" : "header"} ` +
          "value has no content type of its own",
      );
    }
    compiled[spelled] =
      value instanceof Shape
        ? listed(compile(value))
        : {
            shape: "equality",
            example: (Array.isArray(value) ? value : [value]).map((one) => text(one, `${slot}.${spelled}`)),
          };
  }
  return { [slot]: { shape: "object", members: compiled } };
}

/**
 * The one-element list treatment, applied where the value actually is. `forbidden` admits only
 * absence, so there is no value to carry a list and the node stands as the member's shape;
 * `optional` carries the treatment into its `of`. The two are named rather than detected by asking
 * whether a node admits absence, because that question belongs to the shape language and its answer
 * for a component operator is opaque to an SDK (shape spec §3.5) — an SDK that guessed would be
 * holding a second copy of §5.1.
 */
function listed(node: shape.Shape): shape.Shape {
  if (node.shape === "forbidden") {
    return node;
  }
  if (node.shape === "optional") {
    return { ...node, of: listed(node.of as shape.Shape) };
  }
  return { shape: "each-like", items: node, min: 1, max: 1 };
}

/**
 * The one string that spells a header or query value (behavioural spec `request`, ADR 0019). A
 * string is itself; a whole number within ±(2^53 - 1) is its shortest decimal form; a boolean is
 * `true`/`false`. Everything else is refused here, at the call: a value whose spelling differs
 * between languages would have two SDKs send different bytes for the same test, and a value that is
 * not a string at all compiles to a shape the transport's values can never match.
 */
function text(value: unknown, where: string): string {
  if (typeof value === "string") {
    return value;
  }
  if (typeof value === "boolean") {
    return value ? "true" : "false";
  }
  if (typeof value === "number" && Number.isSafeInteger(value)) {
    return String(value);
  }
  throw new TypeError(
    `${where}: ${JSON.stringify(value) ?? String(value)} has no spelling every Janus SDK agrees on; ` +
      "a header or query value is a string, a whole number up to 2^53 - 1, a boolean, a list of " +
      "those, or a shape helper — write the string you mean",
  );
}
