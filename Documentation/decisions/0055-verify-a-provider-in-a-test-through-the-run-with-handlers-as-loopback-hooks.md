# 0055 — Verify a provider inside a test through the protocol's run, with handlers as loopback hooks and the configuration loaded by the engine's host

- **Status**: accepted
- **Date**: 2026-10-30
- **Plan tasks**: S1.14, which this decides; S1.15, which builds the test-framework integrations on it;
  [ADR 0014](0014-hooks-are-resolved-configuration-not-callbacks.md), whose loopback hook this
  realises and whose loader this places; [ADR 0032](0032-speak-to-the-broker-through-protocol-operations-backed-by-a-native-host-capability.md),
  whose native host capability this one follows; [ADR 0041](0041-refuse-a-filter-that-names-no-recorded-variant-and-report-a-narrowed-run-as-filtered.md),
  whose filtered run the SDKs report
- **Evidence**: pact-js's `Verifier` options (`stateHandlers`, `requestFilter`,
  `consumerVersionSelectors`, `publishVerificationResult`); pact-jvm's provider runner (`@State`,
  `@TargetRequestFilter`); lifecycle-hooks spec §3, §6.2, §7.1 and §8.4; the thinness audit's
  budgets (`sdks/*/thinness.json`); `janus verify`'s own use of the loader (`cli/src/verify.rs`)

## Context

Most providers verify inside a test, with state handlers that close over the test's fixtures and a
request filter that adds a token. Janus has every piece that needs except the SDK side. The run is
one operation, `verification/verify`, reporting on a stream. A hook can be an HTTP endpoint the test
process serves (ADR 0014 commitment 1). And `target.hooks` takes the resolved configuration.

Three questions are left open by what exists:

- **What the SDK decides.** A verdict is the engine's. But today's SDKs also retry, re-order, and
  turn a run into a pass or failure in their own ways. The thinness rule says none of that belongs
  in an SDK. Something still has to say which outcome fails the test.
- **How a handler is reached.** "A loopback `http` hook" fixes the mechanism, not the details. A
  port that any local process can call drives the test's fixtures. A name the engine reports must be
  stable across languages, or a hook report means something different in each.
- **Who loads `verifier.janus.yaml`.** ADR 0014 makes the loader "CLI or SDK embedding layer". The
  loader interpolates `${VAR}`, inlines scripts and resolves paths. Written once per language, it is
  three dialects of one file, and a quarter of an SDK's hand-written budget. Not written at all, a
  project that already has a configuration for `janus verify` cannot reuse it in its tests.

## Decision

1. **Verifying in a test is one run of the protocol.** The SDK's `verifier` sends one
   `verification/verify` and polls `events/poll` until the stream's terminal event. Nothing else
   crosses the pipe for the run, and the SDK repeats nothing:
   - The pacts the test names by path are read by the SDK and sent `inline`, because reading files
     is host business (engine-protocol spec §8.3). A broker's selection and pacts by URL are the
     `broker` and `urls` sources, and publishing is `options.publish`. Each is passed as the
     protocol spells it. The SDK reads no environment variable for any of them, as the engine reads
     none for a broker's credentials: the test passes what it wants sent.
   - The provider is `target.transports`, one `http` binding whose `base-url` is the provider URL.
   - The run's outcome is the summary's `status`. `failed` fails the test, carrying the run's
     per-variant results. `verified` passes. `filtered` does not fail the test, as `janus verify`
     exits 0 for it, and the result the test gets says `filtered`, never `verified`. A pending
     pact's failures are in the results, and fail nothing, as the engine decided.
   - A refusal before the run — `variant-not-found`, `filter-matches-nothing`, a component or hook
     that is unavailable — is the engine's error, raised as the SDK raises any engine error.
2. **A handler is a loopback `http` hook, served for one run.**
   - The SDK serves one endpoint on the loopback interface, at a port the system chooses. It starts
     before `verify` is sent and stops when the run ends, whether the run passed, failed or never
     started.
   - Each handler has its own path. Every entry the SDK writes carries an `authorization` header with
     a bearer token made for the run. A request without it is answered 401 and runs nothing, so no
     other process on the machine can drive the test's fixtures. The engine never repeats a hook's
     headers (lifecycle-hooks spec §7.3).
   - A **state handler** for one state name is a `state-setup` entry with `when.state` that name,
     and, when it has a teardown, a `state-teardown` entry with the same `when` and the same name.
     Its setup receives the state's parameters. What it returns, an object, is the result's
     `values`, which a v1–v4 pact's `ProviderState` generators read (ADR 0034). Nothing returned is
     `ok`. A thrown failure is `failed`, with the code `handler-failed` and the failure's message. A
     handler says it cannot reach the state in the language's own way; the SDK answers
     `unsupported`, with the code `state-unsupported` and its reason.
   - A **request filter** is a `before-request` entry. It declares the request slots it changes, by
     default `headers`, and the entry's `changes` is exactly those, as `parts.request.<slot>`. It
     receives each slot's content and returns the request. The SDK answers `changes` for every slot
     it declared, compares nothing, and so never decides what changed.
   - Entries are named by kind and position: `state-1`, `state-2`, … in the order the state handlers
     were declared, and `request-filter-1`, …. The engine reports a hook by its name, so a report
     names the same hook in every language.
   - The SDK's entries follow the configuration's own at each point, in declaration order
     (lifecycle-hooks spec §4.2). A configuration's token hook at `before-verification` runs before
     a filter that reads it.
