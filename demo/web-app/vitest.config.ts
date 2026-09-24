// The Janus SDK is not published, so `pact-janus` points at its source in this repo. That is the only
// thing here a real project would not have: it would `npm install pact-janus` instead.
import { resolve } from "node:path";
import { defineConfig } from "vitest/config";

// JANUS_SDK lets demo/run.sh run a scratch copy of this app from outside the repo.
const sdk = process.env.JANUS_SDK ?? resolve(import.meta.dirname, "../../sdks/typescript/src");

export default defineConfig({
  resolve: {
    alias: { "pact-janus/vitest": `${sdk}/vitest.ts`, "pact-janus": `${sdk}/index.ts` },
    // The SDK's own source would otherwise load its own copy of Vitest, whose afterAll is not this run's.
    dedupe: ["vitest"],
  },
  test: { include: ["test/**/*.test.ts"] },
});
