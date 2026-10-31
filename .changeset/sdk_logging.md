---
typescript-sdk: minor
jvm-sdk: minor
---

# The engine's logs are set and shown as the test's own

How much the engine logs is now set the way the test's language sets any logging, and a `RUST_LOG`
that is set still wins (ADR 0061).

- **TypeScript:** the `logLevel` option of `Janus` and `Verifier`, else `LOG_LEVEL`, else `warn`, as
  in pact-js. The `pact-janus/verifier` facade passes pact-js's `logLevel` through, where it had no
  effect.
- **JVM:** the engine's events are logged through SLF4J to the logger `io.pact.janus.engine`, at their
  own levels, and the engine logs at the most detailed level that logger shows. Under Spring Boot,
  `logging.level.io.pact.janus.engine=debug` in `application.properties` shows them in the test's own
  log and report. The SDK now depends on `slf4j-api`. With no SLF4J provider bound, the engine's
  stderr is inherited, at `warn`.
