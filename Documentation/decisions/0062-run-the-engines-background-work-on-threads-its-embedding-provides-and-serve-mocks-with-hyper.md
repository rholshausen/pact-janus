# 0062 — Run the engine's background work on threads its embedding provides, and serve mocks with hyper

- **Status**: accepted
- **Date**: 2026-11-01
- **Plan tasks**: S2.1 (whose async, thread-free requirement this withdraws); gate W; amends
  [ADR 0035](0035-reach-providers-and-brokers-through-one-async-http-client-configured-by-one-document.md)
  decision 2 and extends [ADR 0023](0023-the-subprocess-is-the-primary-embedding-and-wasm-serves-offline-operations.md)
  decision 5
- **Evidence**: the kernel's three `thread::spawn` calls (`protocol/session.rs`, the exchange loop;
  `protocol/verification.rs`, a run; `protocol/broker.rs`, a publication); `engine/component-http`'s
  serve half on `tiny_http`; `engine/wasm`, the one embedding with no threads, which serves offline
  operations only and which no SDK or CLI uses; today's mock server (`pact-core-mock-server`'s
  `pact_mock_server/src/hyper_server.rs`: hyper 1 through hyper-util's `auto` builder on tokio,
  serving HTTP/1.1 and HTTP/2, with TLS from a self-signed certificate); pact-js's `tls` mock option;
  [ADR 0013](0013-component-hosting-is-an-embedding-capability.md), the `broker` and
  `config-loading` engine capabilities (engine-protocol spec §5.3)

## Context

S2.1 asks for the consumer side's exchange loop and the HTTP transport's server to be async and
thread-free, behind an executor seam, so that native runs on tokio and a `wasm32-wasip3` build runs
on WASI async with the same loop. Gate W would then cost a build target, not a redesign. ADR 0035
applied the same rule to the HTTP client, and said S2.1's server would share its executor.

The rule exists for one embedding. ADR 0023 made `janus-engine` every SDK's primary embedding, and
the WASM component the embedding for offline operations. It named two routes back to a WASM engine
that runs a test and took neither. Both SDKs and the CLI run native, and none of them gains anything
from a thread-free engine. They would pay for one, though:

- **The dispatcher would have to stop blocking.** `janus-engine` reads frames from stdin on its main
  thread. With a mock served on that thread too, the read must not block. `tokio::io::stdin` reads
  on a hidden blocking thread, and Windows pipes resist a read that is truly non-blocking.
- **A verification run would have to yield everywhere.** It waits on the provider, on `exec` hooks'
  processes, on scripted hooks and on the broker. Every one of them would need an async form.
- **The target is not the blocker it was.** `wasm32-wasip3` is tier 2 and stable from Rust 1.100.0
  (12 November 2026). WASI's shared-everything-threads proposal is aimed at the component model's
  iterations after 0.3. That could take months or years, but a thread-free engine would no longer
  be the only way a WASM embedding could run a test.

The engine needs concurrency for one reason, in three places. Some work must go on beside the frame
stream while the dispatcher keeps answering: a transport's exchange loop waits for traffic, and a
verification run and a broker publication must answer at once and report on the event stream
(engine-protocol spec §8.3). That is a capability, and an embedding either has it or does not, as it
does or does not have a broker client (`broker`) or a configuration loader (`config-loading`).

## Decision

1. **The native engine uses threads.** S2.1's async, thread-free requirement is withdrawn, and with
   it the last clause of ADR 0035 decision 2 ("S2.1's server half runs on the same one"). The HTTP
   client stays as ADR 0035 built it: a current-thread runtime per client, which starts no thread,
   because reqwest's TLS, proxies and HTTP/2 are worth the runtime.
2. **The kernel starts no thread itself; it asks its embedding.** A `Background` trait in the
   kernel runs a named task beside the frame stream and returns a handle the kernel can join.
   An embedding registers one (`Engine::register_background`). The kernel ships `Threads`, its
   `std::thread` implementation, which is the only code in the kernel that calls `thread::spawn`. A
   test holds the rest of the kernel's non-test code to that, as `tests/boundary.rs` does for
   `pact_models`. `janus`, `janus-engine` and the Rust tests that start a transport or a run
   register `Threads`. The WASM embedding registers none. The kernel registers none by default, so an
   embedding without the capability gets the right answer without doing anything.
3. **It is an engine capability, `background`.** `engine/hello` lists it beside `broker` and
   `config-loading`, value `{}`. Without it, the three operations that need it answer
   `component-unavailable` naming `background`. They are `consumer-session/start-transport`,
   `verification/verify` and `broker/publish-contracts`. That is what the WASM embedding already
   does for the first two, now for the reason that holds.
