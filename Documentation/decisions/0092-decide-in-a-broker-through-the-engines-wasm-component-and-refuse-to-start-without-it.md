# 0092 — Decide in a broker through the engine's WASM component, and refuse to start without it

- **Status**: accepted
- **Date**: 2026-11-15
- **Plan tasks**: S4.3a (the embedding rows 7 and 8 decide through); S4.5 measures the walk's cost
  at a fleet's scale
- **Evidence**: [broker integration notes](../prototype/broker-integration-notes.md) §3 and §4,
  [ADR 0023](0023-the-subprocess-is-the-primary-embedding-and-wasm-serves-offline-operations.md) decision 2,
  [ADR 0016](0016-subsumption-defaults-to-warn-with-mandatory-reason-exemptions.md),
  [ADR 0089](0089-scope-a-subsumption-policy-and-its-exemptions-by-the-provenance-of-the-shape-a-finding-came-from.md),
  `engine/component-host/tests/engine_wasm.rs`, `engine/wasm/hosts/ruby/pipe.rb`,
  [the broker PR series](../broker-pr-series.md) §3

## Context

Rows 7 and 8 of the broker series put subsumption reasons into `can-i-deploy` and keep a policy on
the integration. Deciding a pair applies the policy's layers, its exemptions, their expiry and their
provenance (ADR 0016, ADR 0089), which is `subsumption/decide`. A broker that applied them itself would
be the second implementation the notes reject (§3a, §6), so a broker has to run the engine. The notes
recommended hosting the released WASM component, `janus-engine.wasm`, under `wasmtime`, and said that
a Ruby binding for it existed was "the one claim in these notes taken on report rather than read".
S4.3a checks that claim in both brokers' languages before any row depends on it.

What was found:

- **Ruby's `wasmtime` gem hosts it.** Version 50.0.0, the version of the `wasmtime` crate this
  repository builds against, has the component model (`Wasmtime::Component`) and WASI preview 2
  (`Wasmtime::WASI::P2`). Its precompiled gems cover Linux (glibc and musl, x86-64 and arm64), macOS
  and Windows for Ruby 3.3 and later. The Ruby broker supports Ruby 3.2, which reached end of life on
  2026-03-31; there the gem builds from source and needs a Rust toolchain. The official broker image
  runs Ruby 3.4.10 on Alpine, which the musl gem covers.
- **The `wasmtime` crate hosts it.** That is what the Rust port would use, and `engine_wasm.rs`
  already hosted the component with it for `upgrade/pact`.
- **The artifact answered differently from `janus check`.** The WASM engine registered the content
  components and nothing of the HTTP component, because it has no transport. A provider shape recorded
  or derived by operation (ADR 0086, ADR 0088) selects with the HTTP component's `http:path`, so no
  entry matched anything, and over the sample provider's pact and recorded shape the artifact answered
  `not-checked` where `janus check` finds three findings. Every shape S4.1 and S4.2 write is selected
  that way.
- **What it costs, under Ruby.** Compiling the 9.7 MB component takes 644 ms, and loading a copy
  compiled ahead of time takes 7 ms. A new instance takes 0.5 ms and its handshake 0.3 ms. Over the
  sample provider's documents, `check` takes 0.9 ms and `decide` 0.24 ms. Over a 1,500-interaction
  contract, `check` takes 575 ms and `decide` over its 909 KB report 143 ms, where native `janus check`
  does both in 558 ms. Crossing the gem as an Array of Integers costs 2 ms for a 616 KB frame and 11 ms
  for a 909 KB answer.

## Decision

1. **A broker decides through `janus-engine.wasm`, over its one exported function.** The Ruby broker
   hosts it with the `wasmtime` gem and the Rust port with the `wasmtime` crate. Each sends the
   engine-protocol frames any host sends: `engine/hello` first, then `subsumption/decide`, and
   `subsumption/check` where S4.5 recomputes a report. A broker never reads the engine's Rust API
   and never reimplements a rule `decide` applies. `engine/wasm/hosts/ruby/pipe.rb` is the
   Ruby host as a broker would write it.
2. **The guest is granted nothing.** It gets no environment, arguments, files, sockets or inherited
   stdio, as `pipe.rb` and `engine_wasm.rs` grant it. A decision is a function of the frame alone.
   The date it is judged against is `as-of`, which the broker supplies from its own clock as the UTC
   date of the request, unless the query names one.
