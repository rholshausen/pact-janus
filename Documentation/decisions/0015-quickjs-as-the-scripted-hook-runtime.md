# 0015 — Make QuickJS the scripted-hook runtime, with the hook context as a script's entire capability surface

- **Status**: accepted
- **Date**: 2026-09-01
- **Plan tasks**: 2.7, from spike 1.6 (feeds 4.2, 5.4, 6.2)
- **Evidence**: [spike 1.6](../../spikes/1.6-script-hook-bakeoff/FINDINGS.md) (five engines benchmarked
  natively, three inside wasmtime, plus §4's `wasm32-wasip2` verification for this ADR),
  [spike 1.2](../../spikes/1.2-wasm-embedding/FINDINGS.md) (the engine as a WASM component),
  [spike 1.4](../../spikes/1.4-engine-hosting-plugins/FINDINGS.md) (epoch deadlines around code the
  engine does not control), [ADR 0003](0003-embedding-priority-per-language.md) (which embedding is
  primary), [ADR 0014](0014-hooks-are-resolved-configuration-not-callbacks.md) (how a script reaches the
  engine)

## Context

The RFC sketches a `wasm-script` hook and leaves the language open; feedback from prior Lua embeddings in
this ecosystem is that people want JS or TypeScript. The plan sent the question to a bake-off because the
deciding constraint is architectural, not a matter of taste: **the script runtime must work in all three
embeddings**, and the primary one is an engine that is itself a WASM component, where a natively embedded
V8 cannot live.

Spike 1.6 measured five engines natively and three inside wasmtime, with one build failure and one
maturity elimination. Its conclusion was that performance decides nothing here — the slowest engine
measured costs about 3 % of matching a single 100 KB document, and hooks run a handful of times per
exchange — while embedding compatibility decides everything. It recommended QuickJS and left exactly one
question open: whether the C-to-WASI build survives inside the engine's own `wasm32-wasip2` component
build, as opposed to the `wasm32-wasip1` runner the spike measured.

That question is now answered. Building `rquickjs` 0.9 for `wasm32-wasip2` produces a **component**
(1.0 MB, the component-model preamble, no manual componentisation step); it runs the spike's hook workload
under wasmtime 35 at ~9 µs per call; and an interrupt handler installed inside that build stops a
`while (true) {}` script at its deadline. Recorded as spike 1.6 §4.

## Decision

**QuickJS, embedded via `rquickjs`, compiled with the engine in all three embeddings, is the scripted-hook
runtime.** Five commitments:

1. **JavaScript is the hook language; TypeScript is a tooling question.** A `.ts` hook is transpiled by
   the loader (ADR 0014) and the engine only ever sees JavaScript. No engine choice depends on it.
2. **The hook context API is the entire capability surface.** A bare QuickJS context has no `require`, no
   `fetch`, no file system, no environment and no timers, and the engine binds none: a script sees its
   context document and a five-function standard library (`janus.text/json/bytes/slot/log`). Everything a
   hook needs from the outside arrives in `ctx.config`, which the loader filled in. Sandboxing is
   therefore a property of what is bound, not a policy that has to be enforced — spike 1.6 finding 6
   inverted the usual risk and this is what it inverts to.
3. **Every invocation is bounded by an interrupt handler.** No interpreter in the bake-off interrupts a
   runaway script by default; the engine installs the handler around every call, and expiry is an outcome
   (`timed-out`), never a hang. In the WASM embedding the host's epoch deadline bounds it from outside as
   well (spike 1.4).
4. **Hooks are synchronous in v1.** No promise pumping, no job queue. A hook that must wait on I/O is an
   `exec`, `http` or component hook, and the fetch-once-reuse-everywhere pattern is served by run-scope
   hook data.
5. **The polyglot escape hatch stays "bring your own component".** A script compiled ahead of time into a
   WASM component — the StarlingMonkey/ComponentizeJS shape, or any other language — is a hook
   *component* (design 2.6 §8), which already exists and needs nothing from this decision.

Spec text: [Lifecycle hooks specification](../specs/lifecycle-hooks/spec.md) §9; API surface
[`hook-api.d.ts`](../specs/lifecycle-hooks/hook-api.d.ts).

## Alternatives considered

- **V8 (`rusty_v8`).** The performance ceiling: 1.2 µs per call, 6.7× QuickJS. Native-only, so scripted
  hooks would become a subprocess-embedding feature and the primary embedding would lose them entirely;
  29.5 MB of distribution on top. Rejected on availability, not speed.
- **Lua (`mlua`).** The fastest embeddable interpreter measured (3.6 µs, 0.03 ms cold, 0.9 MB) — and its
  build script refuses the WASI target outright, so it cannot ride inside the WASM-embedded engine. It is
  also the only engine whose *default* configuration exposes `io` and `os`. Rejected on the constraint the
  plan predicted, with user preference already against it.
- **piccolo** (pure-Rust Lua). Would have rescued the Lua path; 0.3.x, last released two years ago.
  Rejected on maturity.
- **Rhai / Starlark** (Rust-native DSLs). Clean sandbox, mid-pack performance, pure Rust — and a new
  language for users whose stated preference is JS. A DSL only wins if the JS options fail the embedding
  test; they did not.
- **Boa** (pure-Rust JS). Runs the workload correctly everywhere the engine compiles with no C toolchain
  at all — the simplest build story of the JS options — at ~3× QuickJS per call and ~5× its size, with
  historically partial spec coverage. **Kept as the designated fallback**, not the default.

## Consequences

Easier: the low-friction hook path is available in *every* embedding, which is what makes `script` the
default recommendation over `exec` and `http`; a hook script is portable across engine builds; the
capability surface is small enough to specify in one `.d.ts`.

Harder: the engine's build now includes a C toolchain step for QuickJS, on every target including WASI.
That is the maintenance risk the fallback exists for.

Committed to: a bare interpreter with no ambient capabilities, so any future convenience (an HTTP client
for scripts, a file read) is a capability decision with an ADR, not a library import someone adds.
Tripwire: if the QuickJS component build becomes fragile in the engine's own `wasm32-wasip2` build — a
toolchain upgrade that breaks it, or a target where it will not build — switch to Boa, which costs
per-call latency that spike 1.6 finding 2 measured as not load-bearing, and keeps every other property of
this decision intact.
