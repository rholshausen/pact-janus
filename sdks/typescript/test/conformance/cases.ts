// The conformance suite as this SDK reads it (plan task 6.4): the cases under `conformance/cases`,
// loaded as data. Nothing here is TypeScript's opinion about a case — the corpus is the same file
// the JVM driver reads, and this module only types it.

import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

export const repoRoot = resolve(import.meta.dirname, "../../../..");
export const suiteRoot = join(repoRoot, "conformance");

/** A shape-helper call in a case: `{ $: "each-like", args: [...], options: {...} }`. */
export interface HelperCall {
  $: string;
  args?: unknown[];
  options?: Record<string, number>;
}

export type MultiValue = string | string[] | HelperCall;

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
  error: "execute-failed" | "contract-withheld" | "engine-error";
  code?: string;
  variants?: string[];
  interactions?: string[];
  "engine-withheld"?: boolean;
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
  errors?: Record<string, { code: string; message?: string; details?: Record<string, unknown> }>;
}

export interface Case {
  $format: string;
  id: string;
  category: "translation" | "lifecycle" | "variants" | "live";
  title: string;
  covers: string[];
  why?: string;
  note?: string;
  interaction?: InteractionScript;
  engine?: ScriptedEngine;
  steps?: Step[];
  expect?: {
    spec?: Record<string, unknown>;
    at?: Record<string, unknown>;
    ops?: string[];
    frames?: Record<string, Record<string, unknown>>;
    "engine-closed"?: boolean;
    dimensions?: string[];
    "variant-count"?: number;
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