3. **The artifact answers as `janus check` does, and a test holds it to that.** The WASM engine now
   registers the HTTP component's matchers, built without its transport: the crate's transport is
   behind a default `transport` feature. `engine_wasm.rs` runs one conversation (the handshake, a
   `check` of the sample provider's pact against its recorded shape, and `decide` under no policy,
   under an exemption the day before it lapses, and the day after) through the native engine as
   `janus` registers it, through the artifact under the `wasmtime` crate, and through `pipe.rb`
   under the gem. Every frame must be the same. The Ruby half runs when `JANUS_RUBY` names a Ruby
   with the Gemfile's gems; CI's Linux job and the release workflow set it, so the bytes a release
   ships are checked under both runtimes.
4. **A broker pins the artifact by version and SHA-256, as an SDK pins `janus-engine`** ([ADR 0029](0029-attest-what-a-release-built-and-let-an-sdk-verify-the-digest-it-pins.md)).
   It reads the artifact from a configured path; bundling it is the image's job. At boot it compiles
   the component once, or loads a copy compiled ahead of time for its own `wasmtime` version, and
   shakes hands with it. The handshake must offer protocol version 1 and the `subsumption-check`
   capability, and the `engine.version` it answers is the engine-version member of row 6's report
   key (notes §3). Moving the pin is a broker change, made as an SDK moves its pin.
5. **Each decision gets an instance of its own.** An instance costs half a millisecond, holds the
   engine's state, and is not shared between a multi-threaded server's requests, so none needs a lock
   and none can leak one request's state into another. A call that traps, or answers an error frame,
   leaves that pair's subsumption half unanswered. The pair's `deployable` is `nil`, with a reason
   naming the error. It is never the verification-only answer, because a missing input is never a
   pass (engine-protocol spec §8.6 rule 3).
6. **With the `janus` feature on and no usable artifact, the broker refuses to start.** That covers a
   missing artifact, one whose digest is not the pin, one that does not compile, and one whose
   handshake lacks protocol 1 or `subsumption-check`. The refusal names the path and what was wrong.
   This is how a declared component that cannot be loaded fails before anything runs. With the
   feature off, nothing loads the artifact, the gem or the crate: in Ruby the gem is required only on
   the feature's path and is not a dependency of the `pact_broker` gem, and in the port the embedding
   is behind a Cargo feature. So a broker without the feature carries no WASM runtime.

## Alternatives considered

- **Answering verification only, with a warning reason, when the artifact is absent.** This is the
  degradation the notes sketched (§3: "a broker without the embedding degrades to (b)"). It was
  rejected because a deploy pipeline reads `deployable`, not the reasons. A broker that turned the
  feature on and could not decide would pass every pair on half the question, silently. Refusing at
  boot finds the configuration error when the broker is deployed, not when a consumer is.
- **Answering every pair `nil` when the artifact is absent.** This is honest, but it blocks every
  deploy in the fleet over a broker's configuration, and the first anyone hears of it is a blocked
  release.
- **Linking the kernel into the Rust port natively.** This is faster, but it means a QuickJS build
  (libclang) in the port's toolchain. It would also be a second artifact, whose answers would have to
  be held to the first's. S4.5 measures it beside the WASM embedding, and its ADR decides on those
  numbers.
- **Running the `janus-engine` subprocess.** That is a binary per platform in every broker image,
  and a process to supervise. The WASM component is one file for every platform, and needs no
  process.
- **One long-lived instance behind a lock.** This saves half a millisecond a decision, but it
  serialises every `can-i-deploy` a server answers and keeps engine state across requests.

## Consequences

- The claim the notes took on report is now checked on every pull request, under both runtimes, and
  holds. Rows 7 and 8 can be written against `decide`.
- The WASM artifact gives the answer `janus check` gives for shapes selected by operation, which it
  did not before. Every host of `janus-engine.wasm` gets that, not only a broker.
- The Ruby broker's `janus` feature needs Ruby 3.3 or later, or a Rust toolchain to build the gem.
  The series says so in its first pull request that needs the gem (row 7).
- A broker answers with the engine version it pins, so two brokers on different pins can differ
  where a newer engine reports a finding an older one did not (notes §3). Row 6's key carries the
  version for that reason.
- A 1,500-interaction contract takes over half a second to `check`, at about 1.3 times the native
  cost. That is fine for a report computed once and cached. Whether recomputing on a cache miss
  scales across a fleet is S4.5's question.
- *Tripwire:* revisit if the `wasmtime` gem drops the component model or its precompiled gems, or if
  S4.5's measurements show the embedding cannot answer `can-i-deploy` within a broker request's
  budget.
