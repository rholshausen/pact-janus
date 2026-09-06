# AI-layer design notes

Plan task **2.10**. Status: **draft**. No build; no schema; no ADR — the task is explicitly scoped as
"one short doc," and this is it.

The RFC's [AI-assisted verification](../../roadmap/rfc/0000-pact-mkii.md) section sketches two optional
capabilities on top of the deterministic core — **mismatch diagnosis** (feed `explain --executed` to an
LLM) and **agentic verification** (a task description drives an agent through provider setup) — and says
plainly that "the *judgment* of pass/fail always remains with the deterministic engine; the agent only
performs orchestration." This note's job is narrow: check that designs 2.1–2.9, written without either
capability in mind, don't accidentally close a door the RFC wants left open. It is a **positioning**
check, not a design — nothing here adds a schema, a protocol operation, an event kind, or a hook
implementation. Everything the two capabilities need already exists for other reasons; what follows is
where.

The SDK specification (design 2.9 §8) already drew this line once, for the AI-*assisted regeneration*
idea — the conformance suite keeps judgment, the agent proposes a patch. This note is the same line drawn
for the two verification-time capabilities.

## 1. Mismatch diagnosis: `explain --executed` as input

The RFC's example is concrete: "the provider renamed `shippedAt` to `shipped_at` in the invoice variant."
That is a summary of an *executed* plan's failure, and the executed form is already exactly the input an
LLM would need, for reasons none of which were chosen with this in mind:

- **It is a specified surface, not a debugging convenience.** Plan grammar spec §3.1 fixes the pretty
  form's sigils and layout as something "a renderer MUST produce exactly," and §3.2's executed form is
  the same tree with `=> <result>` appended per node — one renderer, two modes. A diagnosis tool reads a
  governed text form, not an implementation detail that can drift under it.
- **It is complete, not first-failure.** Plan grammar spec §2.3: "an interpreter MUST execute every node
  whose inputs are available, so that one run reports every mismatch rather than the first." A
  summarizer sees the whole failure, including sibling mismatches a first-failure trace would have hidden.
- **It already leaves the engine as a document, not a side channel.** Engine protocol spec §9.6's
  `verification/executed-plan` event is "emitted when `verify` options request it — this is how
  `explain --executed` gets its input." The event carries the same document/text the CLI renders; a
  diagnosis tool subscribing to the verification stream gets it the same way the CLI does, with no
  engine-side awareness that the consumer on the other end is an LLM instead of a terminal.
- **Every node that failed carries its own location and message** — `resolve` nodes name a path
  (`$.body.shippedAt`), `%expect:*` actions carry the comparison, and an `error` result carries "a
  message and the path that located it" (plan grammar spec §2.3). A rename-detection summary is string
  work over fields this design already put in the trace for a human reader.

