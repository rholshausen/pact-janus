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
  /** 'exclude' calls, in order: each its [dimension, point] pairs and its reason. */
  exclude?: { when: [string, string][]; reason: string }[];
  request?: {
    method?: unknown;
    path?: unknown;
    query?: Record<string, MultiValue>;
    headers?: Record<string, MultiValue>;
    body?: unknown;
  };
  response?: { status?: unknown; headers?: Record<string, MultiValue>; body?: unknown };
  /** 'message-contents': its presence makes this a message chain (ADR 0068). */
  message?: MessageSlotsScript;
  /** 'message-request': the request a synchronous message answers. */
  "message-request"?: MessageSlotsScript;
}

/** A message's slots, as 'message-contents' and 'message-request' take them. */
export interface MessageSlotsScript {
  metadata?: Record<string, unknown>;
  body?: unknown;
}

/** What a message closure was handed, compared on the members a case names (README §4). */
export interface Received {
  json?: unknown;
  text?: string;
  "content-type"?: string | null;
  metadata?: Record<string, unknown>;
  request?: Received;
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
  failed?: { interaction: unknown; variant: string }[];
}

export interface StepExpectation {
  outcome?: "ok" | Outcome;
  "closure-calls"?: string[];
  received?: Received[];
  contract?: { written?: boolean; text?: string; content?: { interactions?: Record<string, unknown>[] } };
}

export interface Step {
  build?: InteractionScript;
  execute?: { interaction: InteractionScript; closure?: { "fail-on"?: string[]; "skip-on"?: string[]; exchange?: Exchange } };
  finalise?: Record<string, never>;
  expect?: StepExpectation;
}

export interface ScriptedEngine {
  variants?: { id: string; label?: string }[];
  contract?: Record<string, unknown>;
  "contract-text"?: string;
  "withhold-contract"?: boolean;
  results?: Record<string, unknown>[];
  refused?: Record<string, unknown>[];
  /** Variant id -> the result 'report-variant' answers with for it. */
  reports?: Record<string, Record<string, unknown>>;
  /** Variant id -> the 'parts' 'serve-variant' answers with for it: a message's (ADR 0067). */
  served?: Record<string, Record<string, unknown>>;
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
  pacts?: (Record<string, unknown> | string)[];
  broker?: Record<string, unknown>;
  urls?: Record<string, unknown>;
  options?: Record<string, unknown>;
  config?: string;
  components?: ComponentDeclaration[];
  "state-handlers"?: { state: string; setup: Behaviour; teardown?: Behaviour }[];
  "request-filters"?: { changes?: string[]; set?: Record<string, unknown>; throws?: string }[];
  transport?: Record<string, unknown>;
  "message-producers"?: { description: string; returns?: MessageScript; throws?: string }[];
}

/** The message a producer returns (README §3): one of json, text or bytes (base64). */
export interface MessageScript {
  json?: unknown;
  text?: string;
  bytes?: string;
  "content-type"?: string;
  metadata?: Record<string, unknown>;
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

/** One test a subject has, and how it must end (ADR 0076 decision 1). */
export interface TestExpectation {
  test: string;
  /** The test must fail, its error's message holding each of these. */
  fails?: string[];
}

/** A contract file a subject's run wrote, as a step's `contract` compares it. */
export interface ContractExpectation {
  written?: boolean;
  content?: { interactions?: Record<string, unknown>[] };
}

export interface Case {
  $format: string;
  id: string;
  category: "translation" | "lifecycle" | "variants" | "live" | "provider" | "facade" | "framework";
  /** A facade case's facade (ADR 0076 decision 2). */
  facade?: string;
  /** A facade case's migration-note rows. The driver does not read them; the lint does. */
  rows?: { note: string; row: string }[];
  /** Language -> a test written in it, beside the case; in a framework case, framework -> a test file it runs. */
  subjects?: Record<string, string>;
  /** A framework case's files, written into each subject's project before it runs (ADR 0078). */
  project?: Record<string, unknown>;
  title: string;
  covers: string[];
  why?: string;
  note?: string;
  interaction?: InteractionScript;
  /** The 'janus' options beyond consumer and provider. */
  janus?: { components?: ComponentDeclaration[]; "overwrite-contract"?: boolean; redaction?: { add?: string[]; keep?: string[] } };
  engine?: ScriptedEngine;
  verifier?: VerifierScript;
  verify?: boolean;
  provider?: LiveProvider;
  steps?: Step[];
  expect?: {
    tests?: TestExpectation[];
    suite?: { fails?: string[] };
    contracts?: Record<string, ContractExpectation>;
    output?: { stdout?: string[]; stderr?: string[] };
    "engine-log"?: string[];
    spec?: Record<string, unknown>;
    at?: Record<string, unknown>;
    refused?: boolean;
    ops?: string[];
    frames?: Record<string, Record<string, unknown>>;
    "engine-closed"?: boolean;
    dimensions?: string[];
    "variant-count"?: number;
    outcome?: "ok" | Outcome;
    result?: { status?: string; results?: Record<string, unknown>[]; warnings?: Record<string, unknown>[] };
    printed?: string[];
    hooks?: Record<string, unknown> | null;
    "config-loaded"?: boolean;
    invocations?: { status: number; result?: unknown }[];
    "handler-calls"?: HandlerCall[];
    "endpoint-closed"?: boolean;
  };
}

/** This SDK's language, as its report names it and `conformance/facades.json` names a facade's. */
export const LANGUAGE = "typescript";

/** The facades whose cases this SDK runs: those `conformance/facades.json` gives its language (ADR 0076). */
export function facadesShipped(): Set<string> {
  const document = JSON.parse(readFileSync(join(suiteRoot, "facades.json"), "utf8")) as {
    facades: Record<string, { language: string }>;
  };
  return new Set(Object.entries(document.facades).filter(([, f]) => f.language === LANGUAGE).map(([name]) => name));
}

/** A test framework an SDK integrates with, as `conformance/frameworks.json` gives it (ADR 0078). */
export interface Framework {
  language: string;
  subject: string;
}

/** Every framework the framework cases run, by name. */
export function frameworks(): Record<string, Framework> {
  const document = JSON.parse(readFileSync(join(suiteRoot, "frameworks.json"), "utf8")) as {
    frameworks: Record<string, Framework>;
  };
  return document.frameworks;
}

/** A framework case's subjects in this language: each framework's name and its file, beside the case. */
export function frameworkSubjects(testCase: Case): { framework: string; file: string }[] {
  const known = frameworks();
  return Object.entries(testCase.subjects ?? {})
    .filter(([framework]) => known[framework]?.language === LANGUAGE)
    .map(([framework, file]) => ({ framework, file: join(suiteRoot, "cases", testCase.category, file) }));
}

/** The file a case's subject in this language is: beside the case. */
export function subjectPath(testCase: Case): string | undefined {
  const file = testCase.subjects?.[LANGUAGE];
  return file === undefined ? undefined : join(suiteRoot, "cases", testCase.category, file);
}

/**
 * Every case this SDK runs, in id order — the order a report lists them in: every case that is not
 * a facade's or a framework's, every case of each facade this language ships (ADR 0076 decision 2),
 * and every framework case with a subject for one of this language's frameworks (ADR 0078 decision 5).
 */
export function loadCases(): Case[] {
  const shipped = facadesShipped();
  return loadCorpus().filter((testCase) =>
    testCase.category === "framework"
      ? frameworkSubjects(testCase).length > 0
      : testCase.facade === undefined || shipped.has(testCase.facade),
  );
}

/** Every case in the corpus, in id order. */
export function loadCorpus(): Case[] {
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
