# 0034 — Apply a pact's request generators through a generator component, fed by provider-state values

- **Status**: accepted
- **Date**: 2026-10-20
- **Plan tasks**: S1.4 (the build); S1.19 (the same for contracts, which this leaves open)
- **Evidence**: pact-reference's verifier (`pact_verifier::verify_interaction`, which merges every
  state's `params` with what each state-change call returned and calls
  `pact_matching::generate_request` in provider mode; `make_state_change_request`, which reads a
  JSON object reply as those values); its test for issue #441
  (`support_passing_provider_state_params_to_provider_state_generator`); shape-language spec §3.4;
  component-interfaces spec §7.3; lifecycle-hooks spec §3.2, §8.4

## Context

A v3 or v4 pact's request can carry generators: a path built from a provider-state value
(`/orders/${id}`), a header from a random integer, a body date that has to be today's. Today's
verifiers apply them before sending, so the provider sees a request the pact *describes* rather
than the example it recorded. Janus replays the example verbatim on the v1–v4 path, so a pact whose
request depends on a generator verifies differently: a `ProviderState` path asks for the order the
consumer's test happened to use, not the one the state handler just created.

Two things are missing. Nothing in the engine produces a value: the shape language reserves a
`generator` member naming a component, and the component interface specifies `generator/generate`,
but no component implements it and nothing calls it. And the values a `ProviderState` generator
reads never arrive: a state-change endpoint returns them as a JSON object (`{"id": 1234}`), the
`http` hook reads that reply as a hook result, and since `id` is no member of a result it is
dropped.

## Decision

1. **The v1–v4 generator vocabulary is a built-in generator component**, `pact`, answering
   `generator/generate` for `pact:RandomInt`, `pact:RandomDecimal`, `pact:RandomHexadecimal`,
   `pact:RandomString`, `pact:Regex`, `pact:Uuid`, `pact:Date`, `pact:Time`, `pact:DateTime`,
   `pact:RandomBoolean` and `pact:ProviderState`, each configured by the members a pact gives it.
   It produces values with `pact_models`, the same code today's Rust verifier uses, so a value is
   generated the way it is generated today. It is built into the kernel behind the generator
   interface, as the scripted-hook runtime is behind the hook interface, so every embedding has it,
   the WASM one included, with nothing to register: the kernel already depends on `pact_models` for
   reading pacts, and the component adds no dependency.
2. **The legacy compiler applies a pact's request generators, in provider mode, before
   `before-request`.** It finds each target (the path, a header, a query parameter, every body value
   a generator's path resolves to) and asks the component for a value, passing the recorded value as
   `context.example` and the provider-state values as `context.provider-state`. A generator only the
   consumer side runs (`MockServerURL`) is skipped, as it is today. Response generators are not
   applied: today's verifiers compare against the recorded response, and so does this. So a hook at
   `before-request` sees, and may sign, the request that is actually sent.
3. **A `state-setup` hook result may carry `values`**: an object of provider-state values. The
   generator context for an exchange is every state's resolved `params`, in the interaction's state
   order, then every state-setup hook's `values` in the order they ran, a later member replacing an
   earlier one: today's precedence. `values` is not reported by default, as `data` is not (a
   state handler may return a credential).
4. **The `pact-state-change` format reads a JSON object reply as values.** A 2xx reply whose JSON
   object has no `outcome` member is `{ "outcome": "ok", "values": <the object> }`, so a state-change
   endpoint written for today's verifiers works unchanged. One with `outcome` is a hook result, as
   before, so an endpoint can still answer `unsupported`. The `janus` format is unchanged: a result
   document, which may now include `values`.
5. **A generator that cannot produce a value leaves the recorded value in place, with a warning.**
   A `ProviderState` expression naming a value no state provided, a regex that does not parse, a
   generator type no component knows: today's verifier keeps the recorded value silently. Janus keeps
   it too, so the verdict is today's, and says so in a `generator-failed` warning naming the
   interaction, the target and the reason.
6. **Contracts are a separate item (S1.19).** `janus upgrade` still drops generators with
   `generator-dropped`, and a contract's request is replayed by example. Mapping a v1–v4 generator
   onto the shape's `generator` member, which this component makes possible without new vocabulary,
   and applying shape generators when a contract is verified and when a mock serves, need
   variant-semantics and contract-file changes of their own.

## Alternatives considered

- **Call `pact_models` from the legacy module, with no component.** It is a little less code. But
  the plan says the vocabulary should become a component, and the contract path will need the same
  generators named in a shape: a component built now is what that path names later, and what an
  out-of-tree generator can stand beside.
- **A separate crate, registered by each embedding**, as the HTTP transport is. That is right for a
  component that needs the host (a socket, a process); this one needs nothing the kernel lacks, and
  every embedding that forgot to register it would verify generators differently.
- **Pass state values through `data`.** It needs no new member. But `data` is defined as opaque to
  the engine and keyed by the hook that wrote it, and generators would make the engine read it,
  with every hook's scratch in the generator context.
- **Read every pact-state-change reply as values, ignoring `outcome`.** Exactly today's reading, but
  it would take away the `unsupported` answer an endpoint can give now, which is how a provider
  reports a state it cannot reach.
- **Fail the variant when a generator fails.** Stricter than today. A provider whose pact has a
  generator nobody can satisfy would start failing on the day it moved to Janus, which is what S1.1
  exists to prevent. The warning makes it visible without changing the verdict.

## Consequences

- A pact with request generators sends the request today's verifiers send, and a state-change
  endpoint's returned values reach it unchanged. S1.1's harness measures both.
- The engine has a caller of `generator/generate`, and the in-tree component is the first
  implementation of that operation. An out-of-tree generator component is not loaded by anything yet;
  that is S1.19's, when a shape can name one.
- A run is no longer deterministic for a pact with random generators, as it is not today. The
  executed request is in the `debug` log of the HTTP transport.
- The lifecycle-hooks spec gains `values` (§3.2, §4.4, §8.4) and the hook result schema the member;
  the component-interfaces spec names the `example` and `provider-state` context members (§7.3).
  Both are additive within v1.
- **Tripwire**: a v1–v4 generator whose output depends on the content type of the body (XML, form)
  arrives with S1.6's content components; this ADR's targets are JSON bodies, and a generator on a
  body of any other type gets the `generator-failed` warning until then.