Subsumption findings (design 2.8 §4) are the structurally identical case one layer over: a `finding`
already names its `path` (shape spec §6.2's dimension-path grammar) and a `kind` from an open vocabulary
(§4.2's asymmetric-rule table), and the report's text rendering (§6.4) is governed the same way the
executed form is. A diagnosis tool that also narrates *why a provider stopped being subsumed* reads the
same kind of document, not a different one.

**What this means for 2.1–2.9**: nothing changes. A diagnosis tool is a new, additive consumer of an
event and a text form that already exist for `janus explain --executed` and `janus can-i-deploy`-style
reporting. It needs no new engine operation, no new event kind, and no protocol version bump.

**Deferred**: the prompt/summarization logic itself, evaluating diagnosis quality against real
mismatches, and whether the CLI ships a `--explain-with-ai` flag or this stays a separate tool consuming
the same stream. None of that is designed here.

## 2. Agentic verification: the task format

The RFC's ask is "a mode where the verifier emits, from the pact file, a task description that an AI
agent uses to stand up/configure the provider, satisfy provider states, and run verification." Read
literally this sounds like a new artifact — a "task description" schema. It is not: the pieces an agent
would need are already named, and already flow to exactly the kind of process an agent would run as.

**The "task description" is the union of documents a run already hands to hooks, not a new one.**

- `verification/verify`'s request (engine protocol spec §8.3) carries `target` — "transport bindings...
  plus open options such as state-change configuration" — which is the provider side of "stand up the
  provider" today, before any agent is involved.
- `before-verification`'s context (lifecycle hooks spec §3.1) is `run, config` at run scope — "where a
  run acquires something every later hook needs... a container that has to be up." This is the
  RFC's "stand up... the provider" step, already scoped and already invoked once per run.
- `state-setup`'s context (lifecycle hooks spec §3.2) is `run, interaction, variant, state (the one
  state, with parameters already resolved for this variant), exchange, config` — this *is* "satisfy
  provider states," per exchange, with the state's parameters already resolved per variant-semantics
  spec §6.4 and bound per [ADR 0009](decisions/0009-variant-bound-provider-state-parameters.md).
- The contract itself (design 2.5) is the declarative "what": interactions, parts, shapes and exercised
  variants, readable by anything that can parse JSON — an agent does not need a bespoke export of the
  pact file, because there is nothing in it the hook context documents don't already unpack per-exchange.

**The implementation surface an agent runs behind already exists.** Lifecycle hooks spec §2.3 fixes the
model as "one pair of documents, four implementations" — a `HookContext` in, an `InvokeResult` out,
carried as "a component call, a function argument, as stdin and stdout, as a request and a response
body." An agent standing up a provider is not a fifth implementation kind; it is a process behind `exec`
(§8.3) or a service behind `http` (§8.4), the same way `pact-state-change` lets an existing v3 state
endpoint verify unchanged. Nothing about a hook's two-document contract cares whether the process on the
other end is a shell script or an LLM-driven agent loop.

**Judgment stays with the engine, structurally, not by promise.** A `state-setup` hook answers exactly
one of three outcomes — `ok`, `unsupported` (with a reason), `failed` (lifecycle hooks spec §3.2) — and
the engine, not the hook, decides what each means for the run. An agentic `state-setup` hook cannot
assert a pass; it can only claim the provider reached a state, and the actual exchange, executed by the
plan interpreter against the real response, is what produces the verdict. This is the RFC's "the agent
only performs orchestration" made mechanical: the outcome vocabulary an agent can return is three words,
not prose the engine has to trust.

**What this means for 2.1–2.9**: nothing changes. Agentic verification composes `exec`/`http` hooks
(already specified) over context documents (already specified) describing a contract (already specified).
No new hook implementation kind, no new HookContext member, no new verification operation.

### Open items this note flags but does not design

- **No v1 vocabulary bundles a whole run's context up front.** An agent instructed to "stand up the
  provider" for an entire pact might want to see every state and exchange it will eventually be asked
  about before the run starts, not discover them one `state-setup` call at a time. Nothing here proposes
  such a bundle — `before-verification`'s run-scope context is deliberately minimal (§3.1) — but a future
  design that wants one should define it as a `data` value a `before-verification` hook computes and
  publishes into `run.data` (§4.4), not as a change to the point vocabulary.
- **Redaction matters more once a hook's context can reach a prompt.** Lifecycle hooks spec §7.3's
  redaction rules exist so secrets never land in the hook report or `explain` output; if an agentic hook's
  context or its `InvokeResult.error.message` is later routed into an LLM prompt or a diagnosis tool, that
  redaction boundary is the one already doing the relevant work, and any future agent integration should
  be reviewed against it rather than inventing a second one.
- **Deadlines may need a longer bound for a slow agent call.** §5.4's per-invocation deadline is already
  a per-hook-entry config value, not a fixed constant — an agent-backed hook needing more time than a
  script is a configuration choice under an existing knob, not a mechanism gap.
- **Authorizing an agent to configure a real provider is a real security boundary**, and this note takes
  no position on it beyond noting that it is exactly the boundary `exec`/`http` hooks already cross today
  for human-written state-change scripts — an agent behind the same implementation kinds inherits the
  same trust model, for better or worse, and does not need a new one invented for it.

## 3. What this note checked, and found nothing to change

| Capability | Needs a new... | Found |
|---|---|---|
| Mismatch diagnosis | protocol operation | No — reads the existing `verification/verify` stream |
| Mismatch diagnosis | event kind | No — `verification/executed-plan` (engine protocol §9.6) already exists |
| Mismatch diagnosis | text form | No — the executed form (plan grammar §3.2) already exists |
| Agentic verification | protocol operation | No — `verification/verify`'s existing `target`/`options` carry provider setup |
| Agentic verification | hook implementation kind | No — `exec`/`http` (lifecycle hooks §8.3–8.4) already carry an arbitrary process/service |
| Agentic verification | HookContext member | No — `state-setup`'s and `before-verification`'s existing context (lifecycle hooks §3.1–3.2) already names interaction, variant, resolved state, config |
| Agentic verification | schema | No — the contract (design 2.5), the hook-context schema (design 2.7), and the hook InvokeResult schema (design 2.6) already cover it |

That emptiness is the deliverable: the plan's own wording is "so the deterministic design doesn't
accidentally preclude them," and the check comes back negative on every row.

## 4. Explicitly deferred

Not designed here, and not implied by anything above:

- the LLM prompt, summarization logic, or model choice behind mismatch diagnosis;
- the agent orchestrator/loop behind agentic verification, its retry policy, or how it decides which
  hook points to attempt;
- a standard, versioned "task description" document, if one is ever wanted beyond the per-point
  `HookContext` documents this note points to;
- evaluating diagnosis quality, agent reliability, or cost/latency budgets for either capability;
- the security review an agent with provider-control-plane access would need before real use.

Charter non-goals already say this plainly: "the AI-assisted layer (design notes only)." This document is
that note, and its scope ends here.
