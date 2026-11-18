# 0095 — Recompute a missing report for the pair a broker is asked about, in its WASM engine, and never a whole fleet at once

- **Status**: accepted
- **Date**: 2026-11-18
- **Plan tasks**: S4.5 (broker notes row 9); S4.3d and S4.3e build what it decides; S4.11 is the
  scaling work it found
- **Evidence**: `benchmarks/janus/src/fleet.rs` and its results in `benchmarks/results/`
  (`2026-11-18-janus-native-0c0fb4b-dirty.json`, `2026-11-18-janus-wasm-0c0fb4b-dirty.json`),
  [broker notes](../prototype/broker-integration-notes.md) §3c, §4 and §12,
  [ADR 0092](0092-decide-in-a-broker-through-the-engines-wasm-component-and-refuse-to-start-without-it.md)

## Context

The broker notes recommended that a broker embed the engine and recompute a subsumption report when
it has none stored (§3c). A report is a function of a contract, a provider shape and the engine
version, and a broker keys what it stores by all three (row 6). The notes left open whether
recomputing scales across a fleet (§12). ADR 0092 settled the embedding: `janus-engine.wasm` under
Ruby's `wasmtime` gem and the `wasmtime` crate, an instance per decision. It left two questions to
S4.5: whether recomputing on a cache miss is affordable, and whether the Rust port should link the
kernel natively instead.

The cache misses in three ways: a pair no CI run published a report for, a provider publishing a new
shape (each consumer's report against it), and a broker moving its engine pin (every report it holds).

`benchmarks/janus` now measures each, in the WASM embedding and linked natively, on a generated fleet.
One provider has 40 operations, and its shape at each of 10 versions is recorded by the engine from
six answers per operation and a 404, as S4.1 records one: 80 entries. 50 consumers each have a v3
pact of 5, 10, 20, 50 or 100 interactions over those operations, 1,850 in all. Every interaction has
three findings, as real pairs do. Each pair is answered as ADR 0092 decision 5 says: a fresh instance,
its handshake, `subsumption/check` and `subsumption/decide`. Measured on one developer machine
(the results files), medians:

| Scenario | Native | WASM |
|---|---|---|
| a fresh instance | 8 µs | 49 µs |
| one pair, 5 interactions | 8.4 ms | 11.0 ms |
| one pair, 50 interactions | 31 ms | 40 ms |
| one pair, 500 interactions | 256 ms | 328 ms |
| one pair, 50 interactions, against a 400-entry shape | 105 ms | 137 ms |
| deciding a stored report of 50 interactions | 2.3 ms | 2.7 ms |
| a new shape: 50 consumers' reports | 1.2 s | 1.6 s |
| a pin move: 500 pairs, 18,500 interactions, one thread | 12.2 s | 15.9 s |
| the same on four threads | 3.1 s | 4.1 s |

What was found:

- **WASM costs 1.2 to 1.3 times native**, at every size, as ADR 0092 found for one large contract.
  Instantiating is negligible either way. Under Ruby, crossing the gem adds about 2 ms for a 600 KB
  frame (ADR 0092).
- **The cost is linear in interactions** (about 0.6 ms each in WASM against this shape), plus a few
  milliseconds a check spends reading the two documents. Deciding a stored report is cheap.
- **Selection grows with the shape too.** A check tries every entry's selector for every interaction
  (ADR 0025), so its cost is interactions × entries. With 80 entries, selection was 60% of the walk.
  Five times the entries made a check 3.4 times slower. A provider with a few hundred operations and
  a consumer with a few thousand interactions would take seconds a pair.
- **Each selector was compiled again for every interaction.** S4.5 compiles each once per check,
  which made a large pair's check 40% faster and the cross-product 30% faster, with no verdict changed.
  An invalid selector is now reported once, not once per interaction that read it.
- **The cross-product is linear in pairs and scales with threads.** Four threads gave 3.9 times one.

## Decision

1. **Both brokers recompute through `janus-engine.wasm`, and neither links the kernel.** This confirms
   the notes' §3c and ADR 0092 decision 1. Linking natively saves about a quarter of each check.
   That does not pay for building QuickJS in the port's toolchain, or for a second artifact whose
   answers would have to be held to the first's frame for frame.
2. **A report the broker does not hold is computed when a decision needs it, for that pair only.** A
   `can-i-deploy`, a matrix row or a pair page that needs a report keyed (contract SHA, shape SHA,
   engine version) and finds none runs `subsumption/check` in a fresh instance, stores the report it
   gets under that key, and decides as it would over a stored one. A report CI published fills the
   same key. For the contracts measured, that is tens of milliseconds a pair.
3. **The computation is bounded.** A broker sets a time budget on it, configurable, using the
   runtime's own interruption. An interrupted call is a trap, which ADR 0092 decision 5 already
   answers: the pair's subsumption half is unanswered, `deployable` is `nil`, and the reason says the
   report is still being computed. The broker then computes that report with no budget, off the
   request, and stores it, so a later read is a cache hit. A missing input is never a pass.
4. **Nothing waits on recomputing a fleet.** A broker that moves its engine pin keeps the reports it
   holds under the old version and computes new ones as reads miss. Publishing a shape or a contract
   computes nothing in its own request. A broker may warm its cache off the request, for instance on
   `provider_shape_changed` for the consumer versions its matrix shows deployed or latest, but no
   decision depends on that having run.
5. **The harness keeps measuring it.** The fleet scenarios run on every merge with the rest of
   `benchmarks/janus`, and their numbers are on the trend chart (S0.3), so a change that makes the walk
   slower shows.

## Alternatives considered

- **Linking the kernel into the Rust port.** It is faster by the margin above, and needs a native
  build of QuickJS (libclang) and a second artifact held to the first. The margin is less than four
  threads give.
- **Recomputing every affected report when a shape is published.** The publish request would wait on
  N checks, 1.6 s for this fleet's 50 consumers, and most of those reports are never read.
- **Recomputing every report when the engine pin moves, before serving.** Minutes for a large fleet,
  at boot or in a deploy, for reports most of which are never read again.
- **Storing only what CI published (the notes' option b).** A new shape silently leaves every
  consumer's report stale until each consumer builds again (notes §3b).
- **A batch operation that checks many contracts against one shape.** It would read the shape once
  instead of once a pair, which saves a few milliseconds of tens. That is not worth a protocol
  operation now.

## Consequences

- S4.3d stores reports under the key and serves them, and S4.3e reads them, recomputing on a miss as
  decision 2 says. The budget's reason and its background computation are S4.3e's to build in both
  brokers. Its exit includes a pair whose report is computed on read.
- A fleet's first reads after a pin move are slower, a pair at a time, and nothing is slow all at
  once.
- Selection's cost is S4.11: an interaction tries every entry, so a check of a large contract against
  a large shape is seconds, where the walk itself would be a fraction of that.
- *Tripwire:* revisit if a pair real users have takes longer than a broker request can wait even
  after S4.11, or if the WASM embedding's cost moves well beyond 1.5 times native.
