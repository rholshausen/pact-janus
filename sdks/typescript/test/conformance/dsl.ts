// A case's DSL usage, replayed through this SDK's own surface. This is the whole of what makes the
// suite language-independent: a case names a behavioural-spec primitive and its arguments, and each
// language's driver knows only how that primitive is spelled there — `each-like` is `eachLike` here
// and `Shapes.eachLike` on the JVM. Nothing in this file decides what a primitive *means*.

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
  type InteractionBuilder,
  type MultiValue as SdkMultiValue,
  type Shape,
  type Template,
} from "../../src/index.js";
import type { HelperCall, InteractionScript, MultiValue } from "./cases.js";

/** Primitive id (behavioural-spec.json) -> how this SDK spells it. */
const helpers: Record<string, (args: unknown[], options: Record<string, number>) => Shape | Template> = {
  literal: (args) => template(args[0]),
  json: (args) => json(template(args[0])),
  integer: (args) => integer(args[0] as number),
  number: (args) => number(args[0] as number),
  decimal: (args) => decimal(args[0] as number),
  string: (args) => string(args[0] as string),
  boolean: (args) => boolean(args[0] as boolean),
  datetime: (args) => datetime(args[0] as string, args[1] as string | undefined),
  date: (args) => date(args[0] as string, args[1] as string | undefined),
  time: (args) => time(args[0] as string, args[1] as string | undefined),
  regex: (args) => regex(args[0] as string, args[1] as string),
  "any-of": (args) => anyOf(...(args as [string, ...string[]])),
  "one-of": (args) =>
    oneOf(args[0] as string, mapValues(args[1] as Record<string, Record<string, unknown>>, alternative)),
  optional: (args) => optional(template(args[0])),
  nullable: (args) => nullable(template(args[0])),
  "each-like": (args, options) => eachLike(template(args[0]), options),
};

/** A case's template: plain JSON as written, an object carrying `$` as the helper call it names. */
export function template(value: unknown): Template {
  if (Array.isArray(value)) {
    return value.map(template);
  }
  if (isCall(value)) {
    return call(value) as Template;
  }
  if (value !== null && typeof value === "object") {
    return mapValues(value as Record<string, unknown>, template);
  }
  return value as Template;
}

function call(helperCall: HelperCall): Shape | Template {
  const helper = helpers[helperCall.$];
  if (!helper) {
    throw new Error(`the case names a shape primitive this SDK does not implement: '${helperCall.$}'`);
  }
  return helper(helperCall.args ?? [], helperCall.options ?? {});
}

/** The chain a case scripts, built with this SDK's builder, one call per member. */
export function buildInteraction(janus: Janus, script: InteractionScript): InteractionBuilder {
  let interaction = janus.interaction(script.description);
  for (const state of script.given ?? []) {
    interaction = state.params === undefined ? interaction.given(state.name) : interaction.given(state.name, state.params);
  }
  if (script.request) {
    const { method, path, query, headers, body } = script.request;
    interaction = interaction.request({
      ...(method === undefined ? {} : { method: template(method) }),
      ...(path === undefined ? {} : { path: template(path) }),
      ...(query === undefined ? {} : { query: mapValues(query, multiValue) }),
      ...(headers === undefined ? {} : { headers: mapValues(headers, multiValue) }),
      ...(body === undefined ? {} : { body: template(body) }),
    });
  }
  if (script.response) {
    const { status, headers, body } = script.response;
    interaction = interaction.response({
      ...(status === undefined ? {} : { status: template(status) }),
      ...(headers === undefined ? {} : { headers: mapValues(headers, multiValue) }),
      ...(body === undefined ? {} : { body: template(body) }),
    });
  }
  return interaction;
}

/** A header or query value: one string, several strings, or a shape describing its one value. */
function multiValue(value: MultiValue): SdkMultiValue {
  if (typeof value === "string" || Array.isArray(value)) {
    return value;
  }
  return call(value) as Shape;
}

/** A `one-of` alternative: a plain map, each member compiled by the literal rules. */
function alternative(value: Record<string, unknown>): Record<string, Template> {
  return mapValues(value, template);
}

function isCall(value: unknown): value is HelperCall {
  return typeof value === "object" && value !== null && !Array.isArray(value) && typeof (value as HelperCall).$ === "string";
}

function mapValues<In, Out>(source: Record<string, In>, f: (value: In) => Out): Record<string, Out> {
  return Object.fromEntries(Object.entries(source).map(([key, value]) => [key, f(value)]));
}
