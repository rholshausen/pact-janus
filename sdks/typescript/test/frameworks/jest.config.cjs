// The Jest project test/frameworks.test.ts runs, as a user's would be configured: test files in
// CommonJS (.cjs) and as ES modules (.mjs), which Jest runs with Node's --experimental-vm-modules.
// Nothing is transformed: the suites are what a user writes, against the built package. No globals
// are injected, so pact-janus/jest is shown to need none: it takes afterAll from @jest/globals.
module.exports = {
  injectGlobals: false,
  roots: ["<rootDir>/jest"],
  testMatch: ["**/*.test.cjs", "**/*.test.mjs"],
  transform: {},
};
