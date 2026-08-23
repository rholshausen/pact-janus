# Spike 1.6 findings — Script-hook engine bake-off

Status: **complete** — five engines benchmarked natively, three inside wasmtime, one definitive
build failure, one maturity elimination; all engines that ran produced **byte-identical hook
output** (the cross-engine correctness check held). Method: [README.md](README.md). Feeds the
scripted-hook ADR in design 2.7.

## 1. The matrix

Workload: sign a request (FNV-1a over method/path/body/secret → `authorization` header) and
mutate headers — including hook-context marshalling on every call. Same machine as 1.2–1.4.

| Engine | Language | Warm call, native | Cold start, native | Warm call **inside wasmtime** | Dist. size (native / wasm) | Ambient APIs by default |
|---|---|---|---|---|---|---|
| **QuickJS** (rquickjs 0.9, bundled C) | JS | 8.1 µs | 0.17 ms | **12.9 µs** | 1.5 MB / **1.1 MB** | none (no require/os/std) |
| Lua 5.4 (mlua 0.10, vendored C) | Lua | **3.6 µs** | **0.03 ms** | **BUILD FAILS** — lua-src: "don't know how to build Lua for wasm32-wasip1" | 0.9 MB / — | **io + os present**; restrictable via `StdLib` flags |
| Rhai 1.x (pure Rust) | Rhai DSL | 20.7 µs | 0.15 ms | 34.8 µs | 3.1 MB / 2.5 MB | none, by construction |
| Boa 0.20 (pure Rust) | JS | 23.8 µs | 0.16 ms | 41.7 µs | 7.8 MB / 5.5 MB | none (no require/process/fetch) |
| V8 (rusty_v8 130) | JS | **1.2 µs** | 0.75 ms | **impossible** (native embedding only) | **29.5 MB** / — | none in bare V8 |
| piccolo | Lua | — | — | — | — | 0.3.3, last release 2024-06, pre-1.0 — **eliminated on maturity** |

In-wasmtime runs are the whole runner compiled to `wasm32-wasip1` — the interpreter living
inside the WASM-embedded engine, exactly the architectural constraint the plan flagged.

## 2. Findings

1. **QuickJS is the only candidate that wins on the deciding criterion and the user criterion at
   once.** It is JS (what users ask for), it compiled to `wasm32-wasip1` out of the box via
   rquickjs's bundled build and ran the workload inside wasmtime at 12.9 µs/call, it is the
   *smallest* artifact of any JS option (1.1 MB of wasm), its default embedding exposes no
   ambient capabilities, and cold start is 0.2 ms. The Javy toolchain (Bytecode Alliance)
   independently corroborates QuickJS-in-WASI as a production path.
2. **Performance does not decide this bake-off — embedding compatibility does.** The *slowest*
   result in the whole matrix (Boa inside wasmtime, 41.7 µs) is still ~3 % of matching one
   100 KB document (1.2). Hooks run a handful of times per test; every engine here is fast
   enough. V8's 6.7× warm-call advantage over QuickJS is therefore worth nothing against its
   costs: 29.5 MB of distribution and — decisively — native-only, which would fork scripted
   hooks into a "subprocess embedding only" feature. **Rejected.**
3. **The Lua path dies on the constraint the plan predicted, not on speed.** mlua is the
   fastest embeddable interpreter measured (3.6 µs, 0.03 ms cold, 0.9 MB) — and its build
   script simply refuses the WASI target, so it cannot ride inside the WASM-embedded engine.
   Additionally it is the only engine whose *default* configuration leaks capabilities (`io`,
   `os` — the restricted `StdLib` subset fixes it, but safe-by-default matters for hook code).
   piccolo, the pure-Rust rescue, is two years stale at 0.3.x. With user preference already
   against Lua, **rejected**.
4. **Boa is the credible pure-Rust fallback.** It ran the full workload (including
   `Math.imul`, `padStart`, `JSON` bridging) correctly everywhere the engine compiles, with
   zero C-toolchain involvement — the *simplest* build story of the JS options. Costs: ~3×
   QuickJS per call, ~5× its size, and historically incomplete spec coverage (not hit here,
   but hook scripts are small and conservative). **Keep as the designated fallback** if the
   C-to-WASI build of QuickJS ever becomes a maintenance burden inside the engine's own
   wasm32-wasip2 component build (rquickjs compiled for wasip1 here; the engine's component
   build must be verified in 2.7 — the one open item).
5. **Rhai did its control-group job**: mid-pack performance, clean sandbox, pure Rust — and it
   would still be the wrong answer, because it is a new language for users whose stated
   preference is JS/TS. A Rust-native DSL only wins if the JS options had failed the embedding
   test; they didn't.
6. **Sandboxing is a solved problem in every surviving candidate** — bare interpreters expose
   no I/O unless the embedder binds it, which inverts the risk: the 2.7 design must specify
   what the *hook context API* exposes (request parts, config, logging), because that API will
   be the entire capability surface. One gap needs explicit design: **runaway scripts**. None
   of these interpreters interrupts by default; rquickjs supports an interrupt handler (the
   analogue of 1.4's epoch deadline) and 2.7 should mandate one around every hook invocation.
   (When the engine itself runs as a WASM component, the host's epoch deadline already bounds
   it from outside — 1.4.)
7. **TypeScript stays a tooling question, as the plan suspected.** Every surviving engine
   executes plain JS; TS support = transpile at spec-load time (esbuild/swc in the SDK/CLI
   toolchain, or a precompile step) with the engine never knowing. No engine choice is
   affected. StarlingMonkey/ComponentizeJS was assessed on paper: it is a script *compiled
   into* a component ahead of time — that is the "bring your own precompiled WASM component"
   escape hatch (which remains regardless), not an embedded scripting default.

## 3. Recommendation for the 2.7 scripted-hook ADR

**Default hook engine: QuickJS via rquickjs**, embedded in the engine and compiled with it in
all three embeddings. JS as the hook language; TS via load-time transpile in tooling. Mandatory
interrupt handler per invocation; hook capability surface defined entirely by the hook-context
API (no ambient I/O). **Fallback: Boa** (pure Rust) if the QuickJS C-to-WASI build proves
fragile inside the engine's component build — verify that build in 2.7 before the ADR is
accepted. The polyglot escape hatch stays "bring your own WASM component" (1.4's plugin path).
