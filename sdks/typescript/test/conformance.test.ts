// The SDK conformance suite (plan task 6.4, SDK spec §7, ADR 0017), run in TypeScript: every case
// under `conformance/cases`, one Vitest test each, against this SDK and — for the `live` cases —
// the engine `JANUS_ENGINE` names. The cases are the same files the JVM driver runs; what is
// TypeScript's here is only how the DSL is spelled (test/conformance/dsl.ts).
//
// The run writes a report the suite checker reads, so "this SDK is conformant" is a claim the
// build makes, not one this file asserts in prose.

import { afterAll, describe, it } from "vitest";
import { loadCases } from "./conformance/cases.js";
import { writeReport } from "./conformance/report.js";
import { runCase, type CaseResult } from "./conformance/runner.js";

const cases = loadCases();
const results: CaseResult[] = [];

afterAll(async () => {
  // Every case's outcome, including the ones that failed: a report missing a case is itself a
  // finding, and the checker says so.
  await writeReport(results);
});

describe("the SDK conformance suite", () => {
  for (const testCase of cases) {
    it(
      `${testCase.id}: ${testCase.title} [${testCase.covers.join(", ")}]`,
      async () => {
        const result = await runCase(testCase);
        results.push(result);
        if (result.status !== "passed") {
          throw new Error(`${testCase.id} — ${testCase.title}\n${result.detail}`);
        }
      },
      30_000,
    );
  }
});