4. **Handling an arrival is a synchronous step of its own.** The exchange loop is two parts:
   waiting for the next arrival (`poll-inbound` with its timeout), and handling it. Handling matches
   the arrival against the armed exchange, replies or disposes of it, and records the outcome. It
   waits on nothing. A driver that waits for arrivals another way (decision 7) calls the same step.
   A transport instance still handles one arrival at a time, as now. Routing arrivals across several
   armed interactions (variant-semantics spec §4.1) stays open.
5. **The HTTP transport's serve half is hyper.** It is hyper 1 through hyper-util's `auto` builder,
   serving HTTP/1.1 and HTTP/2 with prior knowledge, as today's mock server does.
   - Each served instance has its own thread and its own current-thread tokio runtime. The component
     starts them itself: it is native only and outside the kernel, as `hooks-host`'s readers are.
   - The server collects a request whole and puts it on a channel, where `poll-inbound` waits for
     it within its timeout. `reply` and `dispose` answer through a one-shot channel kept per event.
     `stop` shuts the runtime down and wakes a waiting `poll-inbound`.
   - The transport interface does not change, and `tiny_http` is removed.
   - No new crate arrives. hyper, hyper-util, tokio, h2, rustls and tokio-rustls are already in
     `Cargo.lock` through reqwest; only their server features are new.
   - The runtime is there because hyper needs one, and hyper is the reason to choose it. It brings
     HTTP/2, it is maintained, it is what today's mock server is built on, and it uses the `http`
     types the drive half already uses. The runtime is the server's own, not shared with any client,
     for ADR 0035's reason: one driver contended for by unrelated work.
6. **TLS for a served mock comes when an SDK needs it.** The serve role's options are `host` and
   `port`. S2.5's facade maps pact-js's `tls` and pact-jvm's HTTPS mock onto new serve options,
   specified in component-interfaces spec §5.6 and built on the rustls stack already in the tree.
   How a certificate is made (today's mock server generates a self-signed one) is decided there.
7. **Gate W costs a driver, not a build target.** A WASM engine that runs a test needs two things.
   - An embedding that provides `background` on its target. With shared-everything-threads, that is
     `Threads` on WASI threads. Without them, it is a driver that runs decision 4's step from async
     tasks, and a verification run in a form that yields.
   - A serve half for the target, on `wasi:sockets` or `wasi:http`'s p3 server. The transport was
     never built for WASM (ADR 0023), and tokio does not serve there.

   That is work, and this ADR accepts it as the price of not paying it now. ADR 0023 decision 5
   gains a third route: WASI threads, which need no single-threaded kernel at all.

## Alternatives considered

- **Async and thread-free now, as S2.1 said.** It builds for an embedding that nothing uses. It
  needs a non-blocking stdin that tokio only fakes, and a verification run made async through hooks,
  scripts and the broker client. Two SDKs and a CLI gain nothing from it.
- **A mock-server component that owns its threads and calls into the engine.** It keeps threads out
  of the kernel, which decision 2 keeps too. But the transport would call in, which
  component-interfaces spec §5.3 rules out ("the engine drives; the transport never calls in"). An
  out-of-process transport would then need calls in both directions. The loop that HTTP and message
  mocks share would split per transport. And the run's and publication's threads would stay where
  they are.
- **A `threads` cargo feature on the kernel.** It decides at build time what a registration
  decides at start-up, so an embedding cannot bring its own alternative without changing the kernel.
  `std::thread` compiles for `wasm32-wasip2` anyway, so the feature would guard nothing the compiler
  checks. A registered capability also appears in `engine/hello`, where a host can see it.
- **Keep `tiny_http`.** It blocks and needs no runtime. But it serves HTTP/1.1 only, it starts
  threads of its own anyway, and pact-js and pact-jvm users' mocks serve HTTP/2 and TLS today.
- **Share the HTTP client's executor with the server.** ADR 0035 already rejected one runtime
  shared by unrelated work: a verification's requests and a publication would contend for one
  driver.

## Consequences

Easier: S2.1's server work is a server swap behind an interface that does not change. Neither SDK
and no embedding changes beyond registering `Threads`. The stdin question goes away. A served mock
gains HTTP/2 now, and TLS once its options exist.

Harder: gate W is now a piece of work rather than a build target. The kernel has a seam to keep:
a new background task goes through `Background`, never `thread::spawn`, and the test fails a change
that forgets.

Committed to: the engine capability `background`; `component-unavailable` naming `background` for
the three operations; hyper as the mock's server.

Tripwires:

- **Shared-everything-threads ships** in WASI, and wasmtime and one host an SDK would use (jco, or
  a JVM runtime) run it. The WASM embedding registers `Threads`, and gate W is re-run with a served
  mock and a driven verification.
- **An SDK needs a WASM engine that runs tests before then**, because its platform cannot run a
  native binary. That is the evidence ADR 0023 asks for: build decision 7's thread-free driver.
- **A consumer test sends a mock concurrent requests** that one-at-a-time handling serialises badly,
  such as a browser's parallel fetches. The step runs once per arrival on threads, which decision 4
  makes possible without redesigning the loop.
