# 0003 — Ship dual engine artifacts; set embedding priority per SDK language

- **Status**: proposed
- **Date**: 2026-08-23
- **Plan tasks**: 1.2, 1.3, 1.8 (G1)
- **Evidence**: [spike 1.2 findings](../../spikes/1.2-wasm-embedding/FINDINGS.md) (embedding
  matrix), [spike 1.3 findings](../../spikes/1.3-subprocess-embedding/FINDINGS.md) (subprocess
  lifecycle + pipe cost)

## Context

The RFC asserts "WASM preferred" with a subprocess fallback (bet B1). Spike 1.2 hosted a toy
engine speaking the ADR-0002 byte-pipe from Node, JVM, Go, and Python: the **component model** is
natively hostable only in wasmtime embeddings (wasmtime-py loads it directly; Chicory and wazero
reject the binary; Node needs the `jco transpile` build step, which works well); the **zero-import
core module + 3-function shim** works everywhere at ~30 lines of host code, because the frozen
pipe needs exactly one function. Spike 1.3 proved the subprocess embedding end-to-end in the same
four languages — version handshake, clean shutdown, orphan-free kill behaviour via
exit-on-stdin-EOF, 0.6–1.2 ms spawn-to-ready — with a pipe tax of ~5–10 µs per call and ~0.8 ms
per 100 KB. Two inversions matter: at document scale the subprocess currently *beats* the
practical in-process WASM path on the JVM (1.7 ms vs 5.2 ms per 100 KB) and Python (2.2 ms vs a
75× marshalling penalty in wasmtime-py's dynamic component API), and interpreter-mode runtimes
are disqualified outright (Chicory interpreter: 300+ ms per document operation).

Distribution is the other axis: per-OS native binaries are the historical pain the RFC set out to
remove, so "no native artifact to ship" weighs as heavily as latency.

## Decision

1. **One engine build produces three artifacts**: the WASM component (canonical), the zero-import
   core WASM module exposing the frozen C-ABI shim, and the `pact-engine` subprocess executable.
   All speak the ADR-0002 frames; the per-host shim is written once per SDK and frozen.
2. **Subprocess is a first-class embedding, not a fallback** — proven, cheap, and in some
   languages currently the fastest practical path. It is also the universal escape hatch for any
   language without a validated WASM host.
3. **Primary embedding per language** (fallback in parentheses):
   - **Node/TS**: WASM component via jco transpile (subprocess) — no native binary, negligible
     overhead, works today.
   - **JVM**: core module on Chicory's bytecode-compiler backend (subprocess) — the pure-JVM,
     zero-native-dependencies story ends pact-jvm's per-OS binary distribution pain; its 5 ms
     per 100 KB is acceptable, and the subprocess remains available where throughput dominates.
     Never the interpreter backend.
   - **Python**: core module via wasmtime-py's classic API (subprocess) — upstream ships platform
     wheels, so binary distribution is outsourced. The dynamic component API is barred until it
     gains a `list<u8>` fast path.
   - **Go**: core module via wazero (subprocess) — pure Go, no cgo.
   - **.NET**: subprocess until wasmtime-dotnet is validated (expected to match the wasmtime
     story; no toolchain was available in 1.2).
   - **Ruby, PHP, and other SDKs**: subprocess primary until a credible WASM host is validated
     per language.
   - **CLI**: the native engine binary by definition.
4. **The kernel keeps a zero-import discipline** for the core-module artifact (1.2 finding 7: WASI
   leakage is the component-hosting tax; the zero-import build is what made pure-JVM and Go
   hosting trivial).

## Alternatives considered

- **Subprocess everywhere**: simplest, but reinstates per-OS binary shipping for every SDK — the
  distribution failure mode this redesign exists to remove. Retained as the universal fallback
  instead.
- **Component model everywhere**: not real outside wasmtime embeddings today (rejected binaries in
  Chicory/wazero; Node requires transpilation anyway).
- **Native FFI (status quo)**: the failure modes B1 removes; measured as the 1.7 baseline, never
  an embedding.

## Consequences

Easier: every SDK language has a working embedding today with numbers behind it; the frozen shim
means embedding code is written once per SDK; WASM-vs-subprocess can be swapped per deployment
without protocol changes.

Harder: the build pipeline must produce and test three artifacts from Phase 3 on; the
wasm32-wasip2 build discipline (no ambient imports) constrains kernel dependencies permanently;
a real-Windows CI job is required before the subprocess claims are hardened (1.3's one open
risk); Chicory and wasmtime-py versions become tracked compatibility surfaces.

Tripwires: wasmtime-py growing a bytes fast path flips Python's primary to the component API;
Chicory gaining component-model support (or a JVM perf regression) reopens the JVM choice;
the 1.7 trend line showing WASM marshalling dominating real workloads reopens the per-language
table.
