# Plan task 9.1: the performance report

**Date:** 2026-09-24. **Subject:** Janus at `fe96f7c` plus this task's changes, in its three
embeddings, against the 1.7 baseline (pact_ffi 0.5.6). **Question:** the RFC's unresolved "performance
envelope of WASM vs native FFI", and the four numbers plan §12 asks for: mock throughput,
verification wall-time, variant-matrix overhead, and cold start per test file.

The short answer: **performance does not decide the embedding. Capability does.** Janus is faster
than pact_ffi on every scenario it can run the same way, except large request bodies, where a
per-arming recompile makes it about twice as slow. The subprocess costs almost nothing over
in-process. The WASM component runs the kernel's own work within 10–35% of native. But the WASM
embedding cannot run a mock server or a verification at all, so for the two things a user does most
it is not a slower option. It is no option. The measurements also found a quadratic in the kernel
(fixed in this task) and a fixed 200 ms per consumer session in the HTTP transport (not fixed).

## 1. What was measured

| | |
|---|---|
| Machine | Intel i9-12900K (24 threads), 91 GB, Linux, rustc 1.97.1, release builds. One machine, one day. The harness never ran on anything else, so treat the ratios as the result, not the absolute times |
| Baseline | `benchmarks/` (plan task 1.7), re-run today on the same machine: `results/2026-09-24-pact_ffi-0.5.6.json` |
| Janus | `benchmarks/janus/`, new in this task. It runs the baseline's scenarios through a protocol client over three pipes: **native** (`Engine::dispatch` in process, wired as the `janus` CLI wires it), **subprocess** (`janus-engine` over its stdio framing, as both SDKs embed it), **wasm** (the engine component in `benchmarks/janus/engine-wasm/`, under wasmtime 47.0.4) |
| Workloads | The baseline's own, from its `src/util.rs`, compiled into both harnesses: the same request bodies, the same deterministic 100 KB order document, and v3 pacts shaped as pact_ffi writes them. Where Janus cannot do the same thing, the scenario is renamed and the README says why |
| Reproduce | `cd benchmarks && cargo run --release`, then `cd janus && cargo run --release`. Results land in `benchmarks/results/` |

