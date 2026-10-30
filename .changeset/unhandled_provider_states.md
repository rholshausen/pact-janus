---
default: minor
---

# A provider state no hook sets up is a warning, or a failure when the run asks

Before, a provider state that no `state-setup` hook admitted (none configured, or every one's `when`
selecting other states) ran its exchange without setup and said nothing, so a missing or misspelled
state handler went unnoticed (ADR 0058).

Now the run emits a `verification/warning` with `code: state-unhandled`, the interaction, the variant
and the `state`, before the result. The exchange still runs without setup and the verdict is
unchanged, as pact-js treats it. `janus verify` prints the warning.

`verification/verify` takes `options.unhandled-states: "fail"` to fail such a variant instead, before
its exchange, with `component-unavailable`, `details.component: hook/state-setup`,
`details.reason: state-unhandled` and `details.state`, as pact-jvm treats it.
`janus verify --unhandled-states fail` sets it.

For an SDK maintainer: the behavioural specification's `verify` result gains `warnings`, every
`verification/warning` payload in order, which the run's report says too (ADR 0059)
(`verification.verify.warnings-collected`, conformance case `provider/warnings-collected`). The
bindings' `VerifyOptions` gain `unhandled-states`.
