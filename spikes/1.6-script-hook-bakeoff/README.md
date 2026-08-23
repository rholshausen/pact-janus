# Spike 1.6 — Script-hook engine bake-off

Plan task 1.6 · Feeds design 2.7 (scripted-hook ADR), not G1 · Findings in
[FINDINGS.md](FINDINGS.md) (the durable artifact — code here is disposable).

## Question

Hooks need a low-friction scripting default (the RFC's `wasm-script` sketch); prior-art feedback
says users want JS/TS, not Lua. The architectural constraint that decides the choice: **the script
runtime must work in all three engine embeddings** — if the engine ships as a WASM component, a
natively-embedded V8/SpiderMonkey cannot live inside it. Candidates: Lua (mlua / piccolo),
JS-in-WASM (QuickJS / StarlingMonkey), pure-Rust JS (Boa), native V8 as the ceiling, Rust-native
DSLs (Rhai) as control group.

## Method

One realistic hook workload, implemented identically in each engine's language: **sign a request**
(FNV-1a 32 over `method + path + body + secret`, hex-encoded into an `authorization` header) and
**mutate headers** (add `x-signed-at`, preserve existing entries). Every engine must produce
byte-identical output (cross-engine correctness check, asserted).

`runner/` is one Rust crate with feature-gated engines (`rhai-engine`, `boa`, `lua` = mlua 5.4
vendored, `quickjs` = rquickjs bundled, `v8-engine` = rusty_v8). Per engine:

- **cold start**: fresh engine + compile script + one call (×20, median);
- **warm per-invocation**: hook call including hook-context marshalling, each engine's idiomatic
  bridge (serde for Rhai/mlua, `JsValue::from_json` for Boa, JSON-string bridge for
  QuickJS/V8) — marshalling is part of the cost being measured (×2000);
- **sandbox probe**: what ambient APIs exist by default;
- **the embedding test**: build the same runner for `wasm32-wasip1` with only that engine and run
  it *inside wasmtime* — the interpreter-inside-the-WASM-engine question answered by
  construction. A compile failure is a recorded result.

piccolo is assessed as a maturity check (crates.io state), not benchmarked. TS support is a
transpile-tooling question (esbuild/swc), assessed on paper. StarlingMonkey/ComponentizeJS
assessed on paper (component-native but a different integration shape — a script compiled *into*
a component ahead of time, i.e. the "bring your own component" escape hatch, not an embedded
interpreter).

## Layout

```
runner/       One crate, feature-gated engines, same workload everywhere;
              native + wasm32-wasip1 builds
FINDINGS.md   The matrix, the numbers, and the 2.7 recommendation
```