Nothing built the WASM artifact before this task. ADR 0003 makes it canonical ("one engine build
produces three artifacts") and only the subprocess one existed. `engine-wasm/` is the smallest honest
version: the kernel and the JSON content component behind the frozen pipe, and no transports, because
a `wasm32-wasip2` guest has no threads for the exchange loop and the HTTP server, and the HTTP
transport was not built for the target (Phase 9 finding 3). *Corrected in task 9.4:* this sentence
first said the guest has no sockets; `wasi:sockets` gives it them. It lives in `benchmarks/`, not `engine/`, because this task measures it. Whether it
graduates is a 9.4 question (§6).

The plan meant this harness to run against Janus "from Phase 4 on", so that the question would get a
trend line. It did not. Nothing ran it after 1.7, and CI never did. These are the first Janus data
points. Their first trend line is the fix in §5.1, a before and after.

## 2. The numbers

Medians. µs unless marked. Janus's mock rows include the `serve-variant` frame that arms each request.
In brackets is its share (`arm_median_us`).

| Scenario | pact_ffi | Janus native | Janus subprocess | Janus wasm |
|---|---:|---:|---:|---:|
| `mock-server-startup-cycle` | 40.2 | 151 (p95 343) | 313 (p95 **200,733**) | — |
| `mock-server-latency-small` | 138.7 | 33.2 [13.3] | 34.5 [14.6] | — |
| `mock-server-latency-100k` | 11,801 | 22,967 [16,755] | 23,145 [16,930] | — |
| `mock-server-latency-mismatch` | 140.4 | 40.4 [18.4] | 37.6 [15.3] | — |
| throughput, rps | 8,458 (1 mock × 4 connections) | 83,853 (4 engines) | 81,720 (4 engines) | — |
| `verify-corpus-small` (50 interactions), ms | 427.5 | 13.7 | 14.4 | — |
| `verify-corpus-100k` (5 interactions), ms | 422.1 | 92.7 | 94.4 | — |
| `verify-corpus-small-upgraded`, ms | — | 19.3 | 19.3 | — |
| `cold-start` (engine + handshake) | — | 6.2 | 1,454 | 40.6 + 302 ms compile |
| `explain-pact-small` | — | 294 | 276 | 323 |
| `explain-pact-100k` | — | 21,895 | 22,661 | 24,260 |
| `upgrade-pact-100k` | — | 20,923 | 22,168 | 24,373 |
| `variant-space-rfc-order` (8 variants) | — | 191 | 187 | 256 |
| `match-small` (interpreter only) | — | 12.0 | n/a | 14.1 |
| `match-100k` (interpreter only) | — | 10,670 | n/a | 9,661 |

| Variant matrix | native | subprocess |
|---|---:|---:|
| consumer, RFC order payload: 8 selected variants of 12, one session, ms | 202.1 | 202.3 |
| … the same session serving only the base variant, ms | 201.0 | 201.3 |
| … of which `finalise`, ms | **200.4** | **200.5** |
| … per extra variant, ms | 0.155 | 0.149 |
| provider, `samples/order-service` with v3 state hooks: 6 variants, ms | 2.42 | — |
| … one variant, ms | 1.11 | — |
| … per extra variant (state setup + request + teardown), ms | 0.262 | — |

The baseline moves between runs. Verification took 324 ms and 274 ms on 2026-08-23, 427 ms and 422 ms
in the recorded run today, and 470 ms and 369 ms in a stray re-run an hour later. That re-run also
put the startup cycle at 73 µs against 40, while the other mock rows stayed within 5%. The recorded
file is the first run, and the ratios below use it, because it ran back to back with the Janus
stacks. Read the pact_ffi verifier rows as ±20%. None of the conclusions depend on a gap that small.

## 3. The four questions

**Mock throughput.** A Janus mock answers a small matched request in 33 µs against pact_ffi's 139,
mismatches in 38–40 µs against 140, and 13–15 µs of that is the frame that arms the request. The
throughput rows are not the same experiment. Janus's mock cannot serve concurrent requests. One
transport instance holds one armed exchange, and each arrival consumes it
(`engine/kernel/src/protocol/exchange.rs`). So the baseline's "one mock, four connections" has no
Janus form. The parallelism Janus has is four engines, as four test files would run, and those serve
about 82–84k requests a second. The 10× ratio against 8.5k compares different shapes. The honest
single-mock figure is about 29k requests a second, strictly sequential. That is well above what a
consumer test needs.

Large bodies are the exception. At 100 KB Janus takes 23 ms against 11.8. The request itself is about
6 ms, half pact_ffi's. The other 17 ms is `serve-variant` regenerating the interaction and recompiling
its plan on every arming (§5.3). Before this task's fix it was far worse (§5.1).

**Verification wall-time.** Janus verifies the same v3 pacts against the same stub about 30× faster
for small bodies (0.27 ms against 8.6 ms per interaction) and 4.5× faster at 100 KB. Two caveats keep
this from being a headline. pact_ffi's verifier prints a report per interaction and runs its own async
runtime, which Janus does not. And Janus does less: no pact-broker fetch, no pending pacts, no
publishing. The contract path is 40% slower than the pact path on the same corpus (19.3 ms against
13.7 ms). That is worth a look before anyone reads "upgrade" as free, and it is not explained here.

**Variant-matrix overhead.** Small. The RFC's order payload samples 8 of its 12 points (pairwise, ADR
0008). Each extra variant costs a consumer test 0.15 ms and a verification 0.26 ms, including the two
state-hook HTTP calls. The variant model's cost is not in the variants. On the consumer side it is the
fixed 200 ms that `finalise` takes (§5.2), which is 99% of that session's time. On the provider side
it is what a real provider's state setup costs, which the sample's in-memory store makes negligible.
The RFC's worry that variants multiply test time is real only in proportion to a provider's state
setup. The engine adds almost nothing.

