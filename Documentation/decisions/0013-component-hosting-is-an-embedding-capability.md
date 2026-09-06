# 0013 — Make component hosting a negotiated embedding capability, and distribute out-of-tree components as digest-pinned OCI artifacts

- **Status**: accepted
- **Date**: 2026-08-30
- **Plan tasks**: 2.6 (design; minimal build in 8.2, escape hatch in 8.3)
- **Evidence**: [spike 1.2](../../spikes/1.2-wasm-embedding/FINDINGS.md) (the engine as a WASM guest;
  the zero-import core module); [spike 1.3](../../spikes/1.3-subprocess-embedding/FINDINGS.md)
  (Content-Length framing over stdio: a 30–60 line client per language, orphan prevention by
  stdin EOF, self-synchronising on a malformed body);
  [spike 1.4](../../spikes/1.4-engine-hosting-plugins/FINDINGS.md)
  (wasmtime as the component host: instantiation cost, sandbox defaults, epoch deadlines, WASI import
  leakage as attack surface); [spike 1.5](../../spikes/1.5-message-transport-shape/FINDINGS.md)
  (transports and ambient capability); [ADR 0003](0003-embedding-priority-per-language.md),
  [ADR 0012](0012-one-interface-two-bindings.md)

## Context

ADR 0012 makes the four component interfaces one surface with two bindings. That leaves a question it
deliberately does not answer: *where can an out-of-tree component actually be loaded?*

The answer is not uniform, and this is the first design document to say so. Spike 1.4 hosted WASM
components with stock wasmtime — discovery, sandboxing, trap containment, epoch deadlines, all free.
Spike 1.2 hosted the *engine* as a WASM component in four languages. Composing the two is where it
stops: hosting a component needs a runtime with code generation, and there is none inside a
`wasm32-wasip2` guest. So an engine embedded as WASM — ADR 0003's primary embedding for most SDK
languages — can run in-tree components and nothing else. The native (CLI) and subprocess embeddings can
host everything.

This is the same shape of fork spike 1.6 found for scripted hooks, and it must be surfaced the same
way: as a capability a host can *ask about*, not as a run-time surprise.

The RFC's word for the out-of-process hatch is "gRPC", carried over from today's pact-plugins. That
choice is worth revisiting rather than inheriting, because since the RFC was written spike 1.3 has
specified and measured a subprocess pipe for the engine itself — and the component boundary carries the
same frames (ADR 0012), so it can carry the same framing.

Two further constraints come with the loading path. Third-party components arrive as bytes from
somewhere, so the engine needs a naming, fetching, caching and integrity story. And some components —
transports especially — need ambient capability a sandbox exists to deny: raw sockets, long-lived
servers, an existing native client library. Spike 1.4 finding 7 notes the inverse problem, that a
component built with `std` demands ten WASI interfaces it never uses, so the import set is a surface to
govern rather than merely tolerate.

## Decision

**1. Hosting is a declared engine capability.** `engine/hello` carries a `components` capability naming
the loaders the running embedding can use: `in-tree`, `wasm`, `subprocess`. The WASM-guest embedding declares
`in-tree` only. A host learns what its engine can load before it submits work, through machinery the
protocol already has (protocol §5.3).

**2. An unsatisfiable requirement is a named failure, never a degraded run.** A contract requiring
`content/protobuf >= 2` on an embedding that cannot load it fails with `component-unavailable`, naming
the requirement and the loaders the embedding does have. Silent success on a subset of a contract's
interactions is the failure mode this project spends its exclusions machinery preventing.

**3. Out-of-tree WASM components are OCI artifacts, referenced and cached by digest.** A project config
names a component, its reference (`oci://…`), and — for reproducible runs — its digest. The engine
resolves the reference, caches content-addressed by digest, verifies the digest before instantiation,
and indexes the component's contributions from its handshake. Versions are compared by **major only**,
against the version the handshake declares, never against the tag.

**4. Capability grants are deny-by-default and declared per component.** A WASM component's imports are
checked against the grants its declaration gives it; surplus imports are a load failure, not a run-time
denial. This costs nothing at run time (spike 1.4: a default `WasiCtx` satisfies the imports with
nothing) and it makes the sandbox reviewable in the config rather than implicit in the runtime.

**5. The out-of-process escape hatch is a spawned subprocess speaking the protocol's own stdio framing,
not gRPC.** A component that needs ambient capability runs as a process the engine spawns, exchanging
the same frames behind the same `Content-Length` framing spike 1.3 specified for the engine's own
subprocess pipe. Grants are *not* enforceable there — it runs with the engine user's authority — and
the spec says so in those words. That is the trade for raw sockets, and it is the reason the hatch
exists rather than the sandbox being widened.

## Alternatives considered

- **Host-trampolined component calls**: the engine returns a "needs component call" response frame and
  the host, which *can* host components, satisfies it and calls back. Preserves the frozen pipe, adds no
  imports, and works in every embedding — but it puts a component runtime in every SDK, which is
  directly against "SDKs are thin", and it adds a second control-flow shape to a protocol whose whole
  value is having one. Recorded here because it is the answer if the capability split proves too
  painful in practice; it is not the answer now.
- **"Use the subprocess engine" with no negotiation**: true, but a host finds out by failing. A
  capability costs one member in a handshake that already exists.
- **gRPC for the out-of-process hatch**, as the RFC names it. Rejected on three counts, none of them
  aesthetic: it is a *second* framing in a project whose whole architecture is one frozen pipe; it puts
  a code-generation step in front of an author's first working component, on the extension path where
  friction is the entire cost (spike 1.3 finding 6: the stdio client is 30–60 dependency-free lines per
  language); and it forfeits orphan prevention, which stdin-EOF gives for free in every language and
  gRPC would have to rebuild out of process groups, job objects or heartbeats (spike 1.3 finding 2).
  The interop argument for it is illusory — the frames differ from pact-plugins' entirely, so keeping
  its transport would buy the wire and not the protocol. `grpc` stays available as a loader name if
  evidence ever demands it; the vocabulary is open.
- **Bundling third-party components into the engine artifact**: defeats the extension path entirely.
- **Resolving components by capability search rather than by explicit declaration**: an engine that goes
  looking for something to satisfy `content/protobuf` is an engine whose runs are not reproducible.
  Components are declared; requirements are checked against what was declared.

## Consequences

Easier: the extension story has one distribution model, one integrity check, and a sandbox posture that
can be read off a config file. A project that needs a third-party component knows which embeddings can
run its tests, before running them.

Harder: SDK authors must surface the capability rather than assume it, and a project can now be portable
by embedding rather than portable outright. The subprocess hatch also inherits spike 1.3's one open
risk — real-Windows validation of spawn, kill and EOF exit — which now applies to component processes
as well as to the engine's own, and belongs in the same CI matrix rather than being tracked twice. Component authors targeting the widest reach will prefer
the WASM binding, which means preferring interfaces that do not need ambient capability — a bias worth
naming, since it points third-party effort at content and matcher components and away from transports.

**Tripwire.** If real projects routinely need a third-party component — so that "WASM preferred"
degrades in practice to "subprocess everywhere" for the SDKs that most wanted WASM — then ADR 0003's
priority ordering, not this ADR, is what needs revisiting, and the trampoline alternative above comes
back on the table with evidence behind it.
