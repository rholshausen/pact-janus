// One conformance case's subject, run alone in a child Vitest that test/conformance/subject.ts starts,
// so that the engine's standard error, which it inherits, reaches the parent (ADR 0076). Skipped in
// every other run: the parent names the case, where its contracts go and where to write the run.

import { writeFile } from "node:fs/promises";
import { describe, it } from "vitest";
import { loadCorpus, subjectPath } from "./cases.js";
import { runTests } from "./subject.js";

const id = process.env.JANUS_CONFORMANCE_ISOLATED;

describe.skipIf(id === undefined)("a conformance case's subject, alone", () => {
  it("runs", async () => {
    const testCase = loadCorpus().find((c) => c.id === id);
    const file = testCase && subjectPath(testCase);
    if (testCase === undefined || file === undefined) {
      throw new Error(`no case '${id}' with a TypeScript subject`);
    }
    const run = await runTests(testCase, file, { dir: process.env.JANUS_CONFORMANCE_DIR ?? "" });
    await writeFile(process.env.JANUS_CONFORMANCE_OUT ?? "", JSON.stringify(run), "utf8");
  }, 60_000);
});
