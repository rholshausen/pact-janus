# Spike 1.3 findings — Subprocess embedding

Status: **complete** — all lifecycle scenarios pass from all four host languages (Node 22,
Python 3.14, Go 1.26, JVM 17); Windows covered by cross-compile + full session under Wine
(explicitly *not* real-Windows evidence — see finding 7). Method: [README.md](README.md).

## 1. What was proven

`pact-engine-toy` — the 1.2 toy engine behind `Content-Length`-framed JSON over stdio (LSP
framing), protocol frames on stdout only, logs on stderr, 417 KB release binary. Two exit paths by
design: an explicit `shutdown` op, and **exit-on-stdin-EOF** as the orphan-prevention mechanism.

Every host ran the same scenario list, all green:

| Scenario | Node | Python | Go | JVM |
|---|---|---|---|---|
| handshake negotiates v1; v99 rejected with `protocol-version-unsupported` naming supported versions | ✓ | ✓ | ✓ | ✓ |
| clean shutdown: op acked, exit code 0, no process left | ✓ | ✓ | ✓ | ✓ |
| stdin closed without shutdown → engine exits 0 (EOF path) | ✓ | ✓ | ✓ | ✓ |
| **orphan test**: host runner SIGKILLed mid-session → engine gone | ✓ ≤50 ms | ✓ ≤50 ms | ✓ ≤50 ms | ✓ ≤50 ms |

Pipe benchmark (same frames, payload and iteration counts as spike 1.2, same machine — directly
comparable):

| Host | spawn-to-ready (incl. handshake) | echo small | echo 100 KB | match 100 KB |
|---|---|---|---|---|
| Node 22 | 1.20 ms | 10.5 µs | 2.18 ms | 2.42 ms |
| Python 3.14 | 0.60 ms | 7.0 µs | 2.21 ms | 2.28 ms |
| Go 1.26 | 0.57 ms | 4.9 µs | 2.21 ms | 2.28 ms |
| JVM 17 | 1.06 ms | 4.8 µs | 1.74 ms | 2.27 ms |

## 2. Findings

1. **The fallback is real — B1 is safe to assert.** Spawn, version handshake (both accept and
   reject paths), framed request/response at document scale, clean shutdown, and
   no-orphan-on-kill all work from all four languages with no platform-specific machinery. The
   RFC's "WASM preferred, subprocess fallback" posture now has both halves demonstrated (1.2 +
   this spike).
2. **Exit-on-stdin-EOF makes orphan prevention a design property, not platform code.** No process
   groups, `prctl(PR_SET_PDEATHSIG)`, job objects, or signal choreography anywhere: when the test
   runner died — including by SIGKILL, which permits no cleanup — the OS closed the pipe and the
   engine exited within one 50 ms poll interval, in every language. This works because the engine
   treats its stdin as its lease on life; that behaviour must graduate from toy convention to a
   **normative requirement in the protocol spec (2.1)**, alongside the `shutdown` op. The
   converse case — a wedged engine that stops reading stdin — is not covered by EOF and needs the
   standard kill escalation (`SIGKILL`/`destroyForcibly` after a timeout) in each SDK's process
   manager; that is ordinary subprocess hygiene, not protocol design.
3. **The pipe tax is small and flat**: ~4–10 µs per small round-trip, ~0.7–0.9 ms extra per
   100 KB operation versus the fast in-process WASM paths from 1.2 (~2.2 ms vs ~1.3 ms). Two
   copies through kernel pipes plus framing — no surprises, no cliffs up to the ~200 KB
   match frames.
4. **For two of the four languages, the subprocess is currently the *faster* practical path at
   document scale** — an inversion of the "fallback" framing worth stating plainly: JVM pipe
   1.74 ms vs Chicory-compiled 5.2 ms per 100 KB echo; Python pipe 2.21 ms vs wasmtime-py
   component 97.9 ms (and roughly par with its core+shim 1.32 ms). Only V8's in-process path
   clearly beats its pipe. The G1 embedding-priority decision should treat WASM-vs-subprocess per
   language as a genuinely open trade (isolation, distribution, and ops ergonomics — one process
   vs two — now matter more than raw speed).
5. **Spawn cost is negligible for test-runner lifecycles**: 0.6–1.2 ms from spawn to completed
   handshake. Engine-per-test-file would be affordable; engine-per-run is trivially so. No case
   for daemon/keep-alive complexity on performance grounds.
6. **Length-prefixed framing is robustly self-synchronising.** A frame whose body was truncated
   (host arithmetic bug in an early Wine run) produced an in-band `malformed-frame` error and the
   session *continued in sync* — the declared length, not the JSON, delimits the stream. Unknown
   headers are ignored (LSP behaviour). The framing client is 30–60 dependency-free lines per
   language (the JVM leg is a single self-running file), consistent with the thin-SDK rule.
7. **Windows: strong but indirect evidence.** The binary cross-compiles to PE
   (`x86_64-pc-windows-gnu`), and a full framed session — handshake, echo, shutdown ack, exit
   0 — runs under Wine, confirming binary-mode stdio (no CRLF mangling of frames) and the EOF
   exit path in the Windows build. Not covered by Wine: real `CreateProcess` overhead, console
   ctrl-event semantics, job-object kill-tree behaviour, and antivirus-scan latency on spawn.
   Orphan behaviour on Windows relies on the same mechanism (parent death closes the pipe handle,
   `ReadFile` returns EOF) so it *should* transfer, but this needs a real-Windows CI job before
   G1's conclusions harden (the plan's CI matrix is the natural home).
8. **stdout purity held.** With logs on stderr and protocol on stdout, no leg ever saw a
   corrupted frame — but the toy had no third-party code in-process. The real engine must keep
   the "stdout is the protocol's" rule enforceable (tracing subscriber to stderr by default), or
   frames and stray prints will eventually interleave.

## 3. Recommendation into G1

- Subprocess embedding is **proven, cheap, and in two languages currently the fastest practical
  path** — promote it from "fallback" to a first-class embedding whose per-language priority is
  decided on ergonomics and distribution, not feasibility.
- Adopt **LSP-style Content-Length framing** for the stdio pipe (self-synchronising, trivially
  implementable, survives malformed payloads in-band) — it is the subprocess instantiation of the
  1.1 "frozen pipe per embedding".
- Specify in the protocol spec (2.1), as normative engine behaviour: **exit on stdin EOF**, the
  `shutdown` operation, stdout carries only protocol frames, logs to stderr.
- Carry one open risk into the plan: **real-Windows validation in CI** (spawn, kill, EOF exit) —
  Wine evidence is suggestive, not sufficient.

Out of scope here, noted for protocol design 2.1: the toy pipe is strictly serial
request/response; the real protocol's event streams (`verify` progress) and any concurrent
sessions over one pipe need explicit design (request ids / LSP-style message correlation).
