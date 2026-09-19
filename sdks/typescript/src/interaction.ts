// The interaction builder (behavioural spec 'interaction', 'given', 'request', 'response'): builds
// an interaction-spec document (contract.schema.json#/$defs/InteractionSpec) and makes no protocol
// call — `execute` submits it.

import type { contract, shape } from "./generated/index.js";
import { compile, type Shape, type Template } from "./shapes.js";

/** A header or query parameter: one value, several values, or a shape its one value must match. */
export type MultiValue = string | readonly string[] | Shape;

export interface RequestParts {
  method?: Template;
  path?: Template;
  query?: { readonly [name: string]: MultiValue };
  headers?: { readonly [name: string]: MultiValue };
  body?: Template;
}

export interface ResponseParts {
  status?: Template;
  headers?: { readonly [name: string]: MultiValue };
  body?: Template;
}

export class InteractionBuilder {
  readonly #description: string;
  readonly #states: contract.State[] = [];
  #request: Record<string, shape.Shape> | undefined;
  #response: Record<string, shape.Shape> | undefined;

  constructor(description: string) {
    this.#description = description;
  }

  get description(): string {
    return this.#description;
  }

  /** Appends a provider state; `params` passed through as written. */
  given(name: string, params?: { readonly [k: string]: unknown }): this {
    this.#states.push(params === undefined ? { name } : { name, params: { ...params } });
    return this;
  }

  request(parts: RequestParts): this {
    this.#request = {
      ...slots({ method: parts.method, path: parts.path }),
      ...multiValueSlot("query", parts.query, (name) => name),
      ...multiValueSlot("headers", parts.headers, (name) => name.toLowerCase()),
      ...slots({ body: parts.body }),
    };
    return this;
  }

  response(parts: ResponseParts): this {
    this.#response = {
      ...slots({ status: parts.status }),
      ...multiValueSlot("headers", parts.headers, (name) => name.toLowerCase()),
      ...slots({ body: parts.body }),
    };
    return this;
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
    };
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
 * A `{ name: [values…] }` slot (shape spec §3.6): a string is the one-element list, a list is
 * itself, a shape describes the one value — bounded at exactly one, since an unbounded `each-like`
 * would add a request variant sending the header twice. Header names are lower-cased, the only
 * spelling the HTTP transport presents them under; query names are kept as written.
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
    compiled[spell(name)] =
      typeof value === "string"
        ? { shape: "equality", example: [value] }
        : Array.isArray(value)
          ? { shape: "equality", example: [...value] }
          : { shape: "each-like", items: compile(value as Shape), min: 1, max: 1 };
  }
  return { [slot]: { shape: "object", members: compiled } };
}
