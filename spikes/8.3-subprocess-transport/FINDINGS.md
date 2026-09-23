# Spike 8.3 findings — The out-of-process transport escape hatch

Status: **complete**. The boundary exists and holds: a Node transport, declared by a project, served a
consumer test and drove a verification through the real engine. Every containment obligation passes
on Linux. On Windows the first CI run (`spike-8-3`) passed 13 of 14; the one failure was a bug in a
test fixture that Windows exposed and Linux had been hiding (§2.9, §4). Method: [README.md](README.md).

## 1. What was proven

A `tcp` transport — JSON lines over a raw socket, which a WASM sandbox cannot open without the grant it
exists to withhold — written in Node with no dependencies, declared as
`{ kind: subprocess, reference: "node …/tcp-transport.mjs" }`, and loaded by an engine with **no
transport of its own**:

| Scenario (`tests/transport.rs`) | Result |
|---|---|
| consumer test: `start-transport tcp` → endpoint, `serve-variant`, a raw client's line in, the reply line out, `finalise` → `verified`, contract written with `transport.kind: tcp` | ✓ |
| a request that does not match is answered (with the mismatches), not left hanging | ✓ |
| the written contract verified against a JSON-lines provider, through the same component in the `drive` role | ✓ 1/1 verified |
| the same against a provider that answers differently | ✓ fails, 1/1 failed |