3. **The engine's host loads a configuration: `config/load`.** `{ path }` → `{ config }`, behind
   the `config-loading` capability:
   - `janus` and `janus-engine` register it, as they register the broker client. It runs the loader
     `janus verify --config` runs (`engine/hooks-host`), so a file means the same through the CLI
     and through every SDK.
   - `path` is absolute: the SDK resolves a relative one against its own working directory, as it
     already does for a component's `file` source. A relative path is `malformed-frame`.
   - The variables come from the engine process's environment. That is the test process's, since
     the SDK starts the engine with its own environment.
   - The answer is the resolved document, with `version`, `hooks` and `components`. The SDK puts its
     `components` in `target.components` and its `hooks` in `target.hooks`, with its own entries
     after them.
   - A file that cannot be read, an unset variable or an unreadable script is `hook-unresolved`. A
     document that does not validate is `hook-config-invalid`. Both are the codes the loader already
     raises (lifecycle-hooks spec §11).
   - An engine without the capability, the offline WASM artifact, answers `component-unavailable`
     naming `config-loader`. A host there passes resolved values, as it would anyway.
   - This does not put the environment in a run. The kernel still reads no file and no variable,
     and `verify` still receives a closed document (ADR 0014 commitment 2, ADR 0052). `config/load`
     is a separate operation a host chooses to make before it sends one, exactly as `janus verify`
     loads its file before it sends anything. The answer carries interpolated secrets back to the
     process whose environment they came from. The trace log hides them as it hides any hook entry's.
4. **The provider side is measured as the consumer side is.** The provider primitives are in the
   `idiomatic` layer of each SDK's `thinness.json`, and the loopback endpoint is a layer of its own,
   `loopback`, with its own budget, because serving HTTP is the one new kind of code it adds. A
   budget that grows says so in the commit that grows it.

## Alternatives considered

- **Each SDK writes its own loader.** It is what ADR 0014 allowed. It is three implementations of
  interpolation, script inlining and path resolution. TypeScript transpiling alone is a toolchain on
  the JVM, so the JVM's would differ from the start. Every one of them counts against the thinness
  budget, and the next language writes a fourth.
- **Programmatic configuration only.** The simplest SDK, and a project with a `verifier.janus.yaml`
  for `janus verify` would write its hooks twice, once in the file and once in the test. The two
  would drift, which is the problem one configuration document was meant to remove.
- **`verify` takes a configuration path, and the host resolves it in the run.** It saves one round
  trip. It also makes a run's behaviour depend on the environment it ran in, which ADR 0014 refused
  and ADR 0052 refused again for credentials. The SDK could no longer add its own hooks after the
  file's, so the order would be a rule of the engine's instead of what the request says.
- **A reverse call from the engine to the host for handlers.** ADR 0014 refused it, and nothing here
  changes the reasons: a reverse channel on every pipe, and none possible in the WASM embedding.
- **An unauthenticated loopback endpoint.** One less header. Any local process, a browser page among
  them through DNS rebinding, could then call a state handler that writes to the test's database.
- **Hook names from the state's text.** More readable in a report. A name must match
  `[a-z][a-z0-9-]*` and be unique within its point. A rule that turns any state's text into one is
  one more thing every language must agree on, and two states can collide. The context of every
  invocation already carries the state's own name.
- **A request filter answers with the slots it changed, found by comparing.** It would let a filter
  change anything it likes. Comparing is the SDK deciding what changed, and an entry must declare its
  `changes` before the run anyway (lifecycle-hooks spec §4.3), so the declaration has to be written
  either way.
- **A filtered run fails the test.** It never reads as a pass then. It would also fail every run a
  developer narrowed on purpose while working on one interaction. `janus verify` exits 0 for it and
  says so, and the SDK's result says `filtered`, never `verified`, which is ADR 0041's rule.

## Consequences

Easier: a provider verifies in its own test with handlers that close over its fixtures, and the
engine sees an ordinary `http` hook. One configuration file means the same in the CLI and in every
SDK. A hook report names the same hooks in TypeScript and Java. The provider side adds no
operation but `config/load`, which any host may use.

Harder: a handler is reached over a socket, which costs a millisecond or two per call, where today's
in-process callbacks cost nothing. A state handler cannot share a transaction with the request it
precedes, as it could not before. An SDK on an engine with no network grant cannot serve handlers,
and fails before the run as `hook-unavailable` naming `http`. Message producers, pact-js's
`messageProviders`, are not decided here: they are `produce-message` hooks, whose result is a whole
`parts` document of octets, and they wait for S1.15, which has a message provider example to verify.
