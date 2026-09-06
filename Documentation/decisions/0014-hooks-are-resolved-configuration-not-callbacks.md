# 0014 — Declare hooks in configuration a loader resolves, and give the engine values rather than callbacks, paths or templates

- **Status**: accepted
- **Date**: 2026-09-01
- **Plan tasks**: 2.7 (feeds 5.2, 5.4, 6.2, 6.3; tests benefit B5)
- **Evidence**: [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146) ("Lifecycle hooks",
  `verifier.pact.yaml` in the RFC's own naming — Janus's equivalent is `verifier.janus.yaml`, §Decision
  1), [spike 1.2](../../spikes/1.2-wasm-embedding/FINDINGS.md) (the engine as a WASM
  guest: no ambient file system, no environment, no spawning),
  [spike 1.3](../../spikes/1.3-subprocess-embedding/FINDINGS.md) (what an out-of-process hop costs and
  what it buys), [ADR 0002](0002-document-first-protocol-over-frozen-pipes.md),
  [ADR 0005](0005-poll-based-event-delivery.md) (the host calls the engine; the engine does not call
  back), [ADR 0013](0013-component-hosting-is-an-embedding-capability.md) (deny-by-default grants)

## Context

Today's equivalent of a hook is a **callback registered in the test process**: a JVM request filter, a
JS `requestFilterer`, a state-change handler closed over the test's fixtures. It is the most natural
thing to reach for, and every SDK that has one had to write it again.

Janus cannot have it, and the reason is structural rather than stylistic. The protocol has one direction:
the host calls the engine, the engine answers, and events are polled (ADR 0002, ADR 0005). A registered
callback would require the engine to call *out* to the host mid-operation, which means a reverse channel
on every pipe — including the WASM component embedding ADR 0003 makes primary, where the engine is a
guest that cannot originate a call into the embedder's language runtime at all. Adding it would be the
single largest change to the protocol's shape, in exchange for a feature the RFC already specifies
declaratively.

The question this ADR settles is therefore not "callbacks or configuration" — the RFC and the protocol
have answered that — but **what the engine receives**, which is where the real choices are. A
configuration file contains `${AUTH_TOKEN}` references, script paths relative to the file, and a
`components` block with relative sources. Something must turn those into values. If it is the engine,
then the engine reads the environment and the file system, which the WASM embedding cannot do and the
other two should not: a run whose behaviour depends on the working directory of the process hosting the
engine is a run that reproduces on one machine and not another.

## Decision

**Hooks are declared in project configuration; a loader resolves that configuration; the engine receives
a closed document.**

Four commitments:

1. **Configuration, not callbacks.** Hooks are named entries in `verifier.janus.yaml` (and its
   consumer-side twin), each naming one of four implementations — a component, a script, a command, an
   HTTP endpoint. No engine-to-host call exists, and none is added. An SDK that wants callback-shaped
   ergonomics builds them *on top*: a loopback `http` hook served by the test process is a callback in
   every way that matters to a user, and costs the protocol nothing.
2. **The loader resolves; the engine executes.** The loader — CLI or SDK embedding layer — interpolates
   `${VAR}` from the environment, reads and inlines script sources (transpiling TypeScript), and resolves
   relative paths. The engine reads no files, resolves no paths and expands no variables.
3. **Two schemas, because the difference is checkable.** The authored form (`project-config.schema.json`)
   permits templates and script paths; the resolved form (`hook-config.schema.json`) permits neither. One
   schema with optional members would make "already resolved" an unverifiable claim, and the first engine
   to helpfully expand a leftover `${VAR}` would put environment access back in the kernel. An unset
   variable fails the load naming the variable — never an empty string, which is how `${TOKEN}` becomes a
   run of 401s that look like a provider bug.
4. **Values in, names out.** A hook receives its configuration as values; the engine never echoes those
   values into events, reports, summaries, contracts or logs, and records what a hook *changed* by path,
   never by value. Hook output (`data`) reaches later hooks always and reports only on opt-in.

Spec text: [Lifecycle hooks specification](../specs/lifecycle-hooks/spec.md) §6–§7; schemas
`project-config.schema.json` and `hook-config.schema.json`.

## Alternatives considered

- **Registered host callbacks** (today's request filters). Requires a reverse channel on every pipe;
  impossible in the WASM component embedding; and it puts orchestration back in the SDKs, which the
  architecture forbids.
- **The engine reads the configuration file.** One less moving part, and it hands the kernel a file
  system and an environment — the two things spike 1.2 showed the WASM embedding does not have, and the
  two things that make a run depend on where it was started.
- **One schema with optional template members.** Cheaper to write, and it makes "the engine received
  resolved values" a convention rather than a property. Conventions at a trust boundary are how secrets
  leak.
- **A templating language in the config** (conditionals, includes, defaults such as `${VAR:-x}`). Each
  step is small and the destination is a program written in YAML with no debugger. Branching belongs in
  the script, the command or the component, where it is code in a language with tooling.
- **Passing secrets by reference and letting the engine dereference them** (a vault URL the engine
  fetches). Moves credential access into the kernel and makes the engine a party to the project's secret
  management. The loader already has the environment; it is the right place.

## Consequences

Easier: a run is reproducible from one document, which can be attached to a failure report; the same
hooks work in all three embeddings, including the one that cannot read a file; secrets are named in the
repository and valued only in the environment of the process that starts the run; "which hook changed
this" is answerable from the run's own report.

Harder: a hook cannot close over the test process's memory. The provider-side case that hurts is an
in-process fixture — a JUnit verification whose state handler wants the same in-memory repository the
test set up. The escape hatch is real but not free: the SDK serves a loopback endpoint and configures an
`http` hook against it, which is an extra hop and a port. Design 2.9 owns whether SDKs ship that sugar;
the protocol does not change either way.

Committed to: any future need for an engine-to-host call is a protocol change with its own ADR, not a
hook feature. Tripwire: if Phase 6 finds that both SDK prototypes end up shipping the loopback shim and
users treat it as the default path, the honest conclusion is that in-process hooks are a first-class
requirement, and it should be answered at the protocol level rather than by every SDK reinventing the
shim.
