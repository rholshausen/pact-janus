# 0057 — The HTTP component runs `http:status-class`, as a matcher compiled into the embedding

- **Status**: accepted
- **Date**: 2026-10-30
- **Plan tasks**: S1.35, which this decides; [ADR 0022](0022-a-fragment-replaces-its-slots-plan-and-declares-a-grammar-the-engine-says-it-reads.md),
  whose `matcher/apply` runs a component's own actions; [ADR 0012](0012-one-interface-two-bindings.md),
  whose rule that a built-in goes through the same interface this keeps;
  [ADR 0056](0056-provider-test-integrations-message-producers-a-junit-extension-and-a-pact-js-shaped-verifier.md),
  whose examples found the gap
- **Evidence**: pact-js's `matchers` example, whose third interaction checks its status by class;
  `pact_matching`'s `match_status_code` and `pact_models`' `HttpStatus`, the classes today's verifier
  checks; shape spec §3.5 and component-interfaces spec §2.4 and its worked HTTP example, which already
  name `http:status-class` as the HTTP component's operator

## Context

A v4 pact can check a response's status by its class: `{ "match": "statusCode", "status": "success" }`
admits any 2xx, and `"status": [200, 204]` admits those codes. The legacy compiler refused the rule, so
`janus verify` failed an interaction today's verifier passes. `janus upgrade` already wrote it as the
component operator `http:status-class`, as contract-file spec §8.2 says, but no component contributed
that operator. A contract that used it failed too.

The specifications already decide who owns it. The kernel knows nothing of HTTP statuses (shape spec
§3.5), and the component that speaks HTTP is the one that says what a status class is. The worked
example of the HTTP component's handshake lists `http:status-class` among its contributed actions, and
`matcher` among its interfaces.

What was missing is a way for an embedding's own component to run an action. A plan reaches a
component's action through `matcher/apply`, and only components a project declares were asked.

## Decision

1. **The built-in HTTP component implements the matcher interface's `apply` for `http:status-class`.**
   It lives in `engine/component-http`, beside the transport. Its members are `class`, one of
   `information`, `success`, `redirect`, `client-error`, `server-error`, `non-error` and `error`, or
   `codes`, a list of statuses. Each class is the range today's verifier checks. A status outside it
   fails the node: `Expected a success status (200–299) but was 404`. An unknown class is
   `invalid-config`.
2. **An embedding registers a matcher compiled into it, and every scope runs its actions.**
   `Engine::register_matcher` names the component and the actions it runs, and declares it in-tree under
   the `matcher` interface. A requirement naming it is satisfied, and a declaration cannot take its
   name. Its actions run through the same `matcher/apply` binding a declared component's do, with the
   same arguments. Only declared components are still asked for plan fragments.
3. **The legacy compiler compiles a pact's `statusCode` rule to `http:status-class`,** with the same
   members `janus upgrade` writes. A v1–v4 pact is HTTP, so the legacy layer may name the HTTP
   component's operator. The kernel outside it names no status.
4. **`janus upgrade` writes a list of codes as `codes`.** It wrote them as one comma-separated
   `class` before, which nothing read.

The native embeddings (`janus`, `janus-engine`) register the matcher. The WASM engine has no HTTP
component, so a plan that reaches `http:status-class` there fails the node by name, as it would for
any action no component in scope contributes. It cannot run an HTTP interaction anyway.

## Alternatives considered

- **A core action, `match:status-class`.** Rejected by shape spec §3.5 already: a status class is HTTP
  knowledge, and the kernel holds none outside its legacy layer.
- **A `legacy:status-class` action for pacts only.** It would make a pact and its upgraded contract run
  different code for the same rule, and leave the contract's operator unrun.
- **Ask an in-tree component for fragments too.** No built-in contributes one, and asking would change
  nothing. It is left until one does.

## Consequences

Easier: pact-js's `matchers` example verifies as today's verifier verifies it, and an upgraded contract
with a status class verifies. The corpus has a fixture for every class and a code list
(`corpora/verifier/matching-rules/status-code`), whose mutations step each status past its class's edge,
and plan cases compiled both ways (`corpora/legacy/v4-status-class`).

An in-tree component can now contribute an action. The next one, `header:parse` (plan grammar §4.6),
takes the same path.

Harder: nothing asks the HTTP component about its operator's comparability or variant dimension yet,
so `janus check` treats `http:status-class` as opaque, as it treats every component operator.
