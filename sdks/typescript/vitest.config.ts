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
  resolve: {
    // A conformance case's subject (ADR 0076) imports the SDK as a user's test does, by its package
    // name, from outside this directory: these are its entry points, from this tree's sources.
    alias: [
      { find: /^pact-janus$/, replacement: resolve(import.meta.dirname, "src/index.ts") },
      { find: /^pact-janus\/pact$/, replacement: resolve(import.meta.dirname, "src/facade/pact.ts") },
      { find: /^pact-janus\/verifier$/, replacement: resolve(import.meta.dirname, "src/facade/verifier.ts") },
    ],
  },
  test: {
    include: ["test/**/*.test.ts"],
    // The suites test/frameworks.test.ts runs under Jest and Vitest, as a user's project runs them.
    exclude: ["test/frameworks/**", "**/node_modules/**", "**/.git/**"],
    // Builds or fetches janus-engine once, so no test runs against a stale engine.
    globalSetup: ["test/build-engine.ts"],
    env: {
      JANUS_ENGINE: engineUnderTest(),
    },
  },
});
