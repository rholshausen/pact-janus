// Vitest global setup: build the engine these tests drive, unless JANUS_ENGINE names one already.
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";

export default function setup(): void {
  if (process.env.JANUS_ENGINE) {
    return;
  }
  execFileSync("cargo", ["build", "-p", "pact_janus_cli", "--bin", "janus-engine"], {
    cwd: resolve(import.meta.dirname, "../../.."),
    stdio: "inherit",
  });
}
