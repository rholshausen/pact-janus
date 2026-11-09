// The Jest project test/frameworks.test.ts runs, as a user's would be configured. Nothing is
// transformed: the suite is what a pact-js user writes, against the built package.
module.exports = {
  injectGlobals: false,
  roots: ["<rootDir>/jest"],
  testMatch: ["**/*.test.cjs"],
  transform: {},
};
