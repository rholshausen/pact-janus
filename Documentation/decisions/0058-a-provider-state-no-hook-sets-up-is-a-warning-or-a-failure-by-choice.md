# 0058 — A provider state no hook sets up is a warning, or a failure when the run asks

- **Status**: accepted
- **Date**: 2026-10-30
- **Plan tasks**: none; found verifying PactFlow's `example-provider` through `pact-janus/verifier`.
  [ADR 0056](0056-provider-test-integrations-message-producers-a-junit-extension-and-a-pact-js-shaped-verifier.md)
  decision 3 left a state with no handler to "what the engine does with any state no hook handles",
  which this decides; [ADR 0046](0046-ask-a-provider-for-a-message-through-produce-message-and-let-the-transport-name-its-parts.md),
  whose `message-not-produced` failure the failing mode spells itself after
- **Evidence**: pact-js's `Verifier`, which logs a warning for a state with no handler and runs the
  interaction; pact-jvm's provider runner, which fails it; `janus verify` and both SDKs before this
  change, which ran it and said nothing

## Context

A provider test that comments out, misspells or forgets a state handler should hear about it. Janus did
not say anything. The engine selects `state-setup` hooks per state by their `when` selectors, and a
state no hook admitted simply had no setup. The exchange ran, and passed whenever the provider happened
to be in the state already. An SDK cannot tell either: its handlers are hooks, and a pact fetched from a
broker reaches only the engine.

Today's tools disagree. pact-js warns and runs the interaction; pact-jvm fails it. A provider whose
states needed no setup verifies on pact-js today, and must still verify after moving.

## Decision

1. **The engine warns of a state no `state-setup` hook admits.** Before the exchange, it emits a
   `verification/warning` with `code` `state-unhandled`, the interaction and variant, the `state`'s
   name and a `message`, once per state per variant. The exchange runs without it, and the verdict
   is the provider's, as every warning leaves it (engine-protocol spec §9.6).
2. **A run can ask to fail instead.** `verification/verify`'s `options.unhandled-states` is `warn`
   (the default) or `fail`. With `fail`, the variant fails before its exchange with
   `component-unavailable`, `details.component` `hook/state-setup`, `details.reason`
   `state-unhandled` and `details.state`, as a message no hook produces fails (ADR 0046). No warning
   repeats it. `janus verify --unhandled-states warn|fail` sets it.
3. **The SDKs collect warnings and print them.** A `verify` result carries `warnings`, every
   `verification/warning` payload in order, and each is written to standard error as it arrives,
   naming its code, its interaction and its message. A test shows what it prints, and a warning kept
   only on a result nobody reads would change nothing (behavioural spec `verify`,
   `verification.verify.warnings-collected`).

## Alternatives considered

- **Fail by default, as pact-jvm does.** A pact-js provider whose states need no setup would fail on
  moving, and `janus verify` would disagree with today's verifier on such pacts.
- **Warn in the SDK, from the handler names it knows.** The SDK sees a broker's pacts only through the
  engine, and a configuration's own hooks only by their selectors. The engine is where both are known.
- **A result member instead of an event.** The warning event already exists for what a run notices and
  does not fail on, and `janus verify` already prints it.

## Consequences

Easier: a missing or misspelled state handler is visible in every runner, and a provider that wants
pact-jvm's strictness asks for it.

Harder: a run with states and no state hooks at all, such as `janus verify` without `-s`, now prints a
warning per state per variant. That is the case the warning is for.
