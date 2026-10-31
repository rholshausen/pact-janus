import { resolve } from "node:path";
import { defineConfig } from "vitest/config";
import { enginePath } from "./src/engine/release.js";

const repoRoot = resolve(import.meta.dirname, "../..");
// What `cargo build` calls the engine on this platform.
const engine = process.platform === "win32" ? "janus-engine.exe" : "janus-engine";

// Which engine the tests drive, in the order test/build-engine.ts makes one available: the one
// JANUS_ENGINE names, the pinned release when JANUS_ENGINE_RELEASE asks for it, or this tree's build.
function engineUnderTest(): string {
  if (process.env.JANUS_ENGINE) {
    return process.env.JANUS_ENGINE;
  }
  if (process.env.JANUS_ENGINE_RELEASE) {
    return enginePath();
  }
  return resolve(repoRoot, "target/debug", engine);
}

export default defineConfig({
  test: {
    include: ["test/**/*.test.ts"],
    // Builds or fetches janus-engine once, so no test runs against a stale engine.
    globalSetup: ["test/build-engine.ts"],
    env: {
      JANUS_ENGINE: engineUnderTest(),
    },
  },
});