**Cold start per test file.** The subprocess starts and handshakes in 1.45 ms. In-process is 6 µs,
and the pact_ffi startup cycle is 40 µs. The WASM component takes 302 ms to compile uncached, then
41 µs per instance. A test runner that starts a worker per file pays the compile once per worker
unless the host caches compiled code (wasmtime's cache, a precompiled `.cwasm`, V8's code cache
under jco). The subprocess is cheaper per file than an uncached WASM compile by two orders of
magnitude. The component itself is 6.9 MB built, 5.7 MB stripped, 1.6 MB gzipped. Spike 1.2's toy was
107 KB, and it imports 18 WASI interfaces where the toy imported 10.

## 4. WASM against native: the envelope

On the kernel's own work, through the same protocol frames, WASM costs 10% (explain), 16% (upgrade),
18% (small match) and 34% (variant enumeration) over native, and nothing at document scale. The 100 KB
match is 9% *faster* in WASM, most likely the allocator: the guest's `dlmalloc` against glibc's
`malloc` on an allocation-heavy walk. The subprocess sits within 6% of native on every offline row,
and is within noise of it on the mock and verification rows. The pipe costs 1–2 µs a frame, far below
anything a frame does. Spike 1.2 found that at document scale "the engine dominates, not the
boundary". The real engine confirms it for both boundaries.

So the envelope, as a number: **WASM ≈ native + 10–35% on compute, + 300 ms per uncached process
start, with nothing to cross at the boundary.** Nothing there would reopen ADR 0003's per-language
table, and its tripwire ("WASM marshalling dominating real workloads") has not fired.

What reopens it is capability. The WASM engine answers `consumer-session/start-transport` and
`verification/verify` with `component-unavailable` (recorded in its results as `unsupported`),
because there is no HTTP transport it could load. ADR 0003 makes the WASM component primary for Node,
the JVM, Python and Go. For all four, the component can compile, explain, enumerate variants and
upgrade (the operations measured here), and it cannot run a consumer test or a verification. The fallback those rows
name is the only embedding that does the job. Finding 3 already said this for Node's mock server.
This task adds that it holds for verification too, and for every language, and that the zero-import
core module those rows rely on for the JVM and Go cannot be built at all (§5.5).

## 5. What the measurements turned up

### 5.1 `navigate` was quadratic — fixed

Writing the 100 KB request as a shape made each mock request take about 3 s, and the harness looked
hung. Probing one exchange as the request grew:

| request | 1 KiB | 2 KiB | 4 KiB | 8 KiB | 16 KiB |
|---|---:|---:|---:|---:|---:|
| before, ms | 0.9 | 2.8 | 6.9 | 22.1 | 80.0 |
| after, ms | 0.5 | 0.6 | 0.9 | 1.6 | 3.7 |

`plan::navigate` cloned the value at every step of a path. Resolving
`$.request.body.orders[3].lines[0].sku` copied the whole `orders` array first, so a plan that resolves
every leaf by absolute path paid for the document once per leaf. It now walks by reference and clones
only what it lands on (`engine/kernel/src/plan/value.rs`). Behaviour is unchanged: all 25 kernel test
suites pass, and the golden corpus is 21/21. This harness's v3 pact plans barely moved (WASM
`match-100k`: 11.0 ms before, 9.7 ms after), so the cost fell on shapes that address deep leaves by
absolute path.
`cargo run --release -- probe` in `benchmarks/janus` reproduces the table (Phase 9 finding 26).

### 5.2 Every consumer session costs 200 ms in `finalise` — fixed in task 9.2

*Task 9.2:* fixed as proposed below — the poll waits outside the lock and `stop` unblocks it. `finalise`
is now 0.18 ms native and 0.19 ms subprocess, and the subprocess startup cycle's p95 is 221 µs
(`results/2026-09-24-janus-*-0fc5d7c-dirty.json`; Phase 9 finding 27). The text below is 9.1's.


