// Runs one conformance case against this SDK and says what it found. Every check a case can ask
// for lives here; a case's own JSON says which ones it asks for. A failure is a list of lines, so
// a report can carry it and a test can throw it.

import { mkdir, mkdtemp, readFile, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  ContractWithheldError,
  Janus,
  JanusError,
  VariantsFailedError,
  type Variant,
} from "../../src/index.js";
import type { Case, Exchange, InteractionScript, Outcome, StepExpectation } from "./cases.js";
import { equal, pointer, show, subset } from "./compare.js";
import { buildInteraction } from "./dsl.js";
import { CONSUMER, PROVIDER, ScriptedPipe } from "./scripted-engine.js";

export interface CaseResult {
  id: string;
  status: "passed" | "failed";
  covers: string[];
  detail?: string;
}

export async function runCase(testCase: Case): Promise<CaseResult> {
  const failures: string[] = [];
  try {
    if (testCase.category === "translation") {
      await runTranslation(testCase, failures);
    } else {
      await runSession(testCase, failures);
    }
  } catch (error) {
    failures.push(`the case did not run: ${error instanceof Error ? error.stack ?? error.message : String(error)}`);
  }
  return {
    id: testCase.id,
    covers: testCase.covers,
    status: failures.length === 0 ? "passed" : "failed",
    ...(failures.length === 0 ? {} : { detail: failures.join("\n") }),
  };
}

/** Category 1: what a DSL chain builds, with no engine anywhere. */
async function runTranslation(testCase: Case, failures: string[]): Promise<void> {
  const janus = new Janus({ consumer: CONSUMER, provider: PROVIDER });
  // The schema requires an `interaction` of a translation case, and the corpus lints against it.
  const script = testCase.interaction as InteractionScript;
  const expected = testCase.expect ?? {};

  if (expected.refused) {
    // The chain must not become a document. How it refuses is this language's business; that it
    // refuses is the case's (ADR 0019).
    try {
      const document = buildInteraction(janus, script).build();
      failures.push(`the chain was accepted, and the case expects it refused: ${show(document)}`);
    } catch {
      // Refused, as the case requires.
    }
    return;
  }

  const built = buildInteraction(janus, script).build() as unknown as Record<string, unknown>;
  if (expected.spec && !equal(expected.spec, built)) {
    failures.push(`the built interaction-spec document differs\n  expected: ${show(expected.spec)}\n  actual:   ${show(built)}`);
  }
  for (const [path, value] of Object.entries(expected.at ?? {})) {
    const actual = pointer(built, path);
    if (!equal(value, actual)) {
      failures.push(`at '${path}'\n  expected: ${show(value)}\n  actual:   ${show(actual)}`);
    }
  }
}

