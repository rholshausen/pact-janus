// Shape helpers (behavioural spec, category 'shape'): each one maps 1:1 to a shape-language
// operator and stops there. No helper decides what a value admits, computes a variant dimension,
// or checks what the engine will check anyway — the engine does all three (SDK spec §2.1).

import type { shape } from "./generated/index.js";

/**
 * A shape helper's result. A class rather than a plain `{ shape: … }` object so that a helper's
 * result can never be mistaken for a plain map that happens to have a `shape` member (behavioural
 * spec 'literal': that distinction MUST NOT be made by looking for the member).
 */
export class Shape {
  readonly #node: shape.Shape;

  /** @internal Shapes come from the helpers below. */
  constructor(node: shape.Shape) {
    this.#node = node;
  }

  /** The shape document this helper produced (shape spec §3). */
  toJSON(): shape.Shape {
    return this.#node;
  }
}

/** JSON values, as a DSL author writes them where a shape is expected. */
export type Template = Shape | string | number | boolean | null | readonly Template[] | { readonly [k: string]: Template };

/**
 * The behavioural spec's 'literal' rule: a helper is used as authored; a scalar is `equality`; a
 * plain map is `object`, one member per key in the order written; a list is `array`, one positional
 * entry per element.
 */
export function compile(template: Template): shape.Shape {
  if (template instanceof Shape) {
    return template.toJSON();
  }
  if (template === null || typeof template !== "object") {
    return { shape: "equality", example: template };
  }
  if (Array.isArray(template)) {
    return { shape: "array", entries: template.map(compile) };
  }
  const members: Record<string, shape.Shape> = {};
  for (const [name, member] of Object.entries(template as { readonly [k: string]: Template })) {
    members[name] = compile(member);
  }
  return { shape: "object", members };
}

const withExample = (operator: string, example: unknown, extra: Partial<shape.Shape> = {}): Shape =>
  new Shape({ shape: operator, ...extra, example });

/** A JSON body. Compiles its argument by the 'literal' rules and adds nothing (behavioural spec 'json'). */
export const json = (document: Template): Shape => new Shape(compile(document));

export const integer = (example: number): Shape => withExample("integer", example);
export const number = (example: number): Shape => withExample("number", example);
export const decimal = (example: number): Shape => withExample("decimal", example);
export const string = (example: string): Shape => withExample("string", example);
export const boolean = (example: boolean): Shape => withExample("boolean", example);

/** ISO-8601 unless a `format` is given; the SDK never infers one from the example. */
export const datetime = (example: string, format?: string): Shape =>
  withExample("datetime", example, format === undefined ? {} : { format });
export const date = (example: string, format?: string): Shape =>
  withExample("date", example, format === undefined ? {} : { format });
export const time = (example: string, format?: string): Shape =>
  withExample("time", example, format === undefined ? {} : { format });

/**
 * A string matching `pattern`, unanchored — write `^…$` for a full match. A `RegExp` carrying flags
 * is refused: the pattern text cannot carry them, and dropping one would change what it means.
 */
export function regex(pattern: RegExp | string, example: string): Shape {
  if (pattern instanceof RegExp && pattern.flags !== "") {
    throw new TypeError(
      `regex(${String(pattern)}): flags ('${pattern.flags}') cannot be carried in a shape's pattern; ` +
        "write them into the pattern (e.g. '(?i)') or drop them",
    );
  }
  return withExample("regex", example, { pattern: typeof pattern === "string" ? pattern : pattern.source });
}

/** One of these literal values; the first is the example. */
export function anyOf(...options: [string | number | boolean | null, ...(string | number | boolean | null)[]]): Shape {
  return withExample("any-of", options[0], { options });
}

/** A discriminated union: each alternative binds `discriminator` to its own literal. */
export function oneOf(discriminator: string, alternatives: { readonly [name: string]: { readonly [k: string]: Template } }): Shape {
  const compiled: Record<string, shape.Shape> = {};
  for (const [name, alternative] of Object.entries(alternatives)) {
    compiled[name] = compile(alternative);
  }
  return new Shape({ shape: "one-of", discriminator, alternatives: compiled });
}

export const optional = (of: Template): Shape => new Shape({ shape: "optional", of: compile(of) });
export const nullable = (of: Template): Shape => new Shape({ shape: "nullable", of: compile(of) });

/** An array of elements like `items`; `min` and `max` only when given (absent: 1 and unbounded). */
export function eachLike(items: Template, options: { min?: number; max?: number } = {}): Shape {
  const node: shape.Shape = { shape: "each-like", items: compile(items) };
  if (options.min !== undefined) {
    node.min = options.min;
  }
  if (options.max !== undefined) {
    node.max = options.max;
  }
  return new Shape(node);
}
