# Benchmark baseline harness

Plan task 1.7 ([build], not a spike — this harness is durable and kept running). It measures
**today's stack** (pact_ffi, pinned exactly in `Cargo.toml`) at the same boundary the language
SDKs use. Plan task 9.1 added [`janus/`](janus/), which runs the same scenarios against Janus in
each of its embeddings, so the RFC's "performance envelope of WASM vs native FFI" question gets a
**trend line, not a one-off**. The plan meant that to start in Phase 4 and it did not: the first
Janus data points are 9.1's. The findings are in the
[performance report](../Documentation/performance-report.md).

## Running

```bash
cd benchmarks
cargo run --release
```

Prints a summary and writes `results/<date>-<stack>.json`. Commit the results file when a run is
meant to be a recorded data point. (The verifier scenarios also print pact-verifier output to
stdout; that is the verifier's own reporting, left enabled deliberately as a sanity check that
verification actually passed.)

This crate is excluded from the root workspace: it drags the full pact_ffi dependency tree,
which `cargo build` at the repo root should not pay for. `pact_models` is pinned in the lockfile
(pact_ffi 0.5.6 fails to compile against pact_models 1.3.14 — an enum grew; see spike 1.1 for
why this class of failure is a Janus design criterion).

## Scenarios (schema v1)

| Scenario | What it measures |
|---|---|
| `mock-server-startup-cycle` | create pact handle + start mock server + shut it down |
| `mock-server-latency-small` | matched POST, ~30 B body with type matchers, keep-alive, sequential |
| `mock-server-latency-100k` | matched POST, ~100 KB body under a root type matcher |
| `mock-server-latency-mismatch` | the mismatch path (wrong-typed fields → 500) |
| `mock-server-throughput-4threads` | sustained matched requests, 4 connections, 2 s |
| `verify-corpus-small` | verify 10 pacts × 5 interactions against a local provider stub |
| `verify-corpus-100k` | verify 5 interactions whose response bodies are ~100 KB |

Payloads are generated deterministically in-process (same order-document shape as spike 1.2).
The HTTP client and provider stub are hand-rolled with keep-alive and `TCP_NODELAY`, and send
each message in a single write — split writes interact with Nagle/delayed-ACK and inject a flat
~40 ms artifact that swamps everything (found the hard way; see git history).

## Comparing stacks

Result documents share one schema: `{schema, date, stack, os, arch, scenarios{...}}`. A Janus
run uses a different `stack` string and lands next to the baseline in `results/`. Keep scenario
names and workloads stable; add new scenarios rather than changing existing ones, so old data
points stay comparable.

## The Janus harness (`janus/`, plan task 9.1)

```bash
cd benchmarks/janus
cargo run --release                        # every embedding
cargo run --release -- native wasm         # some of them
cargo run --release -- probe               # one mock exchange's steps, as the request grows
```

It builds `janus-engine` and the engine component first. `JANUS_ENGINE` names an existing
`janus-engine` to use instead. `JANUS_BENCH_LOG=debug` passes the engine's own tracing through to
stderr, which is how you find out why a scenario's assertion failed. It writes one result document
per embedding, `results/<date>-janus-<embedding>-<git describe>.json`. The schema is the baseline's.

It is its own crate and its own workspace, because the two stacks cannot share a lockfile:
pact_ffi 0.5.6 needs an older `pact_models` than the kernel. The payloads, HTTP client, provider
stub and results schema are this directory's `src/util.rs`, compiled into both, so both run the
same workloads byte for byte.

| Embedding | What it is | Runs |
|---|---|---|
| `native` | `Engine::dispatch` in the harness's own process, wired as the `janus` CLI wires it | everything |
| `subprocess` | `janus-engine` over its stdio framing, as both SDKs embed it | everything except the state-hook variant scenario (`janus-engine` registers no hook implementations) |
| `wasm` | the engine component in `janus/engine-wasm/`, under wasmtime 47 | only what needs no I/O: no sockets, no threads |

`janus/engine-wasm/` is ADR 0003's canonical artifact, the engine as a WASM component. Nothing
built one before 9.1. It is the kernel and the JSON content component behind the frozen pipe, plus a
benchmark-only `bench` interface that runs a compiled plan. Offline matching is not a protocol
operation, so without that interface the interpreter could not be measured inside WASM at all.

Janus scenarios keep the baseline's names where the workload is the baseline's. Where Janus cannot
do the same thing, the scenario has a new name:

| Scenario | Janus difference |
|---|---|
| `mock-server-*` | each request is a `serve-variant` frame and then the request, because an arming answers one request. `arm_median_us` is the frame's share |
| `mock-server-latency-100k` | the request shape is the document's structure, typed positionally, because `each-like` pins a request to its cardinality points |
| `mock-server-throughput-4engines` | replaces `-4threads`. One mock serves strictly in sequence, so the parallel shape Janus has is four engines |
| `verify-corpus-*` | the same v3 pacts, read from disk and sent inline, the only source kind the engine reads |
| `verify-corpus-small-upgraded` | the same corpus after `upgrade/pact`, so it takes the contract path |
| `variants-consumer-rfc-order` | the RFC order payload, all selected variants against the base variant only, per fresh session |
| `variants-provider-order-service` | a state-bound contract against `samples/order-service`, all variants against one (native only) |
| `cold-start` | engine start plus handshake. For WASM also `compile_median_ms`, the uncached compile |
| `explain-*`, `upgrade-*`, `variant-space-*` | offline protocol operations, in all three embeddings |
| `match-*` | the interpreter against captured values: native calls the kernel, WASM calls the `bench` export |
| `unsupported` | WASM only: what the engine answers `start-transport` and `verify` with |
