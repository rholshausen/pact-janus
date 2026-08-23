# Spike 1.2 findings — WASM component embedding matrix

Status: **complete** for Node / JVM / Go / Python; .NET (wasmtime-dotnet) **not run** — no dotnet
toolchain on the spike machine, and nothing below suggests it would change the shape of the answer
(wasmtime-dotnet is the same wasmtime embedding Python exercised). Method: [README.md](README.md).

## 1. What was hosted

The toy engine implements the 1.1 draft G1 protocol shape deliberately: a **frozen byte-pipe**
(`call: func(request: list<u8>) -> list<u8>`) carrying JSON frames — handshake with capability
list, echo, a real recursive structural type-matcher, unknown ops answered as structured `err`
frames. One Rust logic crate, built two ways:

- **Component** (cargo-component, wasm32-wasip1 + adapter): 107 KB, exports
  `pact:toy-engine/pipe@0.1.0`. Rust `std` drags in **ten WASI 0.2 imports** (cli, io, clocks,
  filesystem) the engine never deliberately uses — every component host must satisfy them.
- **Core module** (wasm32-unknown-unknown): 72 KB, **zero imports**, C-ABI shim
  (`alloc`/`call`/`dealloc`, packed ptr+len return, host does the pointer math).

Benchmarks per host: cold start; small echo (~35 B); echo of a 100 KB order document; type-match of
that document against itself (~200 KB frame). Same iteration counts everywhere (see
`hosts/node/bench-common.mjs`). Numbers are medians on one Linux x86-64 machine — treat as
order-of-magnitude, not as the 1.7 baseline.

## 2. The matrix

| Host | Component model? | Path benchmarked | Cold start | echo small | echo 100 KB | match 100 KB |
|---|---|---|---|---|---|---|
| **Node 22** (V8) | Not built-in — `jco transpile` build step generates JS bindings | transpiled component + preview2-shim | 3.2 ms (first 6.0) | 5.4 µs | 1.34 ms | 1.47 ms |
| Node 22 (V8) | — | core + hand shim | 0.1 ms | 1.1 µs | 1.33 ms | 1.37 ms |
| **JVM 17** (Chicory 1.4) | **No** — parser rejects binary (`unknown binary version`) | core + shim, interpreter | 7.5 ms | 161 µs | **302 ms** | **414 ms** |
| JVM 17 (Chicory 1.4) | — | core + shim, bytecode compiler | 18 ms (first 112) | 7.9 µs | 5.2 ms | 6.6 ms |
| **Go** (wazero 1.12) | **No** — `invalid version header` | core + shim | 17 ms compile once, then 66 µs/instance | 1.5 µs | 2.2 ms | 2.7 ms |
| **Python 3.14** (wasmtime-py 48) | **Yes** — dynamic API, `Linker.add_wasip2()` | component, dynamic calls | 7.9 ms | 47 µs | **97.9 ms** | **109.9 ms** |
| Python 3.14 (wasmtime-py 48) | — | core + shim (classic API) | 6.1 ms | 64 µs | 1.32 ms | 1.50 ms |
| .NET | not run | — | — | — | — | — |

## 3. Findings

1. **The component model is not a portable hosting story today.** Of the four hosts: two runtimes
   (Chicory, wazero) reject the component binary outright at the version header; Node has no
   built-in support (same rejection from `WebAssembly.compile`) and needs the `jco transpile` build
   step, which emits ~244 KB of generated JS plus four extracted core modules and a WASI shim
   package; only wasmtime-py — a wasmtime embedding, i.e. the same engine wasmtime-dotnet and a
   Rust host would use — loads the component directly. "Component model or core+shims?" from the
   plan resolves to: **component natively only in wasmtime embeddings, generated-shim path in JS,
   nothing elsewhere.**
