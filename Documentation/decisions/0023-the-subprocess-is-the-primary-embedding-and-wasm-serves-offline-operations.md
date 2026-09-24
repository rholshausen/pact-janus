# 0023 — Make the subprocess every SDK's primary embedding, and WASM the embedding for offline operations

- **Status**: accepted; context corrected 2026-09-24 (task 9.4), decision unchanged
- **Date**: 2026-09-24
- **Plan tasks**: 9.2 (supersedes [ADR 0003](0003-embedding-priority-per-language.md); feeds 9.4)
- **Evidence**: [performance report](../performance-report.md) §3–§5.5 (task 9.1),
  [Phase 9 findings](../phase-9-findings.md) 3 and 29, [ADR 0013](0013-component-hosting-is-an-embedding-capability.md)
  (a WASM guest hosts no components), [spike 1.2](../../spikes/1.2-wasm-embedding/FINDINGS.md) and
  [spike 1.3](../../spikes/1.3-subprocess-embedding/FINDINGS.md) (the evidence ADR 0003 stood on),
  `sdks/README.md` (both SDKs ship the subprocess only)

## Context

ADR 0003 made a WASM build of the engine the primary embedding for Node, the JVM, Python and Go, with
`janus-engine` as a first-class fallback. It rested on spike 1.2, which hosted a *toy* engine: a pipe,
not a mock server. Nine phases later the evidence runs the other way, and it is not about speed.

- **A WASM engine cannot run a consumer test or a verification, in any language — as built.** A consumer
  test needs a mock server, and the exchange loop that answers it runs on a thread of its own, as does
  the HTTP transport's server (`tiny_http`). A `wasm32-wasip2` guest has **no threads**. It does have
  sockets (`wasi:sockets`; Rust's `std::net` works on it under wasmtime), so the socket half of the
  problem is the prototype's, not the target's: its HTTP transport was never built for the target, and
  `wasi:http` on p2 offers an incoming-handler model rather than a server a test can point a client at.
  Verification needs an outbound client, which p2 could provide, and `exec` hooks, which need a process
  WASI does not offer. Task 9.1 built the canonical component for the first time and confirmed the
  result: `start-transport` and `verify` answer `component-unavailable` (finding 29). ADR 0013 had
  already established that a WASM guest hosts no out-of-tree components either.

  *Correction (task 9.4).* This bullet first said a p2 guest has neither sockets nor threads. It has
  sockets; the missing threads are the blocker. The decision stands on that alone for today, but it
  changes what would reopen it: see the tripwire.
- **The zero-import core module cannot be built.** ADR 0003's decisions 1 and 4 made it the JVM's and Go's
  primary. `wasm32-unknown-unknown` fails in `getrandom`, reached through `pact_models` → `rand`, and
  behind that `rquickjs` (ADR 0015) needs a libc for QuickJS's C sources. CI built only `wasm32-wasip2`,
  so nothing noticed for six phases.
- **Performance does not argue for WASM.** The component runs the kernel's own work within 10–35% of
  native; the subprocess costs almost nothing over in-process on the same scenarios (9.1 §2–§4). Neither
  number decides anything once the capability gap is on the table.
- **Both SDKs already ship the subprocess only.** The TypeScript SDK (6.2) and the JVM SDK (6.3) each built
  a frame-pipe interface a WASM embedding could implement later. Neither could have used one.

ADR 0003's per-language table therefore names, for four languages, a primary embedding that cannot do
the two things an SDK exists to do.

## Decision

1. **`janus-engine` is the primary embedding for every SDK language**, and the only one an SDK is required
   to ship. Its lifecycle is spike 1.3's, unchanged: spawned per test run by the SDK, pinned by protocol
   version in `engine/hello`, exit on stdin EOF so it cannot outlive its host. It is never a shared daemon.
2. **The WASM component is the embedding for offline operations**: everything that needs no thread, no
   process and no transport — `verification/explain`, `upgrade/pact`, `subsumption/check` and
   `subsumption/decide`, and enumerating an interaction's variants (`consumer-session/variants` on a session
that starts no transport) — the operations 9.1 measured in all three embeddings. It is for hosts that
   want Janus's answers without a native binary: a broker, a browser-based contract viewer, an IDE
   extension, a CI step in a sandbox. It is the same kernel built as `wasm32-wasip2` — which CI already
   builds — and answers `component-unavailable` for any operation that needs a transport, as it does
   today. Whether 9.1's `benchmarks/janus/engine-wasm/` graduates into `engine/` as the shipped artifact
   is 9.4's call.
