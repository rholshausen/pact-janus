// The conformance suite as this SDK reads it (plan task 6.4): the cases under `conformance/cases`,
// loaded as data. Nothing here is TypeScript's opinion about a case — the corpus is the same file
// the JVM driver reads, and this module only types it.

import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import type { ComponentDeclaration } from "../../src/index.js";

export const repoRoot = resolve(import.meta.dirname, "../../../..");
/**
 * Where the executable specification is: `JANUS_SPEC` — an unpacked `janus-spec.tar.gz` from the
 * release the SDK pins, whose paths are this repository's — or this repository (staged plan S0.11).
 */
export const specRoot = process.env.JANUS_SPEC ? resolve(process.env.JANUS_SPEC) : repoRoot;
export const suiteRoot = join(specRoot, "conformance");

/** A shape-helper call in a case: `{ $: "each-like", args: [...], options: {...} }`. */
export interface HelperCall {
  $: string;
  args?: unknown[];
  options?: Record<string, number>;
}

/** What a case may write as a header or query value — including one the rule refuses. */
export type MultiValue = unknown;

export interface InteractionScript {
  description: string;
  given?: { name: string; params?: Record<string, unknown> }[];
  request?: {
    method?: unknown;
    path?: unknown;
    query?: Record<string, MultiValue>;
    headers?: Record<string, MultiValue>;
    body?: unknown;
  };
  response?: { status?: unknown; headers?: Record<string, MultiValue>; body?: unknown };
}

export interface Exchange {
  method: string;
  path: string;
  query?: string;
  headers?: Record<string, string>;
  body?: unknown;
  "expect-status"?: number;
}

export interface Outcome {
  error: "execute-failed" | "contract-withheld" | "engine-error" | "verification-failed" | "refused";
  code?: string;
  variants?: string[];
  interactions?: string[];
  "engine-withheld"?: boolean;
  failed?: { interaction: unknown; variant: string }[];
}

export interface StepExpectation {
  outcome?: "ok" | Outcome;
  "closure-calls"?: string[];
  contract?: { written?: boolean; text?: string; content?: { interactions?: Record<string, unknown>[] } };
}

export interface Step {
  build?: InteractionScript;
  execute?: { interaction: InteractionScript; closure?: { "fail-on"?: string[]; exchange?: Exchange } };
  finalise?: Record<string, never>;
  expect?: StepExpectation;
}

export interface ScriptedEngine {
  variants?: { id: string; label?: string }[];
  contract?: Record<string, unknown>;
  "withhold-contract"?: boolean;
  results?: Record<string, unknown>[];
  errors?: Record<string, { code: string; message?: string; after?: number; details?: Record<string, unknown> }>;
  capabilities?: Record<string, unknown>;
  config?: Record<string, unknown>;
  events?: { kind: string; payload: Record<string, unknown> }[];
  summary?: Record<string, unknown>;
  invoke?: Invoke[];
}

/** What a state handler's setup or teardown does when called (README §3). */
export interface Behaviour {
  returns?: Record<string, unknown>;
  throws?: string;
  unsupported?: string;
}

/** A provider case's verifier, as data. */
export interface VerifierScript {
  "provider-url"?: string;
  pacts?: Record<string, unknown>[];
  broker?: Record<string, unknown>;
  urls?: Record<string, unknown>;
  options?: Record<string, unknown>;
  config?: string;
  components?: ComponentDeclaration[];
  "state-handlers"?: { state: string; setup: Behaviour; teardown?: Behaviour }[];
  "request-filters"?: { changes?: string[]; set?: Record<string, unknown>; throws?: string }[];
}

/** A hook the scripted engine calls during the run. */
export interface Invoke {
  point: string;
  hook: string;
  context: Record<string, unknown>;
  authorised?: boolean;
}

/** One call of a declared handler, as a driver records it. */
export interface HandlerCall {
  handler: string;
  action?: "setup" | "teardown";
  params?: Record<string, unknown>;
  request?: Record<string, unknown>;
}

/** The provider a live provider case's driver serves. */
export interface LiveProvider {
  routes: {
    method: string;
    path: string;
    "require-headers"?: Record<string, string>;
    status: number;
    headers?: Record<string, string>;
    body?: unknown;
  }[];
}

export interface Case {
  $format: string;
  id: string;
  category: "translation" | "lifecycle" | "variants" | "live" | "provider";
  title: string;
  covers: string[];
  why?: string;
  note?: string;
  interaction?: InteractionScript;
  /** The 'janus' options beyond consumer and provider. */
  janus?: { components?: ComponentDeclaration[] };
  engine?: ScriptedEngine;
  verifier?: VerifierScript;
  verify?: boolean;
  provider?: LiveProvider;
  steps?: Step[];
  expect?: {
    spec?: Record<string, unknown>;
    at?: Record<string, unknown>;
    refused?: boolean;
    ops?: string[];
    frames?: Record<string, Record<string, unknown>>;
    "engine-closed"?: boolean;
    dimensions?: string[];
    "variant-count"?: number;
    outcome?: "ok" | Outcome;
    result?: { status?: string; results?: Record<string, unknown>[] };
    hooks?: Record<string, unknown> | null;
    "config-loaded"?: boolean;
    invocations?: { status: number; result?: unknown }[];
    "handler-calls"?: HandlerCall[];
    "endpoint-closed"?: boolean;
  };
}

/** Every case in the corpus, in id order — the order a report lists them in. */
export function loadCases(): Case[] {
  const root = join(suiteRoot, "cases");
  const cases: Case[] = [];
  for (const category of readdirSync(root)) {
    for (const file of readdirSync(join(root, category))) {
      if (file.endsWith(".json")) {
        cases.push(JSON.parse(readFileSync(join(root, category, file), "utf8")) as Case);
      }
    }
  }
  return cases.sort((a, b) => a.id.localeCompare(b.id));
}