2. **The frozen byte-pipe makes the core+shim fallback almost free.** The hand shim is ~30 lines of
   pointer math per host (`hosts/*/`), written once against a 3-function ABI that never grows —
   *because* the 1.1 document-first protocol needs exactly one operation. All four hosts ran the
   identical engine logic and identical frames through it. The component model's marshalling value
   shrinks to "moves a byte list", which is exactly when hand-shimming is trivial and safe. This
   closes 1.1's open risk: byte-frame-pipe ergonomics are fine — better than fine, they are what
   keeps non-component runtimes in play.
3. **At document scale the engine dominates, not the boundary — in every fast runtime.** Echo of
   100 KB costs ~1.3 ms on V8, wasmtime, and native alike (serde parse+serialize inside the guest
   is the floor); jco's component marshalling adds nothing measurable at that size (1.34 vs
   1.33 ms core), and small-call overhead everywhere is µs-scale, three orders below a typical
   test-runner operation. The marshalling-cost question from the plan is a non-issue *except*:
4. **wasmtime-py's dynamic component API is unusable for document frames** (as of v48): `list<u8>`
   is converted one `component_val_t` per byte through ctypes, both directions, even when passed
   Python `bytes` (`wasmtime/component/_types.py` has a bytes type-check but no bulk fast path) —
   75× slower than the same runtime through the classic core API (97.9 ms vs 1.32 ms). A
   quality-of-implementation gap, not an architectural one — but it means Python SDK hosting goes
   core+shim (or upstream gets a bytes fast path) regardless of the component decision.
5. **Interpreter backends are disqualified for document workloads.** Chicory's interpreter takes
   300–414 ms per 100 KB operation — two orders over its own bytecode-compiler backend (5–7 ms),
   which is entirely usable on a pure-JVM, zero-native-deps story (first-compile ~112 ms, then
   ~18 ms per fresh instance, µs-scale calls). Any "pure-language runtime" embedding must be held
   to compiled-backend numbers.
6. **Cold start is a non-problem for test runners.** Worst first-use observed: 112 ms (Chicory
   compiler JIT warm-up), one-off per JVM process; everything else is single-digit-to-tens of ms
   once per process, µs–tens-of-µs per fresh instance afterwards (wazero: 17 ms compile once,
   66 µs per instance).
7. **WASI leakage is the real component-hosting tax.** The engine does no I/O, yet its component
   form imports ten WASI 0.2 interfaces because Rust `std` wants them; every component host had to
   bring a WASI implementation (jco: `@bytecodealliance/preview2-shim`; wasmtime-py:
   `add_wasip2()` + `set_wasi(WasiConfig())` — and the naming asymmetry there cost debugging
   time). The zero-import core build is what made the pure-JVM and Go hosts trivial. The real
   engine should treat its import set as a controlled surface (and the kernel's "knows nothing
   about HTTP" rule helps: no sockets wanted anyway).
8. **RFC "WASM preferred" is confirmed, with a scope note.** The engine-in-WASM runs correctly and
   fast from all four languages — nothing here motivates the subprocess fallback *for these four*
   on capability grounds; 1.3 remains motivated by native-transport needs (real sockets for mock
   servers), not by hosting feasibility.

## 4. Recommendation into G1

- **Ship the engine as both artifacts from one build**: the WASM component (canonical form —
  wasmtime embeddings and jco consume it as-is) and the zero-import core module with the frozen
  3-function C-ABI (JVM/Go/anything-else path). They are the same crate compiled twice; the spike's
  `engine-toy/` layout is the template.
- **Embedding priority per language** (the G1 question this spike answers): Node — jco-transpiled
  component (works today, negligible overhead); JVM — Chicory bytecode-compiler + core shim
  (pure-JVM, no native libs, adequate numbers); Go — wazero + core shim; Python — wasmtime-py
  **classic API + core shim** until upstream grows a `list<u8>` fast path; .NET — expect the
  wasmtime story (validate when a toolchain is available).
- The per-host shim belongs in each thin SDK, is ~30 lines, and is frozen with the pipe — write it
  once per SDK, never again.

Open items this leaves for other tasks: subprocess leg and orphan/Windows behaviour (1.3), engine
*hosting* components (1.4), real marshalling/validation cost trend under load (1.7).
