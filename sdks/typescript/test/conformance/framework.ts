// Runs a framework case (ADR 0078): each of its subjects in this language is a test file that Jest or
// Vitest runs. The file goes into a project of its own, with the SDK built and installed in it as a user
// installs it (test/package.ts), and the framework runs from its own command line, in a process of its
// own, with the project as its working directory. The framework's JSON report says how each test and
// the file ended. Each run is held to the case's data, as an in-process subject's is (subject.ts). Every
// run of one case writes the same contract, and its digest goes in the report. This file decides
// nothing a case means: what a framework reports is the framework's.

import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { cp, mkdir, mkdtemp, readdir, readFile, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { frameworkSubjects, type Case } from "./cases.js";
import { buildPackage } from "../package.js";
import { checkContracts, checkRun, type SubjectRun } from "./subject.js";

/** How one framework is run: its project's configuration, and its command line. */
interface Runner {
  /** The configuration file the project has, by name, for a subject file of this name. */
  config(file: string): { name: string; text: string };
  /** The framework's command line, after `node`, writing its JSON report to `report`. */
  args(config: string, report: string): string[];
  /** The file's own failure, beyond its tests, from the framework's report of the file. */
  suite(file: ReportedFile): string | undefined;
}

/** The members of a test file's entry in Jest's `--json` report, which Vitest's `json` reporter writes too. */
interface ReportedFile {
  status: string;
  message: string;
  assertionResults: { title: string; status: string; failureMessages: string[] }[];
}

const sdk = resolve(import.meta.dirname, "../..");

/** Jest's heading for a file's own failure: its after-all hook's, where the integration finalises. */
const JEST_SUITE_FAILED = "● Test suite failed to run";

const jest: Runner = {
  // No globals are injected, so the integration is shown to need none, and nothing is transformed: a
  // test file is what a user writes, against the installed package.
  config: (file) => ({
    name: "jest.config.cjs",
    text: `module.exports = ${JSON.stringify({ injectGlobals: false, testMatch: [`<rootDir>/${file}`], transform: {} })};\n`,
  }),
  // Jest loads an ES module test file only under Node's --experimental-vm-modules, as a user's run does.
  args: (config, report) => [
    "--experimental-vm-modules",
    "--disable-warning=ExperimentalWarning",
    join(sdk, "node_modules", "jest", "bin", "jest.js"),
    "--ci",
    "--json",
    "--config",
    config,
    "--outputFile",
    report,
  ],
  // Jest's message for a file holds its tests' failures too; the file's own comes under its heading.
  suite: (file) => {
    const at = file.message.indexOf(JEST_SUITE_FAILED);
    return at < 0 ? undefined : file.message.slice(at);
  },
};

const vitest: Runner = {
  config: (file) => ({ name: "vitest.config.mjs", text: `export default ${JSON.stringify({ test: { include: [file] } })};\n` }),
  args: (config, report) => [
    join(sdk, "node_modules", "vitest", "vitest.mjs"),
    "run",
    "--reporter=json",
    "--outputFile",
    report,
    "--config",
    config,
  ],
  // Vitest's message for a file is the file's own error alone.
  suite: (file) => (file.message ? file.message : undefined),
};

const RUNNERS: Record<string, Runner> = { "jest-commonjs": jest, "jest-esm": jest, vitest };

let installed: Promise<string> | undefined;

/** A directory with the SDK installed in its node_modules, built once per run; each subject's project is inside it. */
function installation(): Promise<string> {
  installed ??= (async () => {
    const root = await mkdtemp(join(tmpdir(), "janus-frameworks-"));
    await buildPackage(root, { esm: true });
    // pact-janus/vitest imports Vitest, which a project that runs Vitest has installed: this one.
    await symlink(resolve(sdk, "node_modules", "vitest"), join(root, "node_modules", "vitest"), "junction");
    return root;
  })();
  return installed;
}

/** Runs each of the case's subjects in this language; the digest of the contract they wrote, if they wrote one. */
export async function runFramework(testCase: Case, failures: string[]): Promise<string | undefined> {
  const subjects = frameworkSubjects(testCase);
  if (subjects.length === 0) {
    failures.push("the case has no subject for a framework of this language");
    return undefined;
  }
  const digests = new Map<string, string>();
  for (const { framework, file } of subjects) {
    const runner = RUNNERS[framework];
    const own: string[] = [];
    if (runner === undefined) {
      own.push(`this driver cannot run ${framework}, a framework frameworks.json gives this language`);
    } else {
      const project = await mkdtemp(join(await installation(), `${framework}-`));
      const run = await runIn(project, testCase, file, runner, own);
      if (run !== undefined) {
        checkRun(testCase, run, own);
        await checkContracts(testCase, join(project, "contracts"), own);
        for (const [name, digest] of await contractDigests(join(project, "contracts"))) {
          digests.set(`${framework} ${name}`, digest);
        }
      }
    }
    failures.push(...own.map((line) => `[${framework}] ${line}`));
  }
  // ADR 0078 decision 4: one case's subjects write one contract, whichever framework ran them.
  const distinct = new Set(digests.values());
  if (distinct.size > 1) {
    failures.push(`the frameworks wrote different contracts: ${[...digests].map(([by, digest]) => `${by} ${digest}`).join(", ")}`);
  }
  return distinct.size === 1 ? [...distinct][0] : undefined;
}

/** Sets the project up, runs the framework in it, and reads its report of the subject's file. */
async function runIn(project: string, testCase: Case, subject: string, runner: Runner, failures: string[]): Promise<SubjectRun | undefined> {
  const file = basename(subject);
  await cp(subject, join(project, file));
  for (const [path, content] of Object.entries(testCase.project ?? {})) {
    await mkdir(dirname(join(project, path)), { recursive: true });
    await writeFile(join(project, path), typeof content === "string" ? content : `${JSON.stringify(content, null, 2)}\n`, "utf8");
  }
  const config = runner.config(file);
  await writeFile(join(project, config.name), config.text, "utf8");

  const report = join(project, "report.json");
  const ran = await node(runner.args(config.name, report), project);
  if (!existsSync(report)) {
    failures.push(`the framework wrote no report (exit ${ran.status}):\n${ran.stdout}\n${ran.stderr}`);
    return undefined;
  }
  const reported = (JSON.parse(await readFile(report, "utf8")) as { testResults: ReportedFile[] }).testResults[0];
  if (reported === undefined) {
    failures.push(`the framework ran no test file (exit ${ran.status}):\n${ran.stdout}\n${ran.stderr}`);
    return undefined;
  }
  const names = (testCase.expect?.tests ?? []).map(({ test }) => test);
  const tests = reported.assertionResults.map(({ title, status, failureMessages }) =>
    status === "passed" ? { test: title } : { test: title, error: failureMessages.join("\n") || status },
  );
  const suite = runner.suite(reported);
  const run: SubjectRun = {
    tests,
    unnamed: tests.map(({ test }) => test).filter((test) => !names.includes(test)),
    ...(suite === undefined ? {} : { suite }),
    stdout: ran.stdout,
    stderr: ran.stderr,
  };
  for (const name of names.filter((name) => !tests.some(({ test }) => test === name))) {
    failures.push(`the framework ran no test '${name}'; it ran ${JSON.stringify(tests.map(({ test }) => test))}`);
  }
  // A failed test or suite fails the run, as a CI job sees it: by the framework's exit status.
  const expected = testCase.expect ?? {};
  const shouldFail = expected.suite?.fails !== undefined || (expected.tests ?? []).some(({ fails }) => fails !== undefined);
  if (shouldFail !== (ran.status !== 0)) {
    failures.push(`the framework exited ${ran.status}, and the case expects the run to ${shouldFail ? "fail" : "pass"}`);
  }
  return run;
}

/** Each contract file the run wrote, by name, as the SHA-256 of its bytes. */
async function contractDigests(dir: string): Promise<[string, string][]> {
  if (!existsSync(dir)) {
    return [];
  }
  const names = (await readdir(dir)).filter((name) => name.endsWith(".json")).sort();
  return Promise.all(
    names.map(async (name): Promise<[string, string]> => [
      name,
      createHash("sha256").update(await readFile(join(dir, name))).digest("hex"),
    ]),
  );
}

/** Runs node with `args` in `cwd`, as a user's shell would: the environment as it is, but for Vitest's own markers. */
function node(args: string[], cwd: string): Promise<{ status: number | null; stdout: string; stderr: string }> {
  // Vitest marks its own workers through the environment, which a nested Vitest must not inherit.
  const env = Object.fromEntries(Object.entries(process.env).filter(([name]) => !name.startsWith("VITEST")));
  return new Promise((done, fail) => {
    const child = spawn(process.execPath, args, { cwd, env, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8").on("data", (chunk: string) => (stdout += chunk));
    child.stderr.setEncoding("utf8").on("data", (chunk: string) => (stderr += chunk));
    child.on("error", fail);
    child.on("close", (status) => done({ status, stdout, stderr }));
  });
}