`finalise` stops the transport, and the HTTP transport's `stop` waits for the instance lock, which
the exchange loop holds through each `poll-inbound`, up to its 200 ms timeout
(`engine/component-http/src/lib.rs`, `poll_inbound`). Median `finalise`: 200.4 ms, in every
embedding. The startup cycle escapes it only when the loop has not reached its first poll yet, and
its p95 of 200.7 ms is the same effect. That lock is also one mutex for **every** instance of the
transport. Reading the code, two mocks in one engine would hold up each other's replies for as long
as the other one is polling. That was not measured: every scenario here gives each mock its own
engine. The fix belongs to the transport: wait for arrivals outside the lock, and let `stop` wake
the waiter. It is not made here, because it changes the component's concurrency, not a clone
(Phase 9 finding 27).

### 5.3 Arming a variant recompiles its plan — not fixed

`serve-variant` regenerates the interaction and compiles the variant's plan every time
(`ConsumerSession::serve_variant`), so arming costs in proportion to the shape: 13 µs for the small
request, 16.8 ms for the 100 KB one. A test that repeats a variant (a retrying client, or a loop over
a page of requests) pays it every time. Caching the compiled plan and generated parts per
`(handle, variant)` removes it. The selection is already fixed when `variants` returns (Phase 9
finding 28).

### 5.4 An `each-like` in a request pins the request

The baseline's 100 KB request is one root `type` matcher, cascading to every leaf. The shape language
has no cascade. Its one every-element operator, `each-like`, contributes a cardinality dimension, and
variant-semantics §4.1 applies a dimension in the request to "what the mock will accept". The served
`min` variant accepted exactly one order, and no variant admitted the 310-order document. The harness
uses the nearest spelling that contributes no dimension: the document's structure, typed positionally.
That is a workload, not a way to write a consumer test. A consumer whose requests carry a
data-dependent list has no shape for "every element is typed" that the mock will take (Phase 9
finding 25).

### 5.5 ADR 0003's WASM artifacts

- **The component** now exists, for this task. It has 18 WASI imports it never deliberately uses,
  where the toy had 10. Which dependency adds which was not traced.
- **The zero-import core module** cannot be built. For `wasm32-unknown-unknown`, `pact_models` pulls
  in `rand` → `getrandom`, which refuses the target. Past that, `rquickjs` (ADR 0015) compiles
  QuickJS's C sources, which need a libc that target does not have. ADR 0003 decision 4, "the kernel
  keeps a zero-import discipline", was broken by ADR 0015 and by the `pact_models` dependency, and no
  build checked it. The JVM (Chicory) and Go (wazero) primaries depend on it. Both runtimes can load a
  `wasm32-wasip1` core module with a WASI shim instead, which is a different artifact from the one the
  ADR chose, and one this task did not build.
- **What the component can do** is everything without I/O (§4) (Phase 9 finding 29).

### 5.6 Smaller gaps

- `janus-engine` registers no hook implementations (`cli/src/bin/janus_engine.rs`). So a verification
  whose states come from an `http` or `exec` hook runs only in the `janus` CLI. The provider variant
  scenario is native-only for that reason. An SDK driving provider verification through the subprocess
  would hit the same wall (Phase 9 finding 30).
- Matching captured values offline is not a protocol operation. `janus explain --executed` runs the
  plan beside the protocol, through the kernel's Rust API, so no SDK or WASM host can do it. The
  harness needed a benchmark-only `bench` export to measure the interpreter in WASM at all
  (Phase 9 finding 31).

## 6. For 9.2 and 9.4

- RFC "performance envelope WASM vs native FFI": answered by §4. The envelope is small. The
  capability gap is not, and it is the thing to write down.
- ADR 0003: its per-language table assumes a WASM engine that can run a consumer test. None can,
  today, in any language. That and §5.5's core module are grounds for a superseding ADR in 9.2: either
  subprocess-primary, with WASM for offline operations, or a funded plan for a transport the host
  provides (finding 3's option b).
- 9.4 should carry: the transport lock (§5.2) and the per-arming recompile (§5.3) as known
  performance debt; the benchmark harness in CI as a trend, not a gate, which plan §14 already
  promised; and `engine-wasm/` either graduating into `engine/` with a CI build or being declared a
  measurement artifact only.
