## 0.1.1 (2026-10-31)

The SDK runs janus-engine 0.4.3, where it ran 0.4.2: [its release](https://github.com/rholshausen/pact-janus/releases/tag/v0.4.3) says what changed.

### Features

#### The engine's logs are set and shown as the test's own

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

## 0.1.0 (2026-10-31)

### Features

#### The SDK is released, and fetches the engine it was released with

The SDK is released as files on a GitHub release of its own: `pact-janus-<version>.tgz`, installed by
its URL, or the JVM SDK's `io.pact.janus:janus-sdk` and `janus-bindings` jars, read by Gradle through
an `ivy` repository (ADR 0060). The JVM modules were `io.pact.janus:sdk` and `io.pact.janus:bindings`.

Told no engine to run, the SDK runs the `janus-engine` release it pins. It fetches the engine on
first use into `JANUS_ENGINE_CACHE`, or by default `pact-janus/engines` in the user's cache directory.
It fetches from `JANUS_ENGINE_MIRROR` when that is set, and refuses any file whose SHA-256 is not the
pinned one. `JANUS_ENGINE` still names another engine. The SDK no longer runs a `janus-engine` from
the `PATH`; name it in `JANUS_ENGINE` instead.

The engine it pins is janus-engine 0.4.2, the first release with everything the SDK asks of it: the
v4 `statusCode` matcher, and message producers served from the test.
