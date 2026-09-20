// The conformance report this SDK produces (README §5): one entry per case, which the
// suite checker (`cargo run -p pact_janus_conformance -- check`) reads to decide whether this
// language passed the suite — the claim ADR 0017 makes build-checkable.

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { repoRoot } from "./cases.js";
import type { CaseResult } from "./runner.js";

const SDK = { name: "pact-janus-typescript", version: "0.0.0", language: "typescript" };

export async function writeReport(cases: CaseResult[]): Promise<string> {
  const file = process.env.JANUS_CONFORMANCE_REPORT ?? join(repoRoot, "target/conformance/typescript.json");
  const report = {
    $format: "janus-conformance-report/1",
    sdk: SDK,
    cases: [...cases].sort((a, b) => a.id.localeCompare(b.id)),
  };
  await mkdir(dirname(file), { recursive: true });
  await writeFile(file, `${JSON.stringify(report, null, 2)}\n`, "utf8");
  return file;
}
