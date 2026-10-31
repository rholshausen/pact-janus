// Vitest global setup: the engine these tests drive. JANUS_ENGINE names one already; with
// JANUS_ENGINE_RELEASE set it is the pinned release, fetched and checked against its digest
// (src/engine/release.ts, as an installed SDK fetches it); otherwise it is built from this tree, which is what a change to the engine
// and an SDK in one commit needs.
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { PIN, resolveEngine } from "../src/engine/release.js";

export default async function setup(): Promise<void> {
  if (process.env.JANUS_ENGINE) {
    return;
  }
  if (process.env.JANUS_ENGINE_RELEASE) {
    console.log(`janus-engine ${PIN.version} (released): ${await resolveEngine()}`);
    return;
  }
  execFileSync("cargo", ["build", "-p", "pact_janus_cli", "--bin", "janus-engine"], {
    cwd: resolve(import.meta.dirname, "../../.."),
    stdio: "inherit",
  });
}
