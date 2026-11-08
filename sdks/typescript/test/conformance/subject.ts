// Runs a case's subject (ADR 0076 decision 1): a test written in TypeScript, beside the case, for a
// spelling data cannot carry — a compatibility facade's, or a refusal of one of the language's own
// types. The subject is a module whose exported async functions are its tests. This file calls the
// ones the case names, in its order, with a context, and checks what came of the run against the
// case's data: how each test ended, the contracts written, what was printed, the engine's log, and
// the frames a scripted engine was sent. It decides nothing a case means.

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { mkdtemp, readdir, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import type { FramePipe } from "../../src/index.js";
import { subjectPath, type Case } from "./cases.js";
import { show } from "./compare.js";
import { checkEngine } from "./provider.js";
import { checkContent, contentOf } from "./runner.js";
import { ScriptedPipe } from "./scripted-engine.js";

/** What each test is handed: where contracts go, and the scripted engine when the case has one. */
export interface SubjectContext {
  dir: string;
  engine?: () => FramePipe;
}

/** What a subject's run did, as plain data, so a child run can hand it back. */
export interface SubjectRun {
  /** Each test the case names, in order, with its failure's message when it failed. */
  tests: { test: string; error?: string }[];
  /** The subject's exported functions the case does not name. */
  unnamed: string[];
  /** Why the subject could not be loaded at all. */
  suite?: string;
  stdout: string;
  stderr: string;
}

export async function runSubject(testCase: Case, failures: string[]): Promise<void> {
  const file = subjectPath(testCase);
  if (file === undefined) {
    // ADR 0076 decision 5: a case with subjects runs in every language it is for, and this one has none.
    failures.push(`the case names no TypeScript subject, and it is a case for this language: ${show(testCase.subjects)}`);
    return;
  }
  const expected = testCase.expect ?? {};
  const dir = await mkdtemp(join(tmpdir(), "janus-subject-"));
  const pipe = testCase.engine === undefined ? undefined : new ScriptedPipe(testCase.engine);
  let run: SubjectRun;
  let engineLog = "";
  if (expected["engine-log"] !== undefined) {
    // The engine inherits this process's standard error, which nothing in it can read: a run whose
    // case expects the engine's log runs in a child, whose standard error is the engine's too.
    ({ run, engineLog } = runIsolated(testCase, dir));
  } else {
    run = await runTests(testCase, file, { dir, ...(pipe ? { engine: () => pipe } : {}) });
  }

  if (run.suite !== undefined) {
    failures.push(`the subject could not be loaded: ${run.suite}`);
  }
  for (const name of run.unnamed) {
    failures.push(`the subject has a test '${name}' the case does not name`);
  }
  const wanted = expected.tests ?? [];
  for (const { test, fails } of wanted) {
    const outcome = run.tests.find((t) => t.test === test);
    if (outcome === undefined) {
      continue;
    }
    if (fails === undefined && outcome.error !== undefined) {
      failures.push(`'${test}' failed, and the case expects it to pass: ${outcome.error}`);
    } else if (fails !== undefined && outcome.error === undefined) {
      failures.push(`'${test}' passed, and the case expects it to fail with ${show(fails)}`);
    } else if (fails !== undefined) {
      for (const part of fails.filter((part) => !outcome.error?.includes(part))) {
        failures.push(`'${test}' failed without saying ${show(part)}: ${outcome.error}`);
      }
    }
  }
  if (expected.suite?.fails !== undefined && run.suite === undefined) {
    failures.push(`the subject loaded, and the case expects it to fail with ${show(expected.suite.fails)}`);
  }

  await checkContracts(testCase, dir, failures);
  for (const [stream, parts] of Object.entries(expected.output ?? {})) {
    const text = stream === "stdout" ? run.stdout : run.stderr;
    for (const part of parts.filter((part) => !text.includes(part))) {
      failures.push(`the run's ${stream} does not hold ${show(part)}:\n${text}`);
    }
  }
  for (const part of (expected["engine-log"] ?? []).filter((part) => !engineLog.includes(part))) {
    failures.push(`the engine's log does not hold ${show(part)}:\n${engineLog}`);
  }
  await checkEngine(expected, pipe, failures);
}

/**
 * Loads the subject and calls each test the case names, in order, keeping what it printed. A test
 * fails by throwing or rejecting; its error's message is what a case compares.
 */
export async function runTests(testCase: Case, file: string, context: SubjectContext): Promise<SubjectRun> {
  const names = (testCase.expect?.tests ?? []).map(({ test }) => test);
  const output = { stdout: "", stderr: "" };
  const restore = capture(output);
  try {
    let module: Record<string, unknown> = {};
    let suite: string | undefined;
    try {
      module = (await import(pathToFileURL(file).href)) as Record<string, unknown>;
    } catch (error) {
      suite = message(error);
    }
    const unnamed = Object.keys(module).filter((name) => typeof module[name] === "function" && !names.includes(name));
    const tests: SubjectRun["tests"] = [];
    for (const name of suite === undefined ? names : []) {
      const test = module[name];
      if (typeof test !== "function") {
        tests.push({ test: name, error: "the subject has no test of this name" });
        continue;
      }
      try {
        await (test as (context: SubjectContext) => unknown)(context);
        tests.push({ test: name });
      } catch (error) {
        tests.push({ test: name, error: message(error) });
      }
    }
    return { tests, unnamed, ...(suite === undefined ? {} : { suite }), stdout: output.stdout, stderr: output.stderr };
  } finally {
    restore();
  }
}

/** The contracts the run wrote, each file against the case's `contracts`; one it does not name fails it. */
async function checkContracts(testCase: Case, dir: string, failures: string[]): Promise<void> {
  const expected = testCase.expect?.contracts;
  const written = (await readdir(dir)).filter((name) => name.endsWith(".json")).sort();
  const documents: Record<string, Record<string, unknown>> = {};
  for (const name of written) {
    documents[name] = JSON.parse(await readFile(join(dir, name), "utf8")) as Record<string, unknown>;
  }
  await recordContracts(testCase, documents);
  if (expected === undefined) {
    return;
  }
  for (const name of written.filter((name) => !(name in expected))) {
    failures.push(`the run wrote '${name}', which the case does not name`);
  }
  for (const [name, contract] of Object.entries(expected)) {
    const document = documents[name];
    const wantWritten = contract.written ?? true;
    if (wantWritten !== (document !== undefined)) {
      failures.push(`'${name}' was ${document ? "" : "not "}written, and the case expects it ${wantWritten ? "" : "not "}to be`);
    }
    if (contract.content !== undefined && document !== undefined) {
      checkContent(contract.content, document, `'${name}'`, failures);
    }
  }
}

/**
 * Record mode (README §6): with `JANUS_CONFORMANCE_RECORD` set, writes each contract's content in
 * the form a case's `contracts` expects, to be read and reviewed against the row before it is
 * checked in.
 */
async function recordContracts(testCase: Case, documents: Record<string, Record<string, unknown>>): Promise<void> {
  const directory = process.env.JANUS_CONFORMANCE_RECORD;
  if (!directory || Object.keys(documents).length === 0) {
    return;
  }
  const contracts = Object.fromEntries(
    Object.entries(documents).map(([name, document]) => [name, { content: contentOf(document) }]),
  );
  await writeFile(join(directory, `${testCase.id.replace("/", "-")}.json`), `${JSON.stringify(contracts, null, 2)}\n`, "utf8");
}

/**
 * The run in a child Vitest (test/conformance/isolated.test.ts), whose standard error is the engine's
 * log as well as the child's own.
 */
function runIsolated(testCase: Case, dir: string): { run: SubjectRun; engineLog: string } {
  const out = `${dir}.run.json`;
  const sdk = resolve(import.meta.dirname, "../..");
  // Vitest marks its own workers through the environment, which a nested Vitest must not inherit. A
  // case about the engine's log says how much it logs itself, so no RUST_LOG or LOG_LEVEL decides it.
  const env = Object.fromEntries(
    Object.entries(process.env).filter(([name]) => !name.startsWith("VITEST") && name !== "RUST_LOG" && name !== "LOG_LEVEL"),
  );
  const ran = spawnSync(
    process.execPath,
    [join("node_modules", "vitest", "vitest.mjs"), "run", join("test", "conformance", "isolated.test.ts")],
    {
      cwd: sdk,
      env: { ...env, JANUS_CONFORMANCE_ISOLATED: testCase.id, JANUS_CONFORMANCE_DIR: dir, JANUS_CONFORMANCE_OUT: out },
      encoding: "utf8",
    },
  );
  if (!existsSync(out)) {
    throw new Error(`the child run wrote nothing to ${out}:\n${ran.stdout}\n${ran.stderr}`);
  }
  return { run: JSON.parse(readFileSync(out, "utf8")) as SubjectRun, engineLog: ran.stderr };
}

/** Replaces the console and the process's streams while a subject runs, keeping what they are given. */
function capture(output: { stdout: string; stderr: string }): () => void {
  const saved = {
    log: console.log,
    info: console.info,
    debug: console.debug,
    warn: console.warn,
    error: console.error,
    out: process.stdout.write,
    err: process.stderr.write,
  };
  const line = (stream: "stdout" | "stderr") => (...args: unknown[]) => {
    output[stream] += `${args.map((a) => (typeof a === "string" ? a : show(a))).join(" ")}\n`;
  };
  const write = (stream: "stdout" | "stderr") =>
    ((chunk: string | Uint8Array) => {
      output[stream] += typeof chunk === "string" ? chunk : Buffer.from(chunk).toString("utf8");
      return true;
    }) as typeof process.stdout.write;
  console.log = console.info = console.debug = line("stdout");
  console.warn = console.error = line("stderr");
  process.stdout.write = write("stdout");
  process.stderr.write = write("stderr");
  return () => {
    console.log = saved.log;
    console.info = saved.info;
    console.debug = saved.debug;
    console.warn = saved.warn;
    console.error = saved.error;
    process.stdout.write = saved.out;
    process.stderr.write = saved.err;
  };
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
