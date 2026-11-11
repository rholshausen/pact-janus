# 0084 — Tell what the mock made of each request while the test runs, and make each variant a test where the framework can

- **Status**: accepted
- **Date**: 2026-11-11
- **Plan tasks**: S3.5
- **Evidence**: the variant ergonomics report (`Documentation/prototype/variant-ergonomics-report.md`),
  recommendations 1–3; engine-protocol spec §9 (events, ADR 0005), whose §9.1 named consumer-session
  transport activity as the next stream; `janus-engine`'s frame loop, which answers one frame at a
  time; a probe of Vitest 5 and Jest 30, which register tests after an `await` at a test file's top
  level (Vitest also in an async `describe`), except Jest from CommonJS, which refuses an async
  `describe` and has no top-level `await`;
  [ADR 0064](0064-report-each-variants-test-verdict-to-the-engine-and-answer-with-its-result.md) and
  [ADR 0078](0078-run-a-test-framework-integrations-conformance-cases-in-a-project-of-their-own.md)

## Context

The variant ergonomics report found that a failing variant is identifiable only when the test framework
names it, and recommended three things: one test per variant where the framework registers tests
dynamically, a per-variant result list where it does not, and the exchange loop's match decision as an
event a host can surface while the run is going.

The second exists. `execute` runs every variant, and its one failure names each failed variant by label
and id with the engine's mismatches (ADR 0064). The first exists for JUnit 5, whose `@TestFactory`
returns `janus.variants`. Spock, Vitest and Jest had `execute` alone, and the third was a `tracing`
line on the engine's stderr, which says what the mock rejected without saying which test was running.
A test that hangs on a rejected request, or times out, never reaches `report-variant`, so it never
learns the mismatches at all.

## Decision

1. **A consumer session may have an event stream.** `consumer-session/create` takes `events: true` and
   answers with `stream`. Each request a started transport receives is one `consumer-session/exchange`
   event, once its outcome is recorded: the transport instance, `status` (`matched`, `mismatched`,
   `undelivered`, or `unmatched` when nothing was armed), and but for `unmatched` the `handle` and
   `variant` it went to, with `mismatches`, redacted as `report-variant`'s are. A host that asked says
   what each was, so the engine logs its own account at `debug` for that session, where it was `warn`.
2. **`finalise` ends the stream without waiting on the host.** It appends `consumer-session/finished`
   with `last: true`, whatever the stream holds, and an exchange waiting for room is released, its event
   dropped. Every other stream blocks its producer when full (§9.3), and so does this one while the
   session lives; but `finalise` runs on the dispatch thread, which must never wait for a host that may
   not be polling.
3. **The SDKs drain the stream while each closure runs.** Every 100 ms, and once more after the closure
   ends, before `report-variant`, with `wait-ms: 0`: `janus-engine` answers one frame at a time, so a
   long poll would hold the report behind it. Each event goes to the language's logging, a match at
   debug and anything else at warn, as `'<description>' [<label> [<id>]]: the request did not match`,
   then each mismatch's path and message. A poll that fails stops the polling for that variant, said at
   warn: it is not the test's verdict. The JVM SDK asks only when SLF4J has a provider, since with none
   the events would be logged to nobody, and the engine's warning on its stderr says it instead.
4. **Each variant is a test of its own where the framework can register one once the engine has
   selected the variants.** JUnit 5 keeps its `@TestFactory`. Spock's `where:` block takes
   `janus.variants(interaction)`, and its body runs a row with `janus.run(variant) { mock, v -> }`.
   Vitest and Jest take `await janus.variants(interaction, closure)` at a test file's top level (Vitest
   also in an async `describe`), registering one test per variant with the framework's own `test`. Each
   is named `label [id]`, or the id alone when they are the same, and fails with the closure's own
   error or the engine's verdict. Jest from CommonJS cannot, and has `execute` alone.
5. **No case counts the polls.** Where they fall depends on how long a closure runs. A scripted engine
   names no stream, so the SDK polls nothing there, and a `live` case with a subject in each language
   checks what each logs. For TypeScript, whose logging is the console, `engine-log` includes what the
   SDK handed it.

## Alternatives considered

- **Long-polling the stream.** One poll held open per closure would carry each event the moment it is
  told, but the engine answers frames in turn, so every `report-variant` would wait out the poll. A
  poll that never waits costs a round trip each 100 ms.
- **Push delivery (`push-events`).** It is the stdio pipe's latency upgrade (§9.5), and neither SDK's
  pipe reads unsolicited frames. Polling works on every pipe, as §9.5 requires hosts to implement
  anyway.
- **Leave it to the engine's log.** The line exists, but names a handle rather than the interaction and
  the test, and says it in the engine's words, on a stream the test runner may not show beside the
  test. The event is structured, so the SDK names what the test author named.
- **Drop the event when the stream is full.** It would keep a host that walked away from stalling the
  mock, but loss is never the pressure valve (§9.3). The cap is a thousand undrained requests, and an
  SDK drains every variant.
- **Register Jest's tests from CommonJS synchronously.** It needs the variants before the engine has
  selected them, which is an engine started and an interaction added during collection, synchronously:
  not something a Node module can do.

## Consequences

Easier: a request the mock rejects is in the test's own output while the test runs, under the test
that made it, in both SDKs, whatever the test then does. Spock, Vitest and Jest as ES modules report
each variant as a test, as JUnit 5 does.

Harder: an SDK that asks for events must drain them, or a thousand undrained requests stall its mock.
The SDKs send `events/poll` during every closure; a variant's run costs a poll per 100 ms.

Committed to: `create`'s `events` and `stream`, the two event kinds, and their payloads, in the
protocol (engine-protocol spec §8.2, §9.6).
