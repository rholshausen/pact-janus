// The test-framework integrations (STYLE.md, "Test-framework integration"; staged plan S2.4a): the
// suites in test/frameworks, run by Jest (from CommonJS and as ES modules) and by Vitest in a scratch
// project the built package is installed in, as a user's suites run. Each runner's own JSON report
// says what passed: a variant that fails, fails the test that ran it, and a withheld contract fails
// the suite while the passing suites' contracts are written.

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
    assertionResults: { title: string; status: string; failureMessages: string[] }[];
  }[];
};

/** A report by test file: its status, its message, and each test's status and failure. */
function byFile(report: Report) {
  return Object.fromEntries(
    report.testResults.map((file) => [
      basename(file.name),
      {
        status: file.status,
        message: file.message,
        tests: Object.fromEntries(file.assertionResults.map((t) => [t.title, t.status])),
        failures: file.assertionResults.flatMap((t) => t.failureMessages).join("\n"),
      },
    ]),
  );
}

let root: string;

/** Runs `script` in the scratch project, and reads the JSON report it wrote. */
async function run(script: string, args: string[], flags: string[]): Promise<{ status: number | null; report: Report }> {
  const out = join(root, `${basename(script)}.json`);
  // Vitest marks its own workers through the environment, which a nested Vitest must not inherit.
  const env = Object.fromEntries(Object.entries(process.env).filter(([name]) => !name.startsWith("VITEST")));
  const ran = spawnSync(process.execPath, [...flags, resolve(script), ...args, "--outputFile", out], {
    cwd: root,
    env,
    encoding: "utf8",
  });
  if (!existsSync(out)) {
    throw new Error(`${script} wrote no report:\n${ran.stdout}\n${ran.stderr}`);
  }
  return { status: ran.status, report: JSON.parse(await readFile(out, "utf8")) as Report };
}

async function contract(consumer: string): Promise<string | undefined> {
  const path = join(root, "contracts", `${consumer}-orders-api.janus.json`);
  return existsSync(path) ? readFile(path, "utf8") : undefined;
}

beforeAll(async () => {
  root = await mkdtemp(join(tmpdir(), "pact-janus-frameworks-"));
  await buildPackage(root, { esm: true });
  await cp(join("test", "frameworks"), root, { recursive: true });
  await cp(join(specRoot, "corpora", "verifier", "examples", "pact-js", "http", "pact.json"), join(root, "pact.json"));
  // pact-janus/vitest imports Vitest, which the project has installed: this one.
  await symlink(resolve("node_modules", "vitest"), join(root, "node_modules", "vitest"), "junction");
}, 120_000);

describe.each([
  {
    framework: "Jest",
    script: join("node_modules", "jest", "bin", "jest.js"),
    args: ["--ci", "--json", "--config", "jest.config.cjs"],
    flags: ["--experimental-vm-modules", "--disable-warning=ExperimentalWarning"],
    suites: [
      { files: "jest/*.test.cjs", ext: ".test.cjs", consumer: "jest-cjs" },
      { files: "jest/*.test.mjs", ext: ".test.mjs", consumer: "jest-esm" },
    ],
  },
  {
    framework: "Vitest",
    script: join("node_modules", "vitest", "vitest.mjs"),
    args: ["run", "--reporter=json", "--config", "vitest.config.mjs"],
    flags: [],
    suites: [{ files: "vitest/*.test.ts", ext: ".test.ts", consumer: "vitest" }],
  },
])("under $framework", ({ script, args, flags, suites }) => {
  let files: ReturnType<typeof byFile>;
  let status: number | null;

  beforeAll(async () => {
    const ran = await run(script, args, flags);
    status = ran.status;
    files = byFile(ran.report);
  }, 120_000);

  it("fails the run, for the suites whose contracts are withheld", () => {
    expect(status).toBe(1);
  });

  for (const { ext, consumer } of suites) {
    describe(`from *${ext}`, () => {
      it("passes a consumer suite, and writes its contract", async () => {
        expect(files[`consumer${ext}`]).toMatchObject({ status: "passed", tests: { "gets an order": "passed" } });
        const written = JSON.parse((await contract(consumer)) ?? "{}");
        expect(written.interactions.map((i: { description: string }) => i.description)).toEqual(["get an order"]);
      });

      it("fails the test that ran a failing variant, and only that one", () => {
        const withheld = files[`withheld${ext}`];
        expect(withheld?.tests).toEqual({ "reads when an order shipped": "failed", "deletes an order": "passed" });
        expect(withheld?.failures).toContain("shippedAt=absent");
      });

      it("fails the suite whose contract is withheld, and writes none", async () => {
        const withheld = files[`withheld${ext}`];
        expect(withheld?.status).toBe("failed");
        expect(withheld?.message).toContain("No contract written:");
        expect(await contract(`${consumer}-careless`)).toBeUndefined();
      });

      it("passes a provider test", () => {
        expect(files[`provider${ext}`]).toMatchObject({
          status: "passed",
          tests: { "satisfies UserConsumer's pact": "passed" },
        });
      });
    });
  }
});

it("writes the same contract whichever framework and module system ran the suite", async () => {
  const consumers = ["jest-cjs", "jest-esm", "vitest"];
  const [first, ...rest] = await Promise.all(
    consumers.map(async (consumer) => (await contract(consumer))?.replaceAll(`"${consumer}"`, '"web-app"')),
  );
  expect(first).toBeDefined();
  expect(rest).toEqual([first, first]);
});