/** Categories 2–4: the SDK driving an engine — the case's scripted one, or the real one. */
async function runSession(testCase: Case, failures: string[]): Promise<void> {
  const live = testCase.category === "live";
  const pipe = live ? undefined : new ScriptedPipe(testCase.engine ?? {});
  const contractDir = await mkdtemp(join(tmpdir(), "janus-conformance-"));
  const janus = new Janus({
    consumer: CONSUMER,
    provider: PROVIDER,
    contractDir,
    ...(pipe ? { engine: () => pipe } : {}),
  });
  const dimensions = new Set<string>();
  const variantCounts: number[] = [];

  try {
    for (const [index, step] of (testCase.steps ?? []).entries()) {
      const where = `step ${index + 1}`;
      if (step.build) {
        buildInteraction(janus, step.build);
        checkStep(step.expect, { outcome: "ok" }, where, failures);
      } else if (step.execute) {
        const calls: string[] = [];
        const failOn = step.execute.closure?.["fail-on"] ?? [];
        const exchange = step.execute.closure?.exchange;
        const interaction = buildInteraction(janus, step.execute.interaction);
        const outcome = await capture(() =>
          janus.execute(interaction, async (mock, variant: Variant) => {
            calls.push(variant.id);
            for (const point of assignmentOf(variant)) {
              dimensions.add(point);
            }
            if (failOn.includes(variant.id) || failOn.includes("*")) {
              throw new Error(`the consumer cannot handle variant '${variant.id}'`);
            }
            if (exchange) {
              await send(mock.url, exchange);
            }
          }),
        );
        variantCounts.push(calls.length);
        checkStep(step.expect, { outcome, calls }, where, failures);
      } else if (step.finalise) {
        const outcome = await capture(async () => {
          await janus.finalise();
        });
        const contract = await readContract(contractDir);
        await record(testCase, contract.document);
        checkStep(step.expect, { outcome, contract }, where, failures);
      }
    }

    const expected = testCase.expect ?? {};
    if (expected.ops && pipe && !equal(expected.ops, pipe.ops)) {
      failures.push(`the operations sent differ\n  expected: ${show(expected.ops)}\n  actual:   ${show(pipe.ops)}`);
    }
    for (const [op, body] of Object.entries(expected.frames ?? {})) {
      const sent = pipe?.bodyOf(op);
      if (!subset(body, sent)) {
        failures.push(`the '${op}' frame body\n  expected (a subset of): ${show(body)}\n  actual: ${show(sent)}`);
      }
    }
    if (expected["engine-closed"] !== undefined && pipe && pipe.closed !== expected["engine-closed"]) {
      failures.push(`the engine was ${pipe.closed ? "" : "not "}ended, and the case expects it ${expected["engine-closed"] ? "" : "not "}to be`);
    }
    for (const dimension of expected.dimensions ?? []) {
      if (!dimensions.has(dimension)) {
        failures.push(`the engine assigned no '${dimension}' dimension; it assigned ${show([...dimensions].sort())}`);
      }
    }
    if (expected["variant-count"] !== undefined && !variantCounts.includes(expected["variant-count"])) {
      failures.push(`the engine selected ${show(variantCounts)} variants, and the case expects ${expected["variant-count"]}`);
    }
  } finally {
    // Sessions are the only resource (engine-protocol spec §7.1): a case that did not finalise
    // still has an engine to end, and on a live case that is a subprocess.
    await janus.finalise().catch(() => undefined);
  }
}

interface Actual {
  outcome: "ok" | Outcome;
  calls?: string[];
  contract?: { written: boolean; text?: string; document?: Record<string, unknown> };
}

function checkStep(expected: StepExpectation | undefined, actual: Actual, where: string, failures: string[]): void {
  if (!expected) {
    return;
  }
  if (expected.outcome !== undefined && !matches(expected.outcome, actual.outcome)) {
    failures.push(`${where}: the outcome differs\n  expected: ${show(expected.outcome)}\n  actual:   ${show(actual.outcome)}`);
  }
  if (expected["closure-calls"] !== undefined && !equal(expected["closure-calls"], actual.calls ?? [])) {
    failures.push(
      `${where}: the closure ran on different variants\n  expected: ${show(expected["closure-calls"])}\n  actual:   ${show(actual.calls)}`,
    );
  }
  const contract = expected.contract;
  if (contract && actual.contract) {
    if (contract.written !== undefined && contract.written !== actual.contract.written) {
      failures.push(`${where}: a contract was ${actual.contract.written ? "" : "not "}written, and the case expects it ${contract.written ? "" : "not "}to be`);
    }
    if (contract.text !== undefined && contract.text !== actual.contract.text) {
      failures.push(`${where}: the contract file's text differs\n  expected: ${show(contract.text)}\n  actual:   ${show(actual.contract.text)}`);
    }
    if (contract.content) {
      checkContent(contract.content, actual.contract.document, where, failures);
    }
  }
}

/**
 * Interaction content, and nothing else (ADR 0017): each interaction by description, compared on
 * the members the case names — `states`, `parts`, `selection`. `metadata` and the parties are facts
 * about the SDK that wrote the file, deliberately outside content identity.
 */
function checkContent(
  expected: { interactions?: Record<string, unknown>[] },
  actual: Record<string, unknown> | undefined,
  where: string,
  failures: string[],
): void {
  const recorded = (actual?.interactions ?? []) as Record<string, unknown>[];
  const wanted = expected.interactions ?? [];
  if (recorded.length !== wanted.length) {
    failures.push(`${where}: the contract records ${recorded.length} interactions, and the case expects ${wanted.length}`);
  }
  for (const interaction of wanted) {
    const found = recorded.find((r) => r.description === interaction.description);
    if (!found) {
      failures.push(`${where}: the contract records no interaction '${String(interaction.description)}'`);
      continue;
    }
    for (const [member, value] of Object.entries(interaction)) {
      if (!equal(value, found[member])) {
        failures.push(
          `${where}: '${String(interaction.description)}' recorded a different '${member}'\n  expected: ${show(value)}\n  actual:   ${show(found[member])}`,
        );
      }
    }
  }
}