| Containment (`tests/containment.rs`) | Result |
|---|---|
| a call that never answers → `component-timeout` at its deadline, process killed, next call gets a fresh, re-handshaken process | ✓ |
| a component in a busy loop — no event loop, never reads stdin again — is bounded the same way | ✓ |
| a component that exits → `component-exited` **at once**, not at its deadline (stdout closing is the signal) | ✓ < 1 s against a 5 s deadline |
| a well-framed body that is not JSON → the call times out; the error counts the unattributed frame | ✓ (§2.3) |
| stray output on stdout (an author's debug print) → skipped, stream in sync, same process | ✓ |
| dropping the component closes stdin; the process exits | ✓ |
| **orphan test**: host SIGKILLed → component exits on stdin EOF | ✓ 11 ms |
| `env` grant: the process sees exactly the granted variables — no `PATH`, no `HOME`, no tokens | ✓ enforced |
| `network: false` (the default): the component listens on a socket anyway | ✓ **not** enforced — the finding |
| one hung call on instance t-2 ends instance t-1's listening socket | ✓ (§2.1) |

## 2. Findings

1. **"Recreate the instance" means "kill every instance in the process".** Spec §3.4 says an
   instance that trapped or timed out is never reused, and is recreated. That rule was written from
   spike 1.4's WASM evidence, where an instance is cheap, stateless and per call. Out of process the
   only way to stop a call is to stop the process. §9.3 lets one process host several instances,
   and a transport instance's state is a listening socket. So one hung `poll-inbound` in one
   session takes every other session's mock with it (`one_hung_call_takes_every_instance…`). The
   spec is internally consistent, but its two rules compound badly. The options go to Phase 9
   (finding 18): **A.** one process per instance when the component is a transport (spawn is 16 ms
   for Node, §3); **B.** tell the engine which instances died, so it reports them rather than finding
   out at their next call; **C.** accept it, and say in §3.4 that out of process the unit of
   recreation is the process.

2. **Grants: `env` is enforceable out of process, `fs` and `network` are not.** Spec §9.3 says
   "grants are not enforceable" of the whole binding. That is too strong by one grant. Starting the
   process with an empty environment plus exactly the granted variables costs nothing, and the test
   shows it holds. The component cannot read a token the engine had and did not grant. `fs` and
   `network` genuinely cannot be enforced without per-OS sandboxing: seccomp or Landlock on Linux,
   `sandbox-exec` on macOS, AppContainer on Windows. Each is a platform project, which is what
   spike 1.3 chose this framing to avoid. The loader warns once per load that they are not enforced.
   Two consequences for the spec (Phase 9 finding 19): §9.3 should say which grant *is*
   enforceable; and an empty environment has a cost of its own. The interpreter is found on the
   *engine's* `PATH`, because the component's own has none, and on Windows `SystemRoot` has to
   survive or Winsock does not start.

3. **An unparseable frame cannot be pinned on the call that caused it.** Correlation is by frame id,
   and a body that is not JSON has none. The stream stays in sync, which is spike 1.3 finding 6
   holding. But the waiting call learns only at its deadline, and the deadline kills the process, so
   the resynchronisation buys the caller nothing. When exactly one call is outstanding, the loader
   could attribute the frame and fail that call as `component-malformed` at once. That is a small
   improvement; the error here already counts `unattributed-frames` so the cause is not lost.

4. **The kernel knew three HTTP things a non-HTTP transport walks into.** One is fixed; two are
   recorded in the [kernel-boundary review](../../Documentation/kernel-boundary-review.md) as
   findings 7–9:
   - **7 (fixed here):** `serve-variant` armed only transports of kind `"http"`, so a passive
     interaction of any other kind was never served. It now arms transports of the interaction's
     own kind.
   - **8:** the passive loop and the verifier read and write parts named `request` and `response`.
     Spec §4 says "there is no `request`/`response` pair anywhere in it", and in the kernel there
     is. The `tcp` transport complies by using those names; a message transport would have to as
     well.
   - **9:** a request that does not match is answered with an HTTP-shaped reply: a `status: 500`
     slot, and the mismatches in `body`. The `tcp` transport ignores `status` and writes `body`,
     which happens to work.

5. **A transport may now be declared, which the kernel could not do before.** `Loaded` carried only a
   content interface; transports came only from the embedding's in-tree map. Now a loaded component's
   transport is looked up by the `kind`s its handshake contributes, ahead of in-tree ones, as content
   already was. A component that implements `transport` and contributes no kind fails at load. This
   is durable kernel code (`engine/kernel/tests/declared_transport.rs`), not spike code, because no
   binding can be tested without it.

6. **Correlation by id is what makes one process usable from two threads.** The exchange loop
   `poll-inbound`s continuously on its own thread, while the dispatch thread calls `start`, `stop`
   and `reply`. Each call waits on its own id, so a `stop` does not queue behind a 200 ms poll. The
   Node side needs nothing special for this: its event loop answers frames as they complete. Spec
   §3.1's "calls are serial per instance" holds, but *per process* they are not serial, and neither
   side assumes they are.

7. **`source.reference` is under-specified for `subprocess`.** The schema says "the command the
   engine spawns". It does not say whether that is a shell string or an argument vector, or what a
   relative path in it is relative to. The 2.7 loader resolves `file` references against the
   configuration's directory and leaves `subprocess` alone. The spike splits on whitespace, which
   breaks on the first path with a space in it. Phase 9 finding 20.

8. **`console.log` is the first thing to corrupt a stdout pipe, and one line fixes it.** Every Node
   author's first debugging move writes to the frame channel. `pipe.mjs` sends `console.log` to
   stderr, and the loader also skips stray lines that are not headers rather than dying on them. A
   print that happens to contain a colon still parses as an unknown header, which is ignored. Only
   a print that splits a header line would desynchronise the stream, and the length prefix
   recovers it at the next frame.

9. **One unhandled socket error ends every instance in the process, and Windows makes it
   routine.** The first Windows run failed `one_hung_call_takes_every_instance…` with
   `component-exited` where `component-timeout` was expected. The component had died *before* the
   hung call. The fixture's listening socket had no `'error'` listener. The test connected and
   closed without reading, which resets the connection, and Node turns an unhandled `ECONNRESET` into
   an uncaught exception that ends the process. Windows resets reliably. Linux usually avoided it on
   timing alone, and does crash when the reset is forced (reproduced with `resetAndDestroy()`). The
   binding did its part: the death was reported at once, as `component-exited`, and the next call
   got a fresh process. But this is finding 1 again, from the other side. A component process is a
   single point of failure for every instance it hosts, so one careless socket in a transport ends
   every session's mock. The fixture now handles the error, and `tcp-transport.mjs` handles errors on
   every socket and on its server after `listen`. The lesson for the spec's author guidance (Phase 9
   finding 18): a transport author must treat every socket error as survivable. A catch-all
   `uncaughtException` handler in the framing module would also keep the process alive, but it would
   hide real bugs behind a component that looks healthy.

