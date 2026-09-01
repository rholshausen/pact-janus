# Lifecycle hooks specification (v1, draft)

Plan task: **2.7**. Status: **draft — under review**.

This document specifies the **hook system**: the named points at which the engine calls out to user
code, what it hands each one and what each may change, how hooks are configured and how that
configuration is resolved, the four ways a hook can be implemented, what a failure at each point means,
and how secrets reach a hook without being written down.

Design [2.6](../component-interfaces/spec.md) owns the hook *interface* — how a hook **component** is
invoked and what its result document looks like. This document owns everything around it: a component is
one of four implementations, and the other three (a script, a command, an HTTP endpoint) are not
components at all. The two designs meet at exactly two documents, the context and the result, which is
why this specification defines the first and deliberately does not restate the second.

Its architecture is fixed by two decisions.
[ADR 0014](../../decisions/0014-hooks-are-resolved-configuration-not-callbacks.md) makes hooks
**configuration that a loader resolves**: the project writes a file, the CLI or SDK interpolates it,
reads what it references and hands the engine a closed document — so the engine holds no callbacks,
reads no files, resolves no paths and never sees a template.
[ADR 0015](../../decisions/0015-quickjs-as-the-scripted-hook-runtime.md) makes **QuickJS the scripted-hook
runtime**, compiled with the engine in all three embeddings, with the context API as the whole of a
script's capability surface.

The JSON Schemas under [`schemas/v1/`](schemas/v1/) are the **specified surface**; this prose defines
their semantics. They follow the Engine Protocol's open-world authoring rules
([protocol spec §2.2](../engine-protocol/spec.md)) and are enforced by the same CI checker. The
scripted-hook API additionally ships as [`hook-api.d.ts`](hook-api.d.ts), which is the same two documents
seen from inside a script.

Evidence: spike [1.6](../../../spikes/1.6-script-hook-bakeoff/FINDINGS.md) (the engine bake-off, and the
`wasm32-wasip2` verification that closed its open item), [1.5](../../../spikes/1.5-message-transport-shape/FINDINGS.md)
(message interactions are a *direction*, not a kind — the finding the two message points are built on),
[1.4](../../../spikes/1.4-engine-hosting-plugins/FINDINGS.md) (deadlines and sandboxing around a call the
engine does not control), [1.3](../../../spikes/1.3-subprocess-embedding/FINDINGS.md) (the stdio framing
`exec` reuses).

## Contents