3. **The zero-import core module is withdrawn** (ADR 0003 decisions 1 and 4). The kernel keeps its
   `wasm32-wasip2` discipline (CLAUDE.md), and no longer promises a build with no imports at all.
4. **Native distribution is a packaging problem, and SDKs solve it as packaging.** One statically linked
   `janus-engine` per target triple, resolved by the SDK at a version the SDK pins — the way esbuild,
   Biome and Turborepo ship a native binary through npm optional dependencies, and the way a JVM library
   ships one per classifier. `JANUS_ENGINE` stays the override both SDKs already honour. The build is
   9.4's to specify.
5. **The routes back to a WASM engine that runs a test are named and not taken.** Either way the
   kernel's threaded exchange loop becomes a single-threaded one, which is a kernel change and unproven.
   One route is WASI 0.3 (`wasm32-wasip3`): component-model async lets the exchange loop be an async
   task on `wasi:sockets`, with no thread, and keeps the transport inside the engine. The other is a
   transport the *host* provides — the engine component asks its host for a socket and a poll (finding
   3 option b, ADR 0013's rejected "trampoline") — which puts transport code in every SDK's embedding
   layer, which is what the thinness audit (6.5) measures. The first is preferable if the target and
   its runtimes arrive.

## Alternatives considered

- **Keep ADR 0003's table and fund the host-provided transport now.** Every SDK would ship a "primary" that
  cannot run a test until an unproven kernel change lands, and its first user would be the one SDK
  embedding (Node's) that already has the subprocess working.
- **A `wasm32-wasip1` core module with a WASI shim for Chicory and wazero** (finding 29 option B). Both
  runtimes support WASI p1, so this would replace "zero-import" as the portability story — and fix nothing
  that matters: p1 has no socket API to listen or connect with and no threads, so the module would still
  run no mock.
- **Subprocess for tests, WASM kept as "primary" in name for offline work.** A table in which "primary"
  means "the one you cannot use for the thing you came for" is the misreading this ADR exists to correct.

## Consequences

Easier: one embedding to build, document and test per SDK; a single artifact that does everything,
including hooks (which `janus-engine` now registers, finding 30) and out-of-tree components (ADR 0013);
the SDKs' embedding layers stop carrying a second binding they cannot use.

Harder: per-OS native binaries are back, which is the distribution cost the RFC set out to remove. The
RFC's own drawback — "the subprocess embedding reintroduces process management" — is now the main line
rather than the fallback's footnote; what keeps it from being pact-ruby-standalone again is spike 1.3's
per-run, EOF-exit, protocol-pinned lifecycle, which both SDKs already use. Real Windows CI for the
subprocess (1.3's one open risk) stops being optional. Python, Go and .NET SDKs will each need a
binary-resolution story before their first release.

Committed to: the protocol, not the embedding, is the compatibility surface — an SDK that implements the
frame pipe once can switch embeddings without a protocol change, which is what keeps decision 5 open.

**Tripwire.** Revisit when `wasm32-wasip3` is a supported Rust target and wasmtime (and one other host
an SDK would use: jco for Node, or a JVM runtime) runs p3 components: re-run task 9.1's WASM scenarios
with a mock served and a verification driven from an async, thread-free exchange loop, and if they
work, WASM becomes a candidate primary again. The Rust project is promoting the target to tier 2 as of
this writing; whether it brings threads is open, and an async exchange loop would not need them.
Revisit too if a host-provided transport is prototyped and a WASM engine serves a consumer test's mock
through it; or if per-OS binary distribution proves to be the adoption blocker for an SDK the community
cares about — which is the evidence that would justify funding either route, and nothing short of it.
