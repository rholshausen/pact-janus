# Spike 1.4 findings — Engine hosting WASM components as plugins

Status: **complete** — all scenarios pass: discovery, plan execution mixing kernel and plugin
actions, sandbox probes, panic containment, epoch-based runaway termination, benchmarks.
Method: [README.md](README.md). Toolchain: wasmtime 35 + wasmtime-wasi 35 (native host),
cargo-component 0.21 (plugins).

## 1. Setup

Two plugins export the **frozen byte-pipe** world (`pact:plugin/pipe`, `call: list<u8> ->
list<u8>` carrying JSON frames — the 1.1 draft G1 shape, now exercised on the plugin surface):

- `plugin-matcher` (106 KB): handshake advertises `actions: ["match:luhn"]`; implements a
  Luhn-checksum matcher the kernel doesn't have, mismatches as values.
- `plugin-misbehaved` (123 KB): misbehaves on demand — `read-file`, `get-env`, `panic`, `spin`.

Both are built with std via cargo-component the way a third party would (so they carry the WASI
0.2 imports from 1.2 finding 7). The native host runs a toy plan executor: `core:*` actions
execute in the kernel (the 1.2 type matcher, inlined), `plugin:*` actions dispatch by the
action registry built from handshakes.

## 2. Results

**Plan execution.** A four-node plan (kernel type-match, plugin Luhn pass, plugin Luhn fail,
unmapped plugin action) produces a single executed-plan document with per-node results, kernel
and plugin mismatches in the same shape, each node labeled with its executor, and the unmapped
action reported as a structured `no-plugin-for-action` error. The RFC's "plugins contribute
actions invoked from plans" story works end-to-end over the byte-pipe.

**Sandboxing.** With a default `WasiCtxBuilder` (no preopens, no env, no sockets; only stderr
granted deliberately): `/etc/passwd` and `./Cargo.toml` reads fail *inside* the plugin (os error
44 — with no preopens there is no filesystem to resolve any path against) and come back as the
plugin's own `io-denied` error frames; `std::env::vars()` sees zero variables. Deny-by-default is
real and needs no engine code — capability grants would be explicit `WasiCtxBuilder` calls.

**Panic containment.** A plugin panic becomes a wasm trap: the host's `call` returns `Err` (an
error value at the boundary — no unwinding into the engine, the panic message visible on the
granted stderr). The trapped instance is then *poisoned, loudly*: a further call fails
immediately with `cannot enter component instance` rather than running on corrupt state.
Re-instantiation from the already-compiled component (fresh store + handshake) takes **0.1 ms**,
so "recycle the instance on trap" is a free policy.

**Runaway termination.** With epoch interruption (10 ms ticker thread) and a 20-tick deadline, an
infinite-loop plugin call is terminated by the engine after **200 ms** — exactly the deadline —
surfacing as the same kind of host error. A hung plugin cannot hang a verification. (Amusingly,
the first "infinite" loop had to be `black_box`ed: LLVM folded a bounded loop to its closed form
and returned instantly. Adversarial-plugin tests must be written against the optimizer.)

**Cost** (medians, same machine as 1.2/1.3):

| Operation | Cost |
|---|---|
| Component compile (`Component::from_file`, 106 KB plugin) | 11.3 ms, once |
| Instantiate + WASI wiring (`Linker::instantiate`) | 8.6 µs |
| Instantiate via `InstancePre` | 5.8 µs |
| Luhn match natively in the host (baseline) | ~ns (below timer resolution) |
| Luhn match via plugin (JSON frame both ways, component call) | 1.4 µs |
| 100 KB document through the plugin pipe (serialize + parse both sides) | 2.6 ms |

## 3. Findings

1. **B3 works today with wasmtime as the host.** Discovery, plan-integrated invocation,
   deny-by-default sandboxing, loud fault containment, and bounded execution are all achievable
   with stock wasmtime features (component model + locked WASI ctx + epoch interruption) and no
   custom machinery. This is the inverse-direction confirmation 1.2 gave for engine-as-guest.
2. **The frozen byte-pipe carries the plugin surface without friction.** Handshake-based action
   discovery replaces typed interface negotiation — precisely the capability-negotiation pattern
   the 1.1 gauntlet showed WIT cannot express for growing vocabularies. Nothing in this spike
   wanted a richer WIT surface; the JSON frames did everything including structured errors.
3. **Hot-path cost is real but small: ~1.4 µs per plugin matcher call** (~700 K calls/s), versus
   nanoseconds native — the gap is JSON framing + the component call, not computation. Budget
   guidance for Phase 2: a plan node calling a plugin per *value* is fine at test scale; per-node
   batching (send all values for a node in one frame) is the obvious lever if 9.1-scale
   benchmarks ever show pressure. At document scale (100 KB, 2.6 ms) the cost is double-ended serde,
   same story as 1.2 — the pipe itself is not the bottleneck.
4. **Instance-per-call isolation is affordable** at 5.8 µs (`InstancePre`): a paranoid engine
   could give every plan node a fresh plugin instance for ~0.4 % of the cost of matching one
   100 KB document. Recommended default: instance per session (state isolation between test
   sessions for free), recycle on trap — both now measured as effectively free.
5. **Traps poison loudly, and that is the right shape**: the engine cannot accidentally keep
   using a half-dead plugin; recovery is explicit, cheap (0.1 ms), and local to the plugin. Maps
   directly onto the "errors are values at the engine boundary" rule — the host turns the trap
   into a structured plugin-failure result for the plan node.
6. **Epoch deadlines are the right runaway mechanism**: one ticker thread per engine, a per-call
   deadline, ~10 ms granularity, zero per-call overhead observed. The engine should set a
   deadline around *every* plugin call as policy (this spike's `NORMAL_DEADLINE_TICKS` ≈ 10 s
   for well-behaved calls, tighter for known-cheap ops).
7. **WASI leakage (1.2 finding 7) reappears on this side as attack surface, contained by
   defaults**: third-party plugins built with std demand ten WASI interfaces, but a default
   `WasiCtx` satisfies them with nothing — the imports exist, the capabilities don't. The
   Phase-2 component spec should still specify what a plugin may import (and the engine reject
   surplus), so the surface is governed rather than merely harmless.

## 4. Implications for G1 / Phase 2

- The plugin-facing half of the 1.1 draft recommendation is confirmed executable: frozen
  byte-pipe world + JSON frames + handshake capability discovery, hosted by native wasmtime with
  sandbox/fault/runaway containment as stock policy.
- Phase 2's component-interface spec (2.5/2.6) should specify: the handshake frame schema
  (name, protocol versions, contributed actions), the invoke/error frame shapes, the allowed
  import set for plugin components, instance lifecycle policy (per session; recycle on trap),
  and mandatory deadlines around plugin calls.
- One design note from the toy: the executor needed mutable access to one plugin from multiple
  plan nodes — fine sequentially, but concurrent plan execution will want either a store-per-call
  model (measured affordable) or per-plugin serialization; decide in 2.4/2.5 rather than
  inheriting whatever the prototype does.
