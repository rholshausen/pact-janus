import { resolve } from "node:path";
import { defineConfig } from "vitest/config";

const repoRoot = resolve(import.meta.dirname, "../..");
// What `cargo build` calls the engine on this platform.
const engine = process.platform === "win32" ? "janus-engine.exe" : "janus-engine";

export default defineConfig({
  test: {
    include: ["test/**/*.test.ts"],
    // Builds janus-engine once, so no test runs against a stale engine.
    globalSetup: ["test/build-engine.ts"],
    env: {
      JANUS_ENGINE: process.env.JANUS_ENGINE ?? resolve(repoRoot, "target/debug", engine),
    },
  },
});
