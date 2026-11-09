// What the conformance suite's framework cases (ADR 0078) cannot show, each needing more than one test
// file or a facade: the suites in test/frameworks, run by Jest and by Vitest in a scratch project the
// built package is installed in, as a user's suites run. Each runner's own JSON report says what passed.
// Test files Vitest runs in workers of their own merge into one contract (S2.11, ADR 0065), and pact-js's
// verifier, from pact-janus/verifier, runs under Jest from CommonJS, as pact-js's provider tests do.

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { cp, mkdtemp, readFile, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import { specRoot } from "./conformance/cases.js";
import { buildPackage } from "./package.js";

/** Jest's `--json` report, which Vitest's `json` reporter writes too: the members read here. */
type Report = {
  testResults: {
    name: string;
    status: string;
    message: string;
    assertionResults: { title: string; status: string }[];
  }[];
};

/** A report by test file: its status, its message, and each test's status. */
function byFile(report: Report) {
  return Object.fromEntries(
    report.testResults.map((file) => [
      basename(file.name),
      {
        status: file.status,
        message: file.message,
        tests: Object.fromEntries(file.assertionResults.map((t) => [t.title, t.status])),
      },
    ]),
  );
}

let root: string;

/** Runs `script` in the scratch project, and reads the JSON report it wrote. */
async function run(script: string, args: string[]): Promise<ReturnType<typeof byFile>> {
  const out = join(root, `${basename(script)}.json`);
  // Vitest marks its own workers through the environment, which a nested Vitest must not inherit.
  const env = Object.fromEntries(Object.entries(process.env).filter(([name]) => !name.startsWith("VITEST")));
  const ran = spawnSync(process.execPath, [resolve(script), ...args, "--outputFile", out], {
    cwd: root,
    env,
    encoding: "utf8",
  });
  if (!existsSync(out)) {
    throw new Error(`${script} wrote no report:\n${ran.stdout}\n${ran.stderr}`);
  }
  return byFile(JSON.parse(await readFile(out, "utf8")) as Report);
}

beforeAll(async () => {
  root = await mkdtemp(join(tmpdir(), "pact-janus-frameworks-"));
  await buildPackage(root, { esm: true });
  await cp(join("test", "frameworks"), root, { recursive: true });
  await cp(join(specRoot, "corpora", "verifier", "examples", "pact-js", "http", "pact.json"), join(root, "pact.json"));
  // pact-janus/vitest imports Vitest, which the project has installed: this one.
  await symlink(resolve("node_modules", "vitest"), join(root, "node_modules", "vitest"), "junction");
}, 120_000);

describe("under Jest", () => {
  it("runs pact-js's provider test from CommonJS", async () => {
    const files = await run(join("node_modules", "jest", "bin", "jest.js"), ["--ci", "--json", "--config", "jest.config.cjs"]);
    expect(files["pact-js-verifier.test.cjs"]).toMatchObject({
      status: "passed",
      tests: { "satisfies UserConsumer's pact": "passed" },
    });
  }, 120_000);
});

// Staged plan S2.11, ADR 0065: test files in workers of their own, each with an engine of its own,
// merge into one contract file.
describe("under Vitest", () => {
  let files: ReturnType<typeof byFile>;

  beforeAll(async () => {
    files = await run(join("node_modules", "vitest", "vitest.mjs"), ["run", "--reporter=json", "--config", "vitest.config.mjs"]);
  }, 120_000);

  it("merges two files' interactions into one contract, the bytes one file recording both writes", async () => {
    for (const file of ["merged-orders.test.ts", "merged-deletes.test.ts", "merged-together.test.ts"]) {
      expect(files[file]?.status, file).toBe("passed");
    }
    const merged = await readFile(join(root, "contracts", "vitest-merged-orders-api.janus.json"), "utf8");
    const together = await readFile(join(root, "contracts", "together", "vitest-merged-orders-api.janus.json"), "utf8");
    expect(merged).toBe(together);
    const recorded = JSON.parse(merged).interactions.map((i: { description: string }) => i.description);
    expect(recorded).toEqual(["delete an order", "get an order"]);
  });

  it("fails the second file to record an interaction differently, naming it, and keeps the first's", async () => {
    const outcomes = ["conflict-ok.test.ts", "conflict-accepted.test.ts"].map((file) => files[file]);
    expect(outcomes.map((outcome) => outcome?.status).sort()).toEqual(["failed", "passed"]);
    const failed = outcomes.find((outcome) => outcome?.status === "failed");
    expect(failed?.message).toContain("'create an order'");
    const kept = JSON.parse(await readFile(join(root, "contracts", "vitest-conflicting-orders-api.janus.json"), "utf8"));
    expect(kept.interactions).toHaveLength(1);
  });
});
