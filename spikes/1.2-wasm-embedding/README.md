# Spike 1.2 — WASM component embedding matrix

Plan task 1.2 · Feeds gate G1 · Findings in [FINDINGS.md](FINDINGS.md) (the durable artifact — code
here is disposable).

## Question

The RFC asserts "WASM preferred" for embedding the engine in consumer test runners. Does the WASM
**component model** actually work today from each SDK host language, or does each land on core WASM
plus hand-built shims? And what does it cost (cold start, per-call overhead, marshalling a realistic
~100 KB JSON document)?

This spike also closes the risk 1.1 left open: the toy engine speaks the **document-first protocol
over a frozen byte-pipe** (minimal WIT world, JSON frames governed by the protocol's schemas), so
hosting it per language is exactly the ergonomics test the draft G1 recommendation needs.

## Method

1. **Toy engine** (`engine-toy/`): one Rust logic crate (echo + a real structural type-matcher +
   handshake with capability list; unknown ops answered as structured errors, errors as values),
   built two ways:
   - `component/` — WASM **component** via cargo-component: world `pact:toy-engine/engine` exporting
     `call: func(request: list<u8>) -> list<u8>`.
   - `core-module/` — **core** WASM module (`wasm32-unknown-unknown`) with a C-ABI shim
     (`alloc`/`call`/result read out of linear memory) for hosts without component-model support.
2. **Host per language** (`hosts/`): Node 22 (built-in WebAssembly; jco transpile for the
   component), JVM 17 (Chicory), Go (wazero), Python 3.14 (wasmtime-py). .NET (wasmtime-dotnet) is
   skipped — no dotnet toolchain on this machine; noted in findings.
3. **Per host, record**: component model or core+shim (and what the shim costs in code);
   binding/embedding ergonomics; cold start (compile+instantiate); per-call overhead (small echo);
   marshalling cost for a ~100 KB JSON document (echo and match) — `payloads/` holds the generator.
4. Output: the per-language embedding matrix in FINDINGS.md.

## Layout

```
engine-toy/logic/        Shared engine logic (frames in/out, serde_json)
engine-toy/component/    cargo-component build — the frozen byte-pipe WIT world
engine-toy/core-module/  Core-WASM build with C-ABI shim
hosts/node|jvm|go|python Host embedding + benchmark per language
payloads/                ~100 KB JSON document generator
FINDINGS.md              The embedding matrix and findings
```
