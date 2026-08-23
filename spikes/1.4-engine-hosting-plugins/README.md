# Spike 1.4 — Engine hosting WASM components as plugins

Plan task 1.4 · Feeds gate G1 (and Phase 2 component-interface design) · Findings in
[FINDINGS.md](FINDINGS.md) (the durable artifact — code here is disposable).

## Question

The inverse of 1.2: can a **native Rust engine** (wasmtime) load a WASM component that contributes
a custom matcher action, and invoke it from a matching plan — with the properties B3 actually
needs from third-party code in-process: capability discovery, **sandboxing** (no ambient
filesystem/env/network), **fault containment** (a panicking plugin must not take the engine down),
**runaway protection** (an infinite-looping plugin must not hang a verification), and acceptable
per-invocation overhead on the hot path (matchers run per plan node)?

Per the 1.1 draft G1 recommendation, the plugin interface is the **frozen byte-pipe**
(`call: func(request: list<u8>) -> list<u8>` carrying JSON frames) — so this spike also rehearses
the exact plugin-surface shape Phase 2 would specify.

## Method

1. **`wit/plugin.wit`** — the frozen pipe world, shared by both plugins.
2. **`plugins/matcher`** — a well-behaved plugin: handshake advertises contributed actions
   (`match:luhn`, `echo`); implements a Luhn-checksum matcher (something the core engine does not
   have) returning mismatches as values.
3. **`plugins/misbehaved`** — a hostile-ish plugin with on-demand misbehaviour: `read-file`
   (tries `/etc/passwd` and its own cwd), `get-env` (tries to read the environment), `panic`
   (Rust panic → wasm trap), `spin` (infinite loop). Both plugins are built with cargo-component
   the way a third party would (std → WASI 0.2 imports — the realistic case from 1.2 finding 7).
4. **`engine-host`** — native Rust + wasmtime 35 + wasmtime-wasi (locked-down context: no
   preopens, no env, no sockets), epoch interruption enabled. Scenarios:
   - discovery handshake; **plan execution** mixing a core action (`core:match-type`, the 1.2 toy
     matcher inlined) with plugin actions, producing a per-node executed-plan result;
   - sandbox probes (read-file / get-env from inside the plugin);
   - panic containment: trap surfaces as a host error-value; state of the instance after the
     trap; recovery by re-instantiation;
   - spin + epoch deadline: call terminated by the engine, wall-time measured;
   - bench: per-invocation cost of a plugin matcher vs the same matcher native in the host;
     100 KB document through the pipe; instantiation cost (cold vs `InstancePre`).

## Layout

```
wit/plugin.wit        Frozen byte-pipe plugin world (JSON frames)
plugins/matcher       Luhn matcher plugin (well-behaved)
plugins/misbehaved    Sandbox/fault/runaway probe plugin
engine-host           Native wasmtime host: plan executor, scenarios, bench
FINDINGS.md           Results and G1/Phase-2 implications
```
