# 0061 — The engine's logs are the runtime's own, set and shown by its logging configuration

- **Status**: accepted
- **Date**: 2026-10-31
- **Plan tasks**: S1.37; found verifying PactFlow's `example-provider-springboot` through the JVM SDK,
  where `RUST_LOG=debug` in `build.gradle` showed no engine log anywhere a JVM user looks
- **Evidence**: that project's test run. The engine's debug lines reached Gradle's console as the
  worker process's raw stderr, not its test report (0 of 677 `DEBUG` lines there were the engine's)
  and not IntelliJ's test output. Its `logging.level` settings in `application.properties` could not
  reach them. The pact-js facade's `logLevel`, accepted with no effect
  (`Documentation/provider-test-migration.md`); `cli/src/logging.rs`

## Context

The engine logs through `tracing`, to stderr, at the level `RUST_LOG` names. Both SDKs start it with
its stderr inherited, so its logs go wherever the test process's raw stderr goes, and `RUST_LOG` is
the only way to set how much it logs. Neither is how anyone configures logging in a test.

- A pact-js user sets `logLevel` in the test's options, or `LOG_LEVEL` in the environment. The
  facade accepts `logLevel` and ignores it. Node prints a child's inherited stderr on the terminal,
  so once `RUST_LOG` is set, the logs are at least visible.
- A JVM user configures a logging framework: Logback, Log4j2, or Spring Boot's `logging.level.*`
  over either, all reached through SLF4J, as pact-jvm's own logs are. An inherited stderr bypasses
  that framework entirely. Under Gradle it lands in the console beside the tests, not in any test's
  output. Under an IDE it is often nowhere at all.

## Decision

1. **The engine can write its events as JSON lines.** With `JANUS_LOG_FORMAT=json`, `janus-engine`
   and `janus` write each event on stderr as one JSON object: `timestamp`, `level` (`TRACE` to
   `ERROR`), `target`, and `fields`, whose `message` is the event's message. Spans are left out. The
   same filter applies as for text, so a credential is held back in both. An SDK that forwards events
   reads these members, and never parses text written for a person.
2. **`RUST_LOG` given is `RUST_LOG` used.** An SDK passes a `RUST_LOG` from the environment to the
   engine as it is, directives by target included. Anything it derives applies only when there is
   none.
3. **The TypeScript SDK takes the level as pact-js does.** Its level is the `logLevel` option of
   `Janus` and `Verifier`, which the pact-js facade's `logLevel` now passes through. Without that
   option, it is `LOG_LEVEL`, and failing both, `warn`. Each is spelled as pact-js spells it, in any
   case: `trace`, `debug`, `info`, `warn`, `error`, with `fatal` read as `error`, and `silent` and
   `none` as `off`. An unknown level fails the call that would start the engine, naming the option or
   the variable it came from. The engine's stderr stays inherited, which is where Node users already
   see a child's output.
4. **The JVM SDK logs the engine's events through SLF4J**, to one logger, `io.pact.janus.engine`.
   - The engine runs at the most detailed level that logger has enabled, so
     `logging.level.io.pact.janus.engine=debug`, or a Logback or Log4j2 logger entry, sets it.
   - The SDK pipes the engine's stderr and logs each event at its own level, as
     `target: message key=value …`. It leaves out the `log.*` fields a library logging through the
     `log` crate adds, as the text format does.
   - A line that is not an event goes to the test's stderr as it is, where an inherited stderr
     showed it. That is a crash or a framing error, or the text of an engine older than
     `JANUS_LOG_FORMAT` that `JANUS_ENGINE` names. Logged at one level, an old engine's every line
     would wear that level whatever its own was, and its own is only in the text.
   - `slf4j-api` becomes a dependency of the SDK.
   - With no SLF4J provider bound, there is nowhere to forward to. The engine's stderr is then
     inherited, at `warn` unless `RUST_LOG` says otherwise.
5. **One logger, not one per target.** A target is a Rust module path, and `RUST_LOG` can only be
   derived from one logger's level: a per-target logger set more verbose than its parent would
   never reach the engine. The target is in each message instead.

## Alternatives considered

- **Log events as protocol frames.** Every embedding would get them, the WASM one included, and
  stderr would be left to crashes. But the subscriber would then write to the frame stream beside
  `dispatch`, and the JVM SDK reads frames synchronously, so it is a protocol change for something a
  JSON line on stderr carries.
- **The JDK's `System.Logger` on the JVM.** It needs no dependency, but it reaches JUL by default.
  Under Spring Boot, JUL is bridged to SLF4J, and a level asked of it through the bridge does not
  reliably reflect `logging.level`. pact-jvm users already have SLF4J.
- **Parse the engine's text in the SDK.** That is the engine's display text, which ADR 0054 keeps
  out of every client's hands.
- **Forward the TypeScript SDK's events through `console`.** Vitest and Jest would then attribute
  them to a test. But pact-js wrote its logs to stdout, not through a test runner's console, and
  nobody has asked for it yet.

## Consequences

Easier: a JVM provider test shows the engine's logs in its own log format, in its test report and
in the IDE, set where every other logger in the project is set. A pact-js user's `logLevel` does
what it did before.

Harder: the JVM SDK has a thread per engine that reads its stderr, and a dependency. The engine's
JSON members are now an interface an SDK reads: renaming one is a change file.

Committed to: `JANUS_LOG_FORMAT=json` and its members, the logger name `io.pact.janus.engine`, and
`logLevel` and `LOG_LEVEL` in the TypeScript SDK.

Tripwires:

- **A user needs one part of the engine at a different level** and `RUST_LOG`'s directives are not
  enough: per-target loggers, with `RUST_LOG` built from each one's level.
- **An embedding other than the subprocess needs the logs** (gate W, the WASM engine): they become
  protocol events, and the JSON line is retired.