1. [Scope and conformance](#1-scope-and-conformance)
2. [The model](#2-the-model)
3. [The points](#3-the-points)
4. [Ordering, chaining and changes](#4-ordering-chaining-and-changes)
5. [Failure semantics](#5-failure-semantics)
6. [Configuration](#6-configuration)
7. [Resolution and secrets](#7-resolution-and-secrets)
8. [Implementations](#8-implementations)
9. [Scripted hooks](#9-scripted-hooks)
10. [What a run records](#10-what-a-run-records)
11. [Errors](#11-errors)
12. [Versioning and compatibility](#12-versioning-and-compatibility)
13. [Obligations and open items](#13-obligations-and-open-items)

Worked examples — a provider verification run with five hooks across four points, and the message points
in both directions — live under [`examples/`](examples/), validated against these schemas (and against
designs 2.6's and 2.3's, where this design carries their documents) by
`cargo test -p pact_janus_schema_compat`.

---

## 1. Scope and conformance

**MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT** and **MAY** are to be interpreted as described in
RFC 2119.

Conformance roles:

- The **engine** invokes hooks: it decides when a point is reached, assembles the context, applies the
  result and reports what happened.
- A **loader** — the CLI, or an SDK's embedding layer — reads the project configuration, resolves it
  (§7) and passes the result to the engine. It is the only party that touches the file system or the
  environment.
- A **hook** answers an invocation: a component, a script, a command or an HTTP endpoint.

In scope: the point vocabulary and each point's context, mutability and default failure policy; the
ordering and chaining rules; the configuration document in both its authored and resolved forms;
interpolation, secret handling and redaction; the four implementations and how an embedding declares
which it can run; the scripted-hook runtime and its API; what a run records about hooks; the errors.

Out of scope, with owners:

| Question | Owner |
|---|---|
| how a hook **component** is invoked, and the result document every implementation answers with | design [2.6](../component-interfaces/spec.md) §8 |
| what a component may declare it changes, and how it is loaded, sandboxed and pinned | design [2.6](../component-interfaces/spec.md) §9–10, [ADR 0013](../../decisions/0013-component-hosting-is-an-embedding-capability.md) |
| variant-bound state parameters, and what `state-unavailable` does to a run | design [2.3](../variant-semantics/spec.md) §6, [ADR 0009](../../decisions/0009-variant-bound-provider-state-parameters.md) |
| the frames that carry hook configuration and hook events | design [2.1](../engine-protocol/spec.md) §8–9 |
| what a contract records — which is nothing about hooks (§7.4) | design [2.5](../contract-file/spec.md) |
| the CLI surface that loads a configuration file and reports hook activity | design 5.5 |
| the SDK surface a consumer test uses to reach the two consumer-side points | design 2.9 |

## 2. The model

### 2.1 A point is where the engine drives

A **lifecycle point** is a named place in a run where the engine stops and calls out. Points exist for
one reason: between "the engine has everything it needs" and "the engine has everything it needs except
something only this project knows", the second case is the normal one — the request has to be signed,
the fixture has to exist, the message has to come from the provider's own producer code. A point is
where that project-specific step is invited in, by name, in a configuration a reviewer can read.

Hooks are therefore *not* general extensibility. Anything that changes how matching works is a component
(design 2.6): a matcher, a content handler, a transport. A hook changes how a run is *performed*, and
never what a contract *means* — a distinction §7.4 makes normative.

### 2.2 Three scopes

Every point has a scope, which fixes how often it runs and what its context can contain:

| Scope | Runs | Context beyond `run` |
|---|---|---|
| **run** | once per verification run | the run descriptor; at the end, the summary |
| **exchange** | once per interaction × variant attempt | interaction, variant, parts, endpoint |
| **state** | once per provider state, per exchange | interaction, variant, and the one state |

An **exchange** is one interaction exercised at one variant — the unit
`verification/interaction-result` reports on (protocol §9.6) and the unit a hook failure can fail
without ending the run. The word matters because it is not "an interaction": an interaction with four
selected variants is four exchanges, each with its own state setup, its own request and its own result.

### 2.3 One pair of documents, four implementations

Every invocation, at every point, through every implementation, is the same two documents:

- the engine hands out a **`HookContext`** ([`hook-context.schema.json`](schemas/v1/hook-context.schema.json));
- the hook answers with an **`InvokeResult`** — design 2.6's
  [`hook.schema.json#/$defs/InvokeResult`](../component-interfaces/schemas/v1/hook.schema.json), not
  restated here.

The four implementations differ only in how those two documents are carried: as a component call, as a
function argument, as stdin and stdout, as a request and a response body. This is the property that makes
a hook portable — a signing hook prototyped as a script and later moved into a component is the same hook
handling the same document — and it is checked rather than asserted: the worked examples validate every
result in this design against design 2.6's schema, so a divergence between the two documents is a CI
failure rather than a discovery in Phase 5.

### 2.4 Why the consumer side has almost no points

Hooks exist where the **engine** drives. In a provider verification the engine owns the loop: it walks
interactions and variants, sets up states, sends requests and matches responses, and user code gets a
turn only where a point invites it. In a consumer test the relationship is inverted — the user's test
drives the engine, one call at a time, from inside a test framework that already has `beforeAll`,
`afterEach` and everything else a fixture needs. A consumer-side `before-request` point would be a worse
version of the line of code above it.

So the consumer side gets exactly the points where the engine drives something the test cannot reach:
`produce-message` and `consume-message` (§3.4–3.5), the two directions of a message exchange. Everything
else a consumer hook might want to do belongs somewhere better: dynamic values in a served response are a
**generator's** job (shape spec §3.4), transforming a body is a **content component's** job (design 2.6
§6), and test fixtures are the test framework's job.

Both sides read the same configuration document, because a project has one set of components, one set of
secrets and one place to look for them; only the point vocabulary differs, and a hook configured at a
point its role does not have is a configuration error naming the point (§11), never a hook that silently
never runs.

## 3. The points

The vocabulary is **open** (protocol §2.2 rule 1): later versions add points, and a hook that receives a
point it does not know answers `skipped` (§12.2). v1 defines eight.

| Point | Scope | Role | Mutable | Default on failure |
|---|---|---|---|---|
| `before-verification` | run | provider | — | `abort-run` |
| `state-setup` | state | provider | — | `fail-exchange` |
| `before-request` | exchange | provider | outbound part slots | `fail-exchange` |
| `produce-message` | exchange | both | `parts` | `fail-exchange` |
| `consume-message` | exchange | both | — | `fail-exchange` |
| `after-response` | exchange | provider | — (§3.6) | `fail-exchange` |
| `state-teardown` | state | provider | — | `warn` |
| `after-verification` | run | provider | — | `warn` |

Every point may return `data` (§4.4) and every point may be selected down with `when` (§6.3).

### 3.1 `before-verification`

Runs once, after the pacts have been read and the components resolved, before the first exchange.
Context: `run`, `config`. Nothing is mutable.

This is where a run acquires something every later hook needs — an access token, a database handle's
connection string, a container that has to be up. Its `data` lands in `run.data` under the hook's name
and is visible to every hook for the rest of the run (§4.4), which is the mechanism that keeps a token
fetch out of `before-request`, where it would run once per exchange.

Its default policy is `abort-run` because a run that could not complete its own preparation has not
tested anything, and the alternative — every subsequent exchange failing on a missing token — reports
the same fact once per interaction with the cause hidden in the noise.

### 3.2 `state-setup`

Runs once per provider state of the interaction, in the interaction's recorded state order, before the
exchange. Context: `run`, `interaction`, `variant`, `state` (the one state, with parameters already
resolved for this variant), `exchange`, `config`. Nothing is mutable — a state handler establishes
state; it does not edit the interaction that is about to be exercised.

It runs for **every variant, always**, including consecutive variants whose resolved parameters are
identical, and the verifier MUST NOT reorder variants to create such runs. Variant semantics §6.6 gives
the argument in full; the short form is that the precondition for skipping ("nothing has disturbed the
state") is one the verifier cannot check, and the thing most likely to have disturbed it is the exchange
that just ran.

Three answers, fixed by [ADR 0009](../../decisions/0009-variant-bound-provider-state-parameters.md) and
restated here because this is the point that produces them:

| Outcome | Meaning | Effect |
|---|---|---|
| `ok` | the provider is in the state | the exchange proceeds |
| `unsupported` (with a reason in `error.message`) | the provider cannot reach this state | the variant is `state-unavailable` (§5.5) |
| `failed` | the setup itself is broken | `on-failure` applies; default fails the exchange |

The distinction between the middle row and the last is the whole reason the point has three answers
rather than two: "I cannot produce that" is a contract finding whose remedy is a contract change, and
"my setup script threw" is a bug whose remedy is a fix. A summary that conflates them sends the reader to
the wrong team.

### 3.3 `before-request`

Runs after state setup, immediately before the outbound parts leave the engine, for every exchange whose
transport performs a request. Context: `run`, `interaction`, `variant`, `exchange`, `parts`, `endpoint`,
`config`. Mutable: the slots of the **outbound** parts — `parts.request.headers` and its siblings, as the
transport names them (design 2.6 §4).

This is today's request filter, and it is the point most runs use: signing, bearer tokens, correlation
ids, tenant headers. Two rules keep it from becoming a way to rewrite the test:

- it may change only what its entry **declared** (§4.3), so a hook that adds an `authorization` header
  cannot quietly rewrite a path;
- what it changed is **recorded by path** in the hook report (§10.2), so "which hook set this header" is
  answerable from the run rather than by bisecting the configuration.

The outbound parts it changes are the ones that go on the wire. The **contract is unchanged**: hooks are
not recorded (§7.4), and the request the contract describes is the request the consumer declared, not the
signed one that a particular provider run happened to send.

### 3.4 `produce-message`

Runs instead of an outbound wire exchange, when the interaction's transport binding says the message
comes from user code rather than from a transport. Context: `run`, `interaction`, `variant`, `exchange`,
`config`, with `parts` empty. The hook returns the produced message as a change to **`parts`** — the one
point where the whole parts document is the mutable member.

This is the message-provider verification case: the verifier asks the provider's own producer for the
message it would emit and matches it against the contract. On the consumer side it is the same
direction with the other party in it — a **passive** message interaction (protocol §7.4), where the
application under test is what emits and the engine matches what it emitted. Not a mirror: the mirror
is §3.5, where the engine produces and user code receives. That the roles swap and the direction does
not is the whole reason these points are named for the direction.

Normative, from spike 1.5 finding 6 and protocol §8.3: **a hook and a wire transport are interchangeable
sources of the same parts and MUST produce identical results.** The engine matches the parts it was
given; where they came from is not a matching input. §13 makes that a corpus obligation rather than a
promise.

### 3.5 `consume-message`

The other direction: the engine has message parts and needs user code to receive them. Context: `run`,
`interaction`, `variant`, `exchange`, `parts` (the message), `config`. Nothing is mutable; the hook's
outcome *is* the answer — `ok` if the consumer accepted the message, `failed` with an error if it did not.

Provider-side, this verifies a provider that is itself a consumer: the recorded message is handed to its
handler and the handler's acceptance is the result. Consumer-side, it is the delivery path for an
**emissive** interaction (protocol §8.2): `consumer-session/serve-variant` produces the message and this
hook puts it where the application under test will pick it up.

That one point serves both roles is not a coincidence to be tidied away. Spike 1.5's finding is that
mocking and driving differ by *direction, not kind*; naming the two points by direction rather than by
role is that finding written into the vocabulary.

### 3.6 `after-response`

Runs when the inbound parts have arrived and before matching begins. Context: `run`, `interaction`,
`variant`, `exchange`, `parts` (outbound and inbound), `endpoint`, `config`. **Nothing is mutable in v1.**

A hook that could rewrite the response before matching would be editing the evidence, and the failure
mode is not hypothetical: the header normalised away is the one whose absence the consumer would have
noticed, and the run still reports a pass. The needs that motivate response mutation have better homes —
decoding, decrypting or reshaping a body is a **content component**'s job (design 2.6 §6), where the
transformation is declared, versioned, namespaced and recorded in the contract as a requirement; ignoring
volatile fields is a **shape**'s job (design 2.2), where the tolerance is written into the contract that
a subsumption check can then read.

So `after-response` is for observing: capture a trace id, assert on a side channel, ship a diagnostic. It
is the one point where v1 deliberately offers less than today's tooling, and §13 records what would make
us revisit it.

### 3.7 `state-teardown`

Runs after the exchange, once per state, in **reverse** order of setup, and runs whether the exchange
passed, failed or never ran. Context: as `state-setup`, plus `exchange.outcome`. Nothing is mutable.

Reverse order because teardown undoes setup, and a state that was established last is the one whose
removal the others may depend on. It runs after failures because the state a failed exchange left behind
is exactly the state that will make the next variant fail for the wrong reason.

Its default policy is `warn`, not `fail-exchange`: the exchange has already produced its result, and a
teardown failure cannot retroactively change what the provider answered. It is reported — loudly, in the
hook report and the summary — because leaked state poisons later exchanges, and §13 carries the tripwire
for making it stronger.

### 3.8 `after-verification`

Runs once, after the last exchange, whether the run passed, failed or aborted. Context: `run`, `summary`
(the same summary document the terminal event carries, protocol §9.6), `config`. Nothing is mutable — a
hook cannot revise a verdict that has already been reached, which is a property worth having rather than
a limitation to work around.

Default policy `warn`, for the same reason: the run has its result, and a reporting hook that failed
should not turn a passing verification into a failing one.

## 4. Ordering, chaining and changes

### 4.1 The shape of a run

```
before-verification                      (run scope, once)
  for each interaction, for each selected variant:          ← an exchange
    state-setup            per state, recorded order
    before-request | produce-message      (whichever the transport binding calls for)
    [ the exchange happens ]
    after-response | consume-message
    state-teardown         per state, reverse order
after-verification                       (run scope, once, even after an abort)
```

The alternatives on those two lines are exclusive: an interaction whose message comes from
`produce-message` has no outbound request to filter, and one whose result is a consumer's acceptance has
no response to observe.

### 4.2 Order is declaration order

Hooks at one point run **in the order they are declared**, one at a time, never concurrently, and each
sees the changes the ones before it made. There is no priority number, no dependency graph and no
`before`/`after` keys.

This is a deliberate refusal. Ordering metadata makes the order a function of every entry in the file
rather than of the sequence a reader can see, and the resulting question — "which of these actually runs
first" — is answered by re-implementing the sort in one's head. A list is already an order. A project
that needs a different one edits the list.

Concurrency is refused for the same reason it is refused for variants: two hooks mutating the same parts
concurrently make the result depend on scheduling, and hooks are a handful of calls per exchange, so
there is nothing to win.

### 4.3 A change is declared, permitted, applied, recorded

A `changes` map in a result is applied member by member, and each key must pass three tests:

1. **Permitted at the point** — the key is within the point's mutable set (§3). `parts.request.headers`
   at `before-request` is permitted; the same key at `after-response` is not.
2. **Declared by the entry** — the key appears in the entry's `changes` list (§6.2). For a component
   hook, the entry's list may only *narrow* what the component declared in its handshake (design 2.6
   §8.2), never widen it: two declarations, and the engine takes the intersection.
3. **A dotted path into the context** — `parts.request.headers`, `parts` — addressing a member the
   context actually has.

A key that fails any of them is refused, the invocation is `failed` with `hook-change-refused` in the
error, and the point's failure policy applies. Nothing is partially applied: a result whose changes
include a refused key has none of its changes applied, because a hook that half-ran is a state no author
tested.

The context handed to the next hook carries the applied changes, and `mutable` in every context tells a
hook what it may touch *before* it tries. Declaring mutation up front is what makes a chain reviewable
from its configuration: the file says which hooks run at a point and what each may touch, so the answer
to "what could have rewritten this header" is a page, not an investigation.

### 4.4 `data`, and where it is visible

A result's `data` is opaque to the engine and is the only way a hook talks to a later hook:

- from a **run**-scope point, it lands in `run.data[<hook name>]` and is visible for the rest of the run;
- from an **exchange**- or **state**-scope point, it lands in `exchange.data[<hook name>]` and is cleared
  when the exchange ends.

Keyed by hook name, so a reader of a script that consumes `run.data["auth-token"]` can find the entry
that produced it. Cleared per exchange, so nothing leaks into the next variant by accident — the same
reasoning that makes state setup re-run.

`data` is **not** reported by default (§10.2): a hook's scratch is where credentials naturally land.

## 5. Failure semantics

### 5.1 Outcomes

| Outcome | Meaning |
|---|---|
| `ok` | the hook did its job; changes, if any, are applied |
| `skipped` | the hook decided this invocation was not for it; no changes, not a failure |
| `unsupported` | state points only: the provider cannot reach the state (§5.5) |
| `failed` | the hook could not do its job; `error` says why |

Three more the *engine* concludes when the hook does not answer at all, recorded distinctly because they
send a reader to three different places (§10.2): `timed-out` (the deadline passed), `errored` (the
process died, the endpoint was unreachable, the script threw, the component trapped), and — for a
component — whatever design 2.6 §11.2 synthesised on its behalf.

### 5.2 Policies

`on-failure` takes one of three values, defaulting per point (§3):

| Policy | Effect |
|---|---|
| `abort-run` | the run stops; the summary carries `aborted` with the point, the hook and the error, and the count of exchanges never run (§10.2). `after-verification` still runs. |
| `fail-exchange` | this interaction-and-variant attempt is `failed`, with the hook error as its cause; remaining hooks at the point do not run; the run continues with the next exchange. Teardown still runs. |
| `warn` | recorded, reported, and the run continues — including the remaining hooks at that point. |

The defaults follow one rule: **the default is never the quiet one where quietness would mislead.** A
`before-request` hook that failed to sign a request would produce a 401 the report would attribute to the
provider; failing the exchange with the hook's own error names the real cause. A `state-teardown` hook
that failed cannot change a result that already exists, so it warns.

### 5.3 A hook failure is a result, not a protocol error

A hook that ran and failed is **test outcome information**, not an `EngineError`: the machinery did its
job, and what it learned is that a step of the run did not work. It travels as events and as the run's
result, exactly as a mismatch does (protocol §10.2).

Errors are reserved for the cases where there is no run to report into: a configuration that could not be
resolved, an implementation kind the embedding cannot run, a point that does not exist for the role. All
of them are detectable **before** the first exchange, and §11 requires them to be reported there.

### 5.4 Deadlines

The engine MUST bound every invocation in time and MUST surface expiry as an outcome, never as a hang
(spike 1.4 finding 6, design 2.6 §3.4). `timeout-ms` defaults to 5000 at exchange and state points and
30000 at run points, where a container start is the normal case. The deadline is in the context as
`deadline-ms`, so a hook that talks to something slow can pick its own timeout inside the engine's rather
than being killed mid-call.

Expiry is a failure and takes the point's `on-failure` policy. For scripts the bound is enforced by an
interrupt handler around every invocation (§9.5); for `exec` by killing the process group; for `http` by
abandoning the request; for components by the binding's deadline.

### 5.5 `state-unavailable`, and its waiver

A `state-setup` hook answering `unsupported` makes the variant `state-unavailable`, which **fails the run
by default** and is reported as its own status, not as `failed` (ADR 0009, variant semantics §6.7). The
run continues to the next variant: an unproducible state is a fact about one region of the variant space,
and stopping would hide the rest.

The waiver is variant semantics §3.5's exclusion machinery, not a hook setting: an `allow-state-unavailable`
entry names the interaction and the variant, carries a required reason, and is reported as *waived*
rather than as passing. Nothing in hook configuration can turn `state-unavailable` into a pass — there is
deliberately no `on-failure: warn` path to it, because the switch that would be set once during an
incident is the one that outlives everyone who understood why.

## 6. Configuration

### 6.1 One document, two file names

A project writes [`project-config.schema.json`](schemas/v1/project-config.schema.json): conventionally
`verifier.pact.yaml` on the provider side and `consumer.pact.yaml` on the consumer side, the same
document either way, in YAML or JSON (the schema governs the parsed document; design 5.5 owns which file
names the CLI looks for).

```yaml
version: 1

components:
  - name: csv
    source: { kind: oci, reference: ghcr.io/acme/janus-csv:1.2.0, digest: "sha256:9f2c…" }

hooks:
  before-verification:
    - name: auth-token
      run: { kind: http, url: "${AUTH_URL}/token", headers: { authorization: "Basic ${AUTH_BASIC}" } }
  state-setup:
    - name: fixtures
      run: { kind: http, url: "${PROVIDER_URL}/_pact/state", format: pact-state-change }
  before-request:
    - name: sign-requests
      run: { kind: script, path: ./hooks/sign.ts }
      config: { secret: "${JANUS_SIGNING_SECRET}" }
      changes: ["parts.request.headers"]
```

Components and hooks share the file because they answer the same question — what does this project load,
and what may it do — and splitting them would put a component's grants on a different page from the hook
that needs them. The `components` member is design 2.6's and is validated there.

### 6.2 A hook entry

| Member | Meaning |
|---|---|
| `name` | required; `[a-z][a-z0-9-]*`, unique within its point. It is how the hook appears in events, in the report, in errors and in `explain`. A hook that cannot be named in a failure message is a hook nobody can debug. |
| `run` | required; the implementation, tagged by `kind` (§8) |
| `config` | the hook's own configuration, handed to it as `context.config` |
| `when` | which occurrences of the point this hook runs at (§6.3) |
| `changes` | the context paths it may replace (§4.3). Absent means it can change nothing. |
| `timeout-ms` | per-invocation deadline (§5.4) |
| `on-failure` | `abort-run` \| `fail-exchange` \| `warn` (§5.2) |
| `report-data` | whether its `data` is reported (§10.2); default false |

### 6.3 Selectors

`when` narrows a hook to some occurrences of its point: `interaction` (a description), `state` (a state
name — at state points this is *which state this handler handles*), `transport` (a kind). Members are
exact string matches and are AND-ed; an absent member matches everything.

No globs, no regexes, no expressions. This is the same refusal variant semantics §3.5 makes for
exclusions, for the same reason: a selector that must be *evaluated* to be understood cannot be reviewed,
and the failure it produces — a hook that silently matches nothing — looks exactly like a hook that was
never needed. A project with ten state handlers writes ten entries, and the file says which state each
one is for.

### 6.4 What the configuration deliberately lacks

No conditionals, no variables beyond `${VAR}` interpolation, no includes, no templating of structure, no
priorities. Every one of them is a step toward a configuration language, and the endpoint of that road is
a program written in YAML with no debugger. A project whose hook logic needs branching writes it in the
script, the command or the component, where it is code in a language with tooling.

## 7. Resolution and secrets

### 7.1 What the loader does

[ADR 0014](../../decisions/0014-hooks-are-resolved-configuration-not-callbacks.md): between the file and
the engine there is exactly one transformation, performed by the loader, and it is what separates
[`project-config.schema.json`](schemas/v1/project-config.schema.json) from
[`hook-config.schema.json`](schemas/v1/hook-config.schema.json):

1. **interpolate** every `${VAR}` reference from the environment (§7.2);
2. **inline** every script: read `run.path`, transpile it if it is TypeScript, and put the JavaScript in
   `run.source`;
3. **resolve** relative paths (`cwd`, component sources) against the configuration file's directory;
4. **validate** the result against the resolved schema and reject anything left over.

The engine therefore receives a document with no templates, no paths and no file references, and holds no
callbacks into the host — the protocol has no engine-to-host call and gains none here (ADR 0005). Three
things follow, and they are the reasons for the split: a run does not depend on the directory it was
started in or the environment of the process that hosts the engine; the WASM-embedded engine, which can
read no files at all, runs the same hooks as the native one; and the document that determined a run's
behaviour is one document, which can be attached to a failure report.

### 7.2 Interpolation

`${VAR}` in any string member is replaced with that environment variable's value. One form only: no
defaults, no nesting, no shell expansion. An **unset variable fails the load**, naming it — never an
empty string, because the empty string is how `${TOKEN}` becomes a run of 401s that look like a provider
bug.

The literal `$` is written `$$`. Interpolation happens once, on the authored document; a value that
happens to contain `${…}` after substitution is a value, not a template, and is not re-scanned.

### 7.3 Redaction

Interpolated values are the project's secrets, and the engine's rule about them is simple: **it never
sends them back out.**

- A hook's `config` is passed to the hook and appears in nothing else — not in events, not in the hook
  report, not in the summary, not in `explain` output, not in logs. Renderings of a hook show its name,
  its point and its implementation kind.
- `run.env` and `run.headers` — where a credential reaches an `exec` or `http` hook — are treated
  identically.
- A hook's `data` is carried to later hooks but is **reported only when its entry sets
  `report-data: true`** (§10.2). This refines design 2.6 §8.2's "opaque hook output surfaced to the host":
  the surfacing is opt-in, because the fetch-a-token hook is precisely the one whose output must not
  appear in CI logs.
- A hook that logs a secret has logged it (§9.3's `janus.log` writes what it is given). The engine cannot
  fix that, and this specification does not pretend otherwise.

### 7.4 Hooks leave no trace in the contract

A contract records what the parties agreed, not how a particular run was performed. So: no hook
configuration, no hook names, no hook output, and no hook-modified parts are ever written to a contract
(design 2.5). The request the contract describes is the one the consumer declared; the signature a
provider run added on the way out is a property of that run.

The one place hook activity reaches a durable artifact is a variant's **status**: `state-unavailable`
comes from a `state-setup` hook and is recorded as a verification result, because it is a fact about the
contract — the provider cannot produce a state the consumer declared. That is the exception that proves
the rule: it is recorded as a *finding about the contract*, not as a note about a hook.

## 8. Implementations

`run.kind` selects one of four. They are ordered here by how much of the engine they need.

### 8.1 `component`

`run: { kind: component, component: <name> }`. The named component must be declared in the same
configuration and implement the hook interface; the engine calls `hook/invoke` with the context of §2.3
and applies the result (design 2.6 §8). The component's handshake declares which context members it may
change per point, and the entry's `changes` may narrow that further (§4.3).

This is the implementation for hooks that are *products*: versioned, distributed, digest-pinned, sandboxed
by grants (ADR 0013), and reusable across projects. It is also the only implementation with a per-call
cost measured in microseconds (spike 1.4).

### 8.2 `script`

`run: { kind: script, source: <javascript>, entry?: <function name> }`, authored as `path: ./hooks/sign.ts`
and inlined by the loader. §9 specifies the runtime and the API.

This is the low-friction default and the only implementation available in **every** embedding, because
the interpreter compiles with the engine (ADR 0015).

### 8.3 `exec`

`run: { kind: exec, command, args?, cwd?, env? }`. The engine spawns the command per invocation, writes
the context document to its stdin and closes it, and reads the result document from stdout:

- exit 0 with a parseable result → that result;
- exit 0 with empty stdout → `{ "outcome": "ok" }` (a command that succeeded and had nothing to say);
- exit 0 with unparseable stdout → `failed`, code `hook-result-invalid`, with the first bytes of stdout in
  the error details;
- non-zero exit → `failed`, with the exit status and the tail of stderr as the message.

stderr is captured (bounded; the tail is kept) and attributed to the hook in the run's diagnostics.
The child's environment is **exactly** `run.env` — deny-by-default, the same posture ADR 0013 takes for
component grants — so a hook cannot pick up an ambient credential the configuration does not show.
The deadline kills the process group, so a spawned child that outlives its parent does not outlive the
run.

Spawning per invocation costs milliseconds, which is fine at a handful of calls per exchange and wrong
for a hook called on every one of a thousand. That case already has an answer and it is not a flag on
this one: a long-lived process that speaks the same frames is a **subprocess component** (design 2.6
§9.3), which is `kind: component` with a `subprocess` source.

### 8.4 `http`

`run: { kind: http, url, method?, headers?, format? }`. The engine POSTs the context document to the
endpoint; a 2xx response with a JSON result body is the result, a 2xx with an empty body is
`{ "outcome": "ok" }`, and a non-2xx is `failed` carrying the status and a bounded excerpt of the body.

`format: pact-state-change` sends the v3/v4 provider-state body instead — `{ "state", "params", "action" }`
with `action` `"setup"` or `"teardown"` from the point — so **an existing state-change endpoint keeps
working unchanged**. That is B5's actual migration test: a provider that has a state endpoint today
should be verifiable by Janus by naming it in a config file, not by rewriting it. The response is read the
same way, which means a v3 endpoint that returns nothing is `ok` and one that returns
`{"outcome": "unsupported", …}` reaches the state-unavailable path without knowing what Janus is.

Availability depends on the embedding's network access, which for the WASM-embedded engine is a grant and
not a given (§8.5).

The same implementation is how an SDK can offer callback-shaped ergonomics without the protocol growing a
call it does not have: the test process serves a loopback endpoint and the configuration names it
([ADR 0014](../../decisions/0014-hooks-are-resolved-configuration-not-callbacks.md)). From the engine's
side that is an ordinary `http` hook, which is the point — the sugar costs the protocol nothing.

### 8.5 What an embedding can run

Implementations are an **embedding capability**, declared in the handshake exactly as component loaders
are ([ADR 0013](../../decisions/0013-component-hosting-is-an-embedding-capability.md)):

```json sketch
{ "capabilities": { "hooks": { "implementations": ["component", "script", "http"] } } }
```

`script` is present in every embedding. `exec` requires process spawning, which a WASM-embedded engine
does not have. `http` requires outbound network access, which it has only if the host granted it.
`component` is present wherever a loader is (ADR 0013's `components.loaders`).

A configuration naming an implementation the embedding cannot run fails **before the run starts**, with
`hook-unavailable` naming the kind, the hook and the implementations that *are* available (§11) — the
same shape as an unsatisfiable component requirement, and for the same reason: a degraded run that
silently skipped a signing hook is worse than no run.

## 9. Scripted hooks

### 9.1 The runtime

[ADR 0015](../../decisions/0015-quickjs-as-the-scripted-hook-runtime.md): **QuickJS, embedded via
`rquickjs`, compiled with the engine in all three embeddings.** Spike 1.6 measured it as the only
candidate that is JS (what users ask for), builds for the WASI targets (what the primary embedding
requires), is the smallest JS option, and exposes no ambient capabilities by default. The spike's one
open item — whether the C-to-WASI build survives the engine's own component build — was closed before the
ADR: `rquickjs` builds for `wasm32-wasip2`, produces a component, runs the spike's own hook workload
under wasmtime and honours an interrupt handler there (spike 1.6 §4).

### 9.2 The entry point

A script is a single JavaScript source. The runtime evaluates it once per instantiation and then calls
the function named by `entry` (default `hook`) once per invocation:

```js sketch
function hook(ctx) {
  const headers = janus.json(ctx.parts.request.headers) || {};
  headers.authorization = "Janus " + sign(ctx, ctx.config.secret);
  return { outcome: "ok", changes: { "parts.request.headers": janus.slot(headers) } };
}
```

One script may serve several points and branch on `ctx.point`; several entries in the configuration may
name the same source with different `entry` functions. A missing entry function is a configuration error
detected when the script is first instantiated, not at the first invocation of the point.

Returning nothing means `{ "outcome": "ok" }` with no changes — the common case for an observing hook.
Throwing is `errored` with the exception's message; a script cannot crash a run in any other way.

### 9.3 The context API

[`hook-api.d.ts`](hook-api.d.ts) is the specified surface: the context and result documents as
TypeScript, plus one standard-library object, `janus`, with five functions — `text`, `json`, `bytes` and
`slot` for reading and building slot values (design 2.6 §4's wrapper, whose unconditional form is exactly
what makes helpers necessary), and `log` for the only output a script has.

There is nothing else in scope. No `require`, no `fetch`, no file system, no environment, no timers —
not as a policy the runtime enforces but as a fact about how it is built (spike 1.6 finding 6): a bare
interpreter has no ambient capabilities, and everything a hook needs from outside arrives in
`ctx.config`, which the loader filled in.

### 9.4 Hooks are synchronous

A hook function returns a result; it does not return a promise, and the runtime pumps no job queue in v1.
A hook that must wait on I/O is an `exec`, `http` or component hook — implementations that already have
the machinery for it. The pattern that covers most of what an async script would want is `run.data`: fetch
once at `before-verification` with an `http` hook, read the result in every `before-request` script.

### 9.5 Every invocation is bounded

The runtime installs an **interrupt handler** around every invocation, tripped by the deadline of §5.4;
the invocation ends as `timed-out` and the point's policy applies. This is not optional: no interpreter in
the bake-off interrupts a runaway script by default, and a hook that spins would otherwise hang a
verification. Verified in the WASM build, where the host's epoch deadline bounds it from outside as well
(spike 1.4).

### 9.6 TypeScript

TypeScript is a loader concern. A `.ts` hook is transpiled (esbuild, swc or the SDK's existing toolchain)
before it reaches the engine, which only ever sees JavaScript. Types come from
[`hook-api.d.ts`](hook-api.d.ts), so a hook is checked against the same declarations this specification
ships.

Documents this design carries but does not own are carried unchanged: `parts` and the endpoint
descriptor are design 2.6's, the states are design 2.5's, and `variant.assignment` is design 2.3's array
of `{ dimension, point }`. A hook reads them in the shape their owner uses, and a script finds a
dimension's point with a `find` rather than a lookup, because a second shape for the same fact is a
second thing to keep in step.

### 9.7 What a script may rely on

The ECMAScript language and its standard built-ins, including `JSON`, `Math`, `Date` and typed arrays.
`Date` and `Math.random` are available and are the author's responsibility: a hook that puts a random
value into a signed header is fine, and one that puts a timestamp into a *matched* part would be fighting
the contract, which is what generators exist for (shape spec §3.4).

## 10. What a run records

### 10.1 Events

Every invocation produces one `verification/hook` event (protocol §9.6) carrying the invocation record of
§10.2. Events are the RFC's requirement that hook activity be observable; they arrive on the same stream,
in the same order, with the same delivery guarantees as everything else, so a host can render hook
activity inline with interaction results.

### 10.2 The hook report

The run summary carries a [`HookReport`](schemas/v1/hook-report.schema.json): every invocation in order,
each with its point, hook name, implementation, outcome, the effect the policy gave it, the exchange and
variant it belonged to, its duration, and **the context paths it changed — names, never values.**

That last choice is the point of the document. "Which hook rewrote this header" is the question a hook
system either answers or spends the rest of its life not answering, and the answer needs the path; the
value would put a signed header, a token or a body into every log that carries a summary. `data` appears
only when the entry set `report-data`, for the same reason (§7.3).

Hooks that never ran are absent, and hooks that ran and changed nothing are present with an empty
`changed` — the difference matters, because only one of the two means a selector is wrong.

An aborted run reports `aborted`: the point, the hook, the error and how many exchanges never ran, kept as
a count of things not done rather than folded into the failed tally.

### 10.3 `explain`

Plans do not contain hooks. A plan is how matching happens (design 2.4), and a hook is how a run is
performed; putting hooks in the plan grammar would make a document that must be stable across runs
depend on the configuration of one.

`explain --executed` renders the hook report alongside the executed plan, in run order, showing name,
point, outcome and changed paths. Design 5.4 owns the rendering; this specification owns the fact that the
information is available in the same stream, at the same time, and needs no second mechanism to collect.

## 11. Errors

Configuration failures are `EngineError`s, all detected **before the first exchange**. The first two are
in the protocol's `document` category — they are about a document the user authored, surfaced with
positions (protocol §10.2); the third is in `component`, because "this embedding cannot run that" is the
same fact as an unsatisfiable component requirement and a host should handle it the same way:

| Code | Raised when | `details` |
|---|---|---|
| `hook-config-invalid` | the document does not validate; a duplicate name within a point; a `changes` path that is not a context path; a point that does not exist for the role; an entry whose `run` names no kind | `problems: [{path, message}]` |
| `hook-unresolved` | resolution could not complete: an unset `${VAR}`, an unreadable script path, a transpile failure. Raised by the **loader**, which is where the file system is. | `variable` or `path`, and the hook |
| `hook-unavailable` | an implementation kind the embedding cannot run, or a `component` hook naming an undeclared component | `kind`, `hook`, `implementations` (what is available) |

Runtime hook failures are **not** errors (§5.3): they are outcomes, in events and in the hook report. The
one exception is the abort path, which is still not an error frame — the run ends with its terminal event
carrying a summary whose `aborted` member says which hook stopped it.

Component hook failures reach the report as design 2.6 §11 specifies, with the component's own error
passed through verbatim: the kernel does not translate component error interiors, and this design does not
either. The same holds for the other three implementations: the `code` inside a hook's own `error` is the
hook author's open vocabulary — `state-unreachable`, `handler-rejected` — and the engine records it as
given. It is the hook's word for what went wrong, and rewriting it into an engine code would lose the only
thing it was carrying.

## 12. Versioning and compatibility

### 12.1 Three things that version separately

The **configuration document** (`version: 1`) versions with this design. The **point vocabulary** grows
without a version bump, being an open vocabulary. A **hook's own** implementation versions however its
author likes — a component by its version (design 2.6 §2.3), a script by the repository it lives in.

### 12.2 Growing the vocabulary

A new point is additive: engines that have it invoke it, engines that do not never do, and a
configuration naming an unknown point is a `hook-config-invalid` naming it rather than a silently ignored
block — an ignored hook is indistinguishable from a hook that ran, and the ignored one is how an unsigned
request reaches a provider.

In the other direction, a hook handed a point it does not know MUST answer `skipped` rather than guessing
what to do. That is the open-discriminator rule (protocol §2.2 rule 3) at the hook boundary.

A point's **context may grow** members additively; a point's **mutable set may grow** but MUST NOT shrink
within a version, because shrinking it turns a working hook into a refused change. A point's default
`on-failure` is part of its specification and changing it is a version bump — the difference between
`warn` and `fail-exchange` is the difference between a green build and a red one, and no project should
discover it from a patch release.

### 12.3 What a hook may rely on

That the two documents of §2.3 are the same in every implementation; that its `config` is fully resolved;
that it will be called at most once per point per exchange per entry, sequentially, with the previous
hook's changes applied; that it will be bounded in time and told the bound. Everything else — the order
of interactions, which variants were selected, whether another hook is configured at the same point — is
the run's business and may change between runs.

## 13. Obligations and open items

This design makes claims that later tasks must test rather than inherit. They are listed here so the
Phase 9 report accumulates evidence instead of reconstructing it.

| Claim | Who tests it | What would falsify it |
|---|---|---|
| A hook and a wire transport are interchangeable sources of the same parts (§3.4) | 4.3 (message transports), corpus pair per spike 1.5 finding 6 | the same parts producing different results by origin |
| The same hook behaves identically as a script, an `exec` and a component (§2.3) | 5.4, extending 2.6's conformance corpus to the hook path | any implementation-visible difference beyond latency |
| `pact-state-change` makes an existing v3 state endpoint work unchanged (§8.4) | 5.2, against a real provider from the migration corpus | an endpoint that needs edits to verify |
| Scripted hooks are viable in all three embeddings (§9.1) | 4.2 and 5.4, on the real engine build, not the spike's runner | the QuickJS component build regressing → the ADR's Boa fallback |
| Per-invocation `exec` spawning is fast enough at prototype scale (§8.3) | 1.7's benchmark harness once hooks exist | spawn cost visible in verification wall-time |

Two deliberate v1 limitations, with their tripwires:

- **`after-response` mutates nothing** (§3.6). If Phase 5 finds a real case that is neither a content
  component's nor a shape's — a response that must be transformed before matching, where the
  transformation is not part of what the contract means — the point gains a declared, recorded mutable
  set in a later version. The bar is a case, not a preference.
- **`state-teardown` warns by default** (§3.7). If teardown failures turn out to poison later exchanges in
  practice, the default becomes `abort-run`, which is a version bump per §12.2. Variant semantics §6.6
  carries the related tripwire — a state its author declares variant-independent — and if that arrives it
  is a member of this design's state entries.
