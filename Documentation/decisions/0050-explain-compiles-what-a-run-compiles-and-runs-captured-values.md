# 0050 — `explain` compiles what a run compiles, and runs captured values

- **Status**: accepted
- **Date**: 2026-10-25
- **Plan tasks**: S1.12
- **Evidence**: Phase 9 findings 23 and 31; `cli/tests/janus-engine-node/test/explain.test.ts`, the
  protocol-level client; `cli/tests/cli.rs`'s `explain_executed_prints_a_components_fragment_and_runs_its_action`

## Context

`verification/explain` is where the RFC's inspectability claim lives: it prints the plan the engine
will run. Two findings showed it did not.

- **Finding 23.** `explain` took no `components`, so for an interaction a component contributes a
  plan fragment for, it printed the generic plan: `match:integer` where `verify` runs `csv:integer`.
  It also rebuilt a contract's specification without its `content-types`, so it could not have found
  the fragment anyway.
- **Finding 31.** `janus explain --executed` compiled through `verification/explain`, then ran the plan
  with the kernel's Rust API. That was the one place the CLI stepped outside the protocol, so matching
  captured values offline was a CLI feature. No SDK, WASM host or subprocess host could do it.

The findings listed options: `components` on the request, `--config` on the CLI, marking slots compiled
generically; and `values` on `explain`, or a separate `plan/execute` operation.

## Decision

1. **`verification/explain` takes `components`**, resolved exactly as `verify`'s `target.components`
   are, by the same loaders. The subject's requirements, declared content types and contributed
   fragments are checked against them as a run checks them, before it starts. An interaction a run
   would refuse is refused here too, as `component-unavailable`, rather than explained as a plan no
   run executes. A contract's `content-types` reach the compiler.
2. **`verification/explain` takes `values`**: captured values by path, with `encoded` tagging octets
   as `base64`, as a golden corpus case gives them. It runs the plan against them with the scope's
   content detection (ADR 0049) and the actions its components contribute, and returns `executed`:
   `status`, `mismatches` as a run reports them, and the executed text form. One call compiles and
   runs, so what runs is exactly what was printed.
3. **`janus explain --config <file>`** reads the `components` of a verifier configuration, as
   `verify --config` does, and `--executed` sends its values in the same call. The CLI no longer runs
   a plan itself.

## Alternatives considered

- **A separate `plan/execute` operation, taking a plan document.** It would execute any plan,
  including one a host had edited. But a plan document read back is untrusted (AGENTS.md's
  architecture rules), and the host would have to send back the components that compiled it. One call
  keeps the plan and the components that compiled it together.
- **Mark slots compiled generically, and say a fragment might apply** (finding 23's option C). That
  is honest, but `explain` would still not show the plan a run executes.
- **Keep explaining a refused interaction as its generic plan.** A user explaining a CSV contract with
  no component declared would see a plan, and then `verify` would refuse it. Saying which component is
  missing is the plan's answer in that case.

## Consequences

- Findings 23 and 31 are closed. The protocol-level client explains a CSV contract with the component
  declared, and runs captured values through `csv:integer`.
- `explain` of an interaction whose declared type or requirement no loaded component satisfies now
  fails with `component-unavailable`, where it printed a generic plan. That is a change to what
  `explain` answers, and the change file says so.
- Every host can match captured values offline through the protocol. The WASM embedding no longer needs
  a benchmark-only export to time the interpreter, though `benchmarks/janus` still uses it.
- **Tripwire**: if a host needs to execute a plan it did not get from `explain` (a plan from a run's
  `verification/executed-plan` event, say), `plan/execute` is the operation, taking the plan with the
  components that compiled it.
