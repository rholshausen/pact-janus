# 0102 — A check loads the components a project declares, and compares their operators as their handshakes declare

- **Status**: accepted
- **Date**: 2026-11-20
- **Plan tasks**: S5.7
- **Evidence**: `cli/tests/cli.rs` (`check_with_config_decides_a_components_operator_that_is_otherwise_reviewed`),
  `third-party/janus-csv` (`csv:column`), `engine/kernel/src/component/loader.rs`'s handshake tests;
  [ADR 0050](0050-explain-compiles-what-a-run-compiles-and-runs-captured-values.md),
  [ADR 0087](0087-compare-the-built-in-components-operators-by-their-own-matchers-and-a-compare-each-declares.md),
  [ADR 0093](0093-publish-and-fetch-provider-shapes-and-reports-through-the-engines-broker-client-found-by-the-links-a-broker-advertises.md),
  [ADR 0096](0096-narrow-the-entries-a-check-tries-by-what-each-selector-fixes-and-a-prefix-an-operator-declares.md)

## Context

ADR 0087 taught the subsumption walk to decide a component operator: a finite provider by the operator's
own action, anything wider by the component's `matcher/compare`, for an operator its component declares
`exact` or `conservative` (component-interfaces spec §7.5). Only the built-in HTTP component's operators
benefited. `subsumption/check` loaded no declared component, `janus check` had no way to name one, and
the WASM binding answered neither an operator's class nor `compare`, so a third party's operator was
opaque to the walk whatever its handshake said: every comparison of one, however decidable, was a review.
`janus explain` had the same gap until ADR 0050 gave it `--config`.

## Decision

1. **`subsumption/check` and `broker/check-contracts` take `components`**, the project's declared
   components, resolved by the same loaders as `verification/verify`'s `target.components`. A component
   that cannot be loaded fails the call as `component-unavailable`, before anything is walked. The walk
   runs selectors and compares operators with the scope's contributions: the declared components, then the
   embedding's own matchers. A contract's `requires` is not checked against them: a check reads documents
   and runs no interaction, and an operator nothing loaded is still answered, opaquely, as a review.

2. **A declared component's comparability is read from its handshake**, by the engine, whatever binding
   loaded it: each `contributes.operators` entry's `comparability`. An in-tree matcher still answers in
   code. A class the engine does not know is opaque, as an absent one is.

3. **An operator declared `exact` or `conservative` must be decidable, and is checked when the component
   loads.** The engine decides a finite provider against it by running the operator's own action, so the
   component must contribute an action of the operator's name (spec §7.1) and implement `matcher`. A
   handshake that declares one comparable without both is refused as `component-invalid`, naming why,
   rather than found wanting in the middle of a check. `exact` still obliges the component to implement
   `compare`, which cannot be checked before it is asked.

4. **The WASM binding forwards `matcher/compare` and `matcher/prefix`** over the component pipe, as it
   forwards `apply`. A malformed `compare` answer is a synthesised error, which the walk treats as no
   answer: `unknown`.

5. **`janus check --config <file>`** reads a verifier configuration's `components`, as `janus verify
   --config` and `janus explain --config` do, and sends them with every `subsumption/check`, or with
   `broker/check-contracts` under `--broker-url`.

6. **The worked third-party component has a comparable operator.** `third-party/janus-csv` contributes
   `csv:column`, a field that spells a value of its `type` (`text`, `integer`, `number`, `boolean`),
   declared `exact`. Its `compare` decides two columns by their types (`integer` within `number`, every
   type within `text`), a column against `string` or `any` as `yes`, `string` against a narrower column as
   `no`, and anything else as `unknown`: `integer` means a number in a JSON body and text that spells one in
   a CSV body, and `compare` is not told the slot's content type.

## Alternatives considered

- **Load every component a contract `requires` from a registry, unasked.** Rejected: components are
  declared, never discovered (component-interfaces spec §10.2), and a check that fetched code because a
  document named it would run what nobody chose.
- **Read the comparability in each binding.** Rejected: the handshake is the source of truth about what a
  component contributes (spec §3.2), and reading it once in the engine means a subprocess binding, when
  S5.5 builds one, gets it for nothing.
- **Pass the slot's content type to `compare`.** Not now: no operator needs it to answer what it answers,
  and `unknown` is the honest answer where it would. A component operator whose meaning depends on the
  type it is decoded under is the case that would add it, as a member `compare` may read.
- **Refuse a contract whose `requires` the check's components do not satisfy.** Rejected: a provider's
  pipeline checking many consumers' contracts may not have every consumer's components, and an opaque
  operator is already reported as a review, which a policy can block on.

## Consequences

- A third party's comparable operator is decided by `janus check`, locally and against a broker, as the
  built-in HTTP component's are, once the project declares it.
- A component that declared an operator comparable and contributes no action of its name no longer
  loads. None did: `csv:column` is the first declared operator outside the tree.
- `explain --config` and `check --config` read the configuration through one function, so they cannot
  disagree about what a project declared.