## 3. What it costs

`cargo run --release --example call_cost`, Node 22, Linux, same machine as spikes 1.3 and 8.1:

| | this spike | read against |
|---|---|---|
| spawn to completed handshake | **16 ms** (Node start-up) | spike 1.3: 1.2 ms for the Rust engine binary |
| one synchronous op, round trip | **10–11 µs** | spike 1.3: 10.5 µs Node ↔ engine — the same pipe, the same number |
| `poll-inbound` with `timeout-ms: 0` | 1.07 ms | the component's `setTimeout(0)`, not the pipe: Node's timer floor |
| one consumer exchange, end to end (client line in → match → reply out) | **~93 µs** | 8.1: ~69 µs *per WASM call*, fixed |

The pipe is not the cost. Once a process is running, an out-of-process transport call is ~6× cheaper
than a WASM component call, because the WASM loader pays instance-per-call and this binding reuses one
process. The cost is start-up (16 ms per component per session). And the 1 ms timer floor would
be paid on every idle poll if the kernel ever polled with `timeout-ms: 0` (it polls with 200 ms).

## 4. Windows (spike 1.3's open risk)

Not observed. What the code does about it, so CI has something specific to confirm or refute:

- the interpreter is resolved on the engine's `PATH` with `.exe`/`.cmd`/`.bat`;
- `SystemRoot` is kept in the component's otherwise-empty environment;
- orphan prevention relies on the same mechanism as spike 1.3: parent death closes the pipe handle
  and Node sees `end` on stdin;
- the tests kill with `Child::kill` (TerminateProcess) and check liveness with `tasklist`.

The `spike-8-3` CI job runs this directory's tests on `windows-latest`. Its first run — the first
real-Windows evidence for component processes, and for the engine's own subprocess, since the
mechanism is the same — passed 13 of 14. Spawning through the engine's `PATH`, the empty environment
with `SystemRoot`, timeouts with kill escalation, respawn, exit on stdin EOF, and **the orphan test**
(a host killed by `TerminateProcess`, its component gone) all held on real Windows. That closes
spike 1.3's open risk for this mechanism. The one failure was not the binding: it was §2.9, a
fixture bug that Windows' connection resets exposed.

## 5. Not done

- **The gRPC stretch.** A transport that *speaks* gRPC to a provider, reusing this pipe. It would
  prove nothing about the boundary that `tcp` does not, and the plan scoped it as a stretch. It is
  worth doing when Phase 9 decides whether the pact-plugins ecosystem needs a bridge.
- **A shipped embedding.** Neither `janus` nor `janus-engine` registers this loader. Their
  `engine/hello` still says `["in-tree", "wasm"]`, which is true. Graduating the loader into
  `engine/component-host` is a decision for after §2.1 is settled, because §2.1 changes what
  "recreate" means.
- **Content, matcher and hook components over this binding.** The loader binds `transport` only. The
  others are the same adapter with other op names; §9.3 exists for transports.

## 6. Recommendations

- Keep the binding as specified: Content-Length framing, exit on EOF, correlation by id. Every
  claim ADR 0013 made for it held in a language other than Rust.
- Amend spec §9.3's grants paragraph: `env` is enforced, `fs` and `network` are documented intent.
  And settle §2.1 before any embedding ships this loader.
- Fix kernel-boundary findings 8 and 9 before a message transport arrives, because it will not be
  able to pretend to have a `request` and a `response`.
