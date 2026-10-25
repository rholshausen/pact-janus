# 0048 — Carry generators on shapes, and fill fresh values where a value is produced

- **Status**: accepted
- **Date**: 2026-10-25
- **Plan tasks**: S1.19
- **Evidence**: `engine/kernel/tests/generators.rs`, which verifies the contract each generator pact
  upgrades into and checks the provider is sent what the pact's verification sends;
  `engine/kernel/tests/consumer_flow.rs`'s `a_generated_response_value_is_fresh_each_exchange_and_the_example_is_recorded`;
  S1.1's harness, which now verifies every fixture's upgraded contract too
  (`corpora/verifier/upgrade-disagreements.json`); ADR 0034, which built the `pact` generator component

## Context

Verifying a v1–v4 pact applies its request generators (ADR 0034). Upgrading the pact dropped them with
`generator-dropped`, so the contract replayed a recorded value where the pact sent a generated one: a
`ProviderState` path, a fresh id. The shape language reserves a `generator` member naming a component
(shape-language spec §3.4) and leaves its form to design 2.6. Variant-semantics spec §5.2 says a verifier
replays by example. Two questions the plan leaves open:

- **The member's form.** `generator/generate` takes `{ generator, config, context }`. The contract-file
  spec's worked upgrade sketches a generator as a pact writes one, `{ "type": "RandomInt", "min": 1 }`.
- **The evidence.** A mock that serves a fresh value could record what it served, or what the shape's
  example says. A contract's identity is content-addressed (contract-file spec §2.3), and selection is
  deterministic (ADR 0008).

## Decision

1. **A node's `generator` is `{ "name": "<component>:<generator>", "config": { … } }`.** `name` is what
   `generator/generate` names, and `config` is passed as it stands. It belongs to the component, and
   the kernel does not read it. `janus upgrade` writes a v1–v4 generator as `pact:<type>`, with every
   member the pact gives it except `type` as its configuration.
2. **The recorded value stays the variant's evidence.** The generator marks a slot as one to fill
   fresh where the slot's value is produced. A mock records the example, not the value it served, so
   recording the same interaction twice writes the same contract.
3. **A verifier fills a contract's request.** Once the states have answered and before `before-request`,
   as a pact's request is generated (ADR 0034 decision 2), it fills every position the request's shapes
   give a generator, on the provider's side, from the same provider-state values. A slot carried as
   octets is read as the document its shape is about, by its declared type or the type it is labelled
   with, and written back by the same component. A generator that produces nothing, or a slot that
   cannot be read or written, keeps the recorded value, with ADR 0034's `generator-failed` warning.
   This is variant-semantics spec §5.2's one exception to replay by example.
4. **A mock fills its reply, every time it serves it,** on the consumer's side, with the variant's
   resolved state parameters as provider-state values. This answers S2.3's fresh-per-exchange question:
   a generated value is fresh per exchange. A shape's matching is unchanged.
5. **A generator runs on the side it is for.** `generator/generate`'s context gains `side`, `provider`
   or `consumer`. The `pact` component keeps the example for a generator that does not run on that
   side, as today: `ProviderState` runs only on the provider's.
6. **The walk is the shape's.** An `object`'s members, an `array`'s entries by position, every element
   an `each-like` holds, every value an `each-entry` holds, and the value an `optional` or `nullable`
   wraps. A `contains` entry's element is not known until it is matched, so nothing below one is filled.
   A node with a generator is replaced, and nothing below it is walked.
7. **The upgrade places each generator where today's verifier generates it.** A path, status or
   query parameter's every value, a header's first value, and a body position by its path: through
   JSON members and entries (`*` and `[*]` reaching every one, or an `each-like`'s template), a form
   name's first value or its `[i]`th, and an XML attribute or text, matching names by local name since
   ADR 0039's document keeps no prefixes. A request body a generator fills keeps the pact's type on its
   evidence, so a verifier can read it. A generator is still `generator-dropped` when no component
   provides it (`MockServerURL`, a plugin's), when today's verifier does not apply it (a text, binary or
   multipart body), or when its path names no node: no value the example has, or one element of a
   list the contract shapes as a template.
8. **A form value may be written from a number or a boolean**, as its text, as the XML component writes
   one (ADR 0039 decision 2). A generator asked for a form value's text may answer with a number.
9. **S1.1's harness verifies each fixture's upgraded contract.** For every case whose pacts upgrade
   into contracts with all their interactions, `janus verify` runs on the contracts too, against a
   provider of their own, and each verdict is compared with `janus verify`'s on the pact. A difference
   is in `corpora/verifier/upgrade-disagreements.json`, with what accounts for it, or the run fails.

10. **A JSON body declares its type too**, amending ADR 0047 decision 1. The harness found that a
    reply with no `Content-Type` is decoded as JSON by the pact's verification (ADR 0036 decision 4)
    and as bytes by a contract whose JSON body is undeclared. JSON needs no `requires` entry, since every
    engine is built with it. A converted body's evidence is the pact's octets, JSON's too (ADR 0047
    decision 7).
11. **What else the harness found is converted as the legacy compiler compiles it, or reported.**
    - `eachKey` becomes `each-entry` whose keys follow the rule's own matchers, with its values left
      open: v1–v4 compare a member the pact names only when the reply has it, which `each-entry`
      cannot say (`rule-unmapped`).
    - `eachValue` on a list becomes `each-like`, `min: 0`, over the rule's own matchers.
    - A v4 interaction marked `pending`, whose failure v1–v4 report without failing the run, has no
      contract form, which `pending-dropped` reports (S1.28).

## Alternatives considered

- **The pact's own form, `{ "type": "RandomInt", "min": 1 }`.** It is what a pact author knows. But it
  names no component, so another generator vocabulary could not be named beside it, and `type` would
  mean a different thing in a shape node.
- **Record what the mock served.** The evidence would be what the consumer actually handled. But every
  recording would write a different contract, and a provider's `janus check` would see a new contract
  every run with nothing changed.
- **Generate in the kernel's own walk for v1–v4 bodies, with `pact_models`' handlers.** That is exactly
  today's generation. But it would put v1–v4 knowledge on the contract path, outside the legacy module
  (S1.11), and a contract written by a consumer suite has no pact behind it.
- **Check that a generated value is admitted by its shape** (shape-language spec §3.4's SHOULD). It is
  not done yet. A v1–v4 generator on a position with no rule is frozen to `equality` and admits only
  the example, so a check would refuse a pact's own generators. It needs the S3.4-style loosening first.

## Consequences

- An upgraded pact with request generators sends the provider what the pact's verification sends, and
  no `generator-dropped` remains for a generator the `pact` component provides at a position today's
  verifier generates.
- A consumer's DSL still cannot attach a generator. A contract a consumer suite writes carries none
  unless an interaction specification names one. The DSL primitive, through the behavioural
  specification and both SDKs, is staged plan S3.6.
- A mock serving a converted contract with a `ProviderState` response generator keeps the example, as
  today's mock server does.
- Generated output is not checked against its shape, which shape-language spec §3.4 says an engine
  SHOULD do.
- **Tripwire**: if a provider relies on a generator the upgrade drops, typically `MockServerURL` in a
  response a mock serves, the `pact` component provides it on the consumer's side, with the mock's URL
  in the context.
