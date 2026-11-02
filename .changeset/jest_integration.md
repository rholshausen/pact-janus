---
typescript-sdk: minor
---

# `pact-janus/jest`, the Jest integration

`useJanus(config)` from `pact-janus/jest` does under Jest what it does under Vitest: it creates the
suite's `Janus` and runs `finalise` in an `afterAll`, so the contract is written after the file's
last test and a withheld contract fails the file. It is CommonJS and ES modules alike, takes
`afterAll` from `@jest/globals`, so it works with `injectGlobals: false`, and is tested on Jest 30,
which it names as an optional peer dependency. The provider side already ran under Jest; it is now
tested there too, from CommonJS and as ES modules.