function matches(expected: "ok" | Outcome, actual: "ok" | Outcome): boolean {
  if (expected === "ok" || actual === "ok") {
    return expected === actual;
  }
  if (expected.error !== actual.error) {
    return false;
  }
  if (expected.code !== undefined && expected.code !== actual.code) {
    return false;
  }
  if (expected.variants !== undefined && !equal(expected.variants, actual.variants)) {
    return false;
  }
  if (expected.interactions !== undefined && !equal(expected.interactions, actual.interactions)) {
    return false;
  }
  if (expected["engine-withheld"] !== undefined && expected["engine-withheld"] !== actual["engine-withheld"]) {
    return false;
  }
  return true;
}

/** This SDK's failures, as the case's language-independent outcomes (README §4). */
async function capture(run: () => Promise<void>): Promise<"ok" | Outcome> {
  try {
    await run();
    return "ok";
  } catch (error) {
    if (error instanceof VariantsFailedError) {
      return { error: "execute-failed", variants: error.failures.map((f) => f.variant.id) };
    }
    if (error instanceof ContractWithheldError) {
      return {
        error: "contract-withheld",
        interactions: error.failedTests,
        "engine-withheld": error.engineWithheld,
      };
    }
    if (error instanceof JanusError) {
      return { error: "engine-error", code: error.code };
    }
    throw error;
  }
}

/**
 * Record mode: with `JANUS_CONFORMANCE_RECORD` set to a directory, writes the contract content a
 * case produced, in the form a case's `contract.content` expects. This is how a `live` case's
 * expected content is captured — from the engine's own record, then read and reviewed against the
 * contract spec before it is checked in, never pasted in unread (README §6).
 */
async function record(testCase: Case, document: Record<string, unknown> | undefined): Promise<void> {
  const directory = process.env.JANUS_CONFORMANCE_RECORD;
  if (!directory || !document) {
    return;
  }
  const interactions = ((document.interactions ?? []) as Record<string, unknown>[]).map(
    ({ description, states, parts, selection }) => ({
      description,
      ...(states === undefined ? {} : { states }),
      ...(parts === undefined ? {} : { parts }),
      ...(selection === undefined ? {} : { selection }),
    }),
  );
  await mkdir(directory, { recursive: true });
  await writeFile(
    join(directory, `${testCase.id.replace("/", "-")}.json`),
    `${JSON.stringify({ interactions }, null, 2)}\n`,
    "utf8",
  );
}

async function readContract(directory: string): Promise<{ written: boolean; text?: string; document?: Record<string, unknown> }> {
  const file = join(directory, `${CONSUMER}-${PROVIDER}.janus.json`);
  try {
    await stat(file);
  } catch {
    return { written: false };
  }
  const text = await readFile(file, "utf8");
  return { written: true, text, document: JSON.parse(text) as Record<string, unknown> };
}

/** The dimension ids a variant's assignment names (variant semantics spec §3.9). */
function assignmentOf(variant: Variant): string[] {
  const assignment = (variant as unknown as { assignment?: { dimension?: string }[] }).assignment ?? [];
  return assignment.flatMap((point) => (typeof point.dimension === "string" ? [point.dimension] : []));
}

/** A live case's consumer: the request the case says to send, and the status it expects back. */
async function send(baseUrl: string, exchange: Exchange): Promise<void> {
  const url = `${baseUrl}${exchange.path}${exchange.query ? `?${exchange.query}` : ""}`;
  const response = await fetch(url, {
    method: exchange.method,
    headers: {
      ...(exchange.headers ?? {}),
      ...(exchange.body === undefined ? {} : { "Content-Type": "application/json" }),
    },
    ...(exchange.body === undefined ? {} : { body: JSON.stringify(exchange.body) }),
  });
  const body = await response.text();
  const expected = exchange["expect-status"];
  if (expected !== undefined && response.status !== expected) {
    throw new Error(`${exchange.method} ${url}: the mock answered ${response.status}, not ${expected}: ${body}`);
  }
}
