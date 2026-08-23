# Benchmark baseline harness

Plan task 1.7 ([build], not a spike — this harness is durable and kept running). It measures
**today's stack** (pact_ffi, pinned exactly in `Cargo.toml`) at the same boundary the language
SDKs use, and from Phase 4 on the same scenarios run against Janus, so the RFC's "performance
envelope of WASM vs native FFI" question gets a **trend line, not a one-off**.

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
