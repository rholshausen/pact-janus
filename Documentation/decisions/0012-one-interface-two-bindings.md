# 0012 — Define components as the engine protocol's frames turned around, and give the interface two bindings

- **Status**: proposed
- **Date**: 2026-08-30
- **Plan tasks**: 2.6 (answers the RFC's day-one-components question; feeds 3.8, 4.2, 8.1, 8.4)
- **Evidence**: [spike 1.4](../../spikes/1.4-engine-hosting-plugins/FINDINGS.md) (plugin-side byte-pipe,
  handshake discovery, sandbox, trap containment, deadlines, costs);
  [spike 1.2](../../spikes/1.2-wasm-embedding/FINDINGS.md) (engine as WASM guest, WASI leakage);
  [spike 1.1](../../spikes/1.1-idl-bakeoff/FINDINGS.md) (the evolution gauntlet);
  [spike 1.5](../../spikes/1.5-message-transport-shape/FINDINGS.md) (transport primitives);
  [ADR 0002](0002-document-first-protocol-over-frozen-pipes.md),
  [ADR 0003](0003-embedding-priority-per-language.md)

## Context

The RFC asks whether "everything is a component" is a day-one architecture or a target — whether HTTP
and JSON may be kernel-privileged initially. The project plan tells this task to *try* full symmetry and
report where it hurts, because the report is the evidence the question needs.

Two things are being decided under one heading, and they come apart cleanly:

1. **What the interface is.** Spike 1.1's evolution gauntlet showed a typed IDL cannot express a growing
   vocabulary without breaking old readers, and spike 1.4 confirmed the inverse direction on the plugin
   surface: handshake-declared contributions over the frozen byte-pipe did everything — action
   discovery, plan-integrated invocation, structured errors — and "nothing in this spike wanted a richer
   WIT surface". The interface question is therefore already answered by ADR 0002; it only has to be
   applied.
2. **How a built-in is wired to it.** This is the contested half. "Built-ins implement exactly these
   interfaces" is an architectural claim about *shape*; "built-ins are loaded exactly like plugins" is a
   much stronger claim about *packaging*, and it is the one that carries costs.

Two measurements bound the packaging question. Spike 1.4 measured the pipe at 1.4 µs per matcher call
and 2.6 ms for a 100 KB document round trip — double-ended serde, paid per body by whichever component
decodes it. And spike 1.2 established the constraint that decides it: the engine's preferred embedding
is a WASM component, and a WASM guest cannot host WASM components — there is no wasmtime inside a
`wasm32-wasip2` guest. If "built-in" meant "loaded like a plugin", the embedding ADR 0003 made primary
would ship with no HTTP transport and no JSON content handler at all.

Meanwhile the shape language (§3.5) and the plan grammar (§4.6) have already committed the other
direction: `status-code` is not a core operator, `json:parse` is not a core action, and both documents
say in terms that the kernel resolves neither itself. Privileging HTTP and JSON now would put that
knowledge back where it is hardest to see — inside the interpreter's dispatch table.

## Decision

**1. A component interface is the engine protocol's frame shape, turned around.** The engine calls a
component with `RequestFrame`s and receives `ResponseFrame`s carrying `ok` or a structured error, over
the same frozen byte-pipe (`call: func(request: list<u8>) -> list<u8>`), governed by the same
open-world authoring rules and the same CI checker. Contributions — operators, actions, content types,
transports, hook points — are declared in a handshake, never in a type. The engine is the only caller:
a component never calls back, and no event frames exist on this pipe.

**2. Four interfaces**: `transport`, `content`, `matcher` (matching and generating), `hook`. A component
is a package that may implement several; its **name is its namespace** for contributed operators and
actions, so names are unique within a resolution scope whatever interfaces they fill. Requirements name
a role: `content/protobuf >= 2` (contract spec §7).

**3. Built-ins implement exactly those interfaces, through a second binding.** In-tree components are
resolved through the same registry, declare the same contributions, answer the same operations with the
same documents — but are invoked through a **native binding**: a specified projection of the frame
surface onto in-process calls that passes documents as values instead of serialising them. The
interface is one interface; the wire under it is an implementation choice made per component, not a
different contract.

**4. The symmetry claim is made falsifiable, not asserted.** Every in-tree component MUST also be
runnable through the byte-pipe binding, and CI runs one component conformance corpus through both
bindings, requiring identical results. "Writing a plugin is documented by reading the core" stops being
a hope the moment the core is exercised through the plugin path on every build.

## Alternatives considered

- **Full packaging symmetry — built-ins loaded exactly like third-party components.** Killed by
  hosting, not by cost: the primary embedding cannot host components at all (ADR 0013), so this option
  would leave the WASM engine with no transport and no content handler. The per-body serde would have
  been a reason to dislike it; the embedding is what makes it impossible.
- **HTTP/JSON kernel-privileged for the prototype.** Cheapest, and it forfeits exactly the evidence the
  RFC asked this prototype to produce. It also contradicts two accepted-in-review specs that already
  namespace the content and transport vocabulary.
- **A typed IDL (WIT) per interface.** Killed by spike 1.1's gauntlet and spike 1.4 finding 2: the
  vocabularies these interfaces carry are exactly the ones that grow, and a typed surface makes growth
  breaking. The same reasoning that produced ADR 0002 applies unchanged one layer down.
- **Contributions declared in a static manifest** (so the engine can index without instantiating).
  Rejected: two sources of truth for what a component provides, one of which is unverifiable until the
  component runs. The handshake is the truth.

## Consequences

Easier: one surface to specify, document, test and version; a fragment or action author writes against
the same grammar whichever side of the boundary they sit on; the RFC's "documented by reading the core"
claim becomes a CI job. The kernel-boundary review (task 3.8) gets a mechanical check rather than a
reading exercise.

Harder: we now owe a *specified projection*. For every operation, what the native binding does with a
document must be stated well enough that a divergence is a test failure rather than a judgement call —
and two bindings are two implementations to keep in step. Batching (`matcher/apply` takes many values)
exists in the interface from day one so the pipe binding has a lever the native binding does not need.

We are committed to: no operation existing in one binding and not the other; a built-in that panics
being converted at the dispatch boundary exactly as a WASM trap is; and an in-tree component never
reaching into engine internals, because the conformance run would not survive it.

**Tripwires.** (a) If an operation cannot be expressed identically in both bindings — different
documents, not merely different performance — then this is not one interface and 2.6 must be reopened.
(b) If the out-of-tree component in task 8.1 cannot be written without reading engine source, the
symmetry is nominal and the report should say so plainly rather than defend the design.
