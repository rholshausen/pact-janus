# Variant ergonomics report

Plan task **4.6** (`[explore]`). Closes Phase 4 (Milestone M2). Deliberately writes consumer code
that mishandles a variant and evaluates the failure experience against the two questions the plan
sets: is the failing variant identifiable, and is the sampled matrix understandable? Sharp edges
here are an RFC "drawbacks" item; this report documents them honestly, alongside the one place the
exploration led straight to a same-session fix.

**Method.** Two reproductions against the *real* engine (plan task 4.5's own infrastructure, not a
stand-in for it): [`engine/kernel/examples/careless_consumer.rs`](../engine/kernel/examples/careless_consumer.rs)
(Rust, in-process embedding, committed) and a Vitest `it.each` harness driving the `janus-engine`
subprocess (TypeScript, exploratory only — not committed; reproduction steps in §6). Both submit the
RFC order interaction widened with an `optional` `shippedAt` (shape-language spec
`examples/order-payload.md`'s own field, legitimately absent for a `PENDING` order), call
`consumer-session/variants`, correctly loop over **every** variant it selects — the honesty rule is
satisfied, nothing is skipped — and then read the response the way most hand-written test code
actually does: assume the field is there.

## Verdict

**Identifiability is not a property of the engine or the protocol at all — it is entirely a
function of what the consumer's own code and test framework choose to do with a failure**, and
that varies enormously by language and by how the loop is written. A flat imperative loop's crash
in Rust carries *zero* variant context by default (finding 1); a parameterized test framework
(Vitest's `it.each`) names the variant for free (finding 4). One real gap surfaced along the way —
the exchange loop itself was completely silent — and was cheap enough to close in this same session
(finding 3). The sampled matrix itself is not the problem: its ids, labels and `origin` are
well-designed and read cleanly on their own (finding 5); nothing currently connects that document
to a failure automatically.

## Findings

### 1. A flat imperative loop's crash carries zero variant context — Rust's baseline experience

Running the reproduction with no per-iteration output at all (the loop body has nothing else to
report until it crashes) gives the *entire* failure:

```
$ cargo run -p pact_janus_kernel --example careless_consumer
selected variants: ["base", "response.body.shippedAt#presence=absent"]

thread 'main' (408264) panicked at engine/kernel/examples/careless_consumer.rs:147:8:
shippedAt should always be present
```

Two variants were selected; one of them crashed. There is no way to tell *which* from this output.
`RUST_BACKTRACE=1` doesn't help either — it names the panicking line, not the data that was in
flight when it panicked:

```
thread 'main' panicked at engine/kernel/examples/careless_consumer.rs:147:8:
shippedAt should always be present
stack backtrace:
   0: __rustc::rust_begin_unwind
   ...
   5: careless_consumer::main
             at ./engine/kernel/examples/careless_consumer.rs:147:8
```

The committed example prints `variant <id>: status=<n>` before each request, which happens to
appear just above the panic (stdout is flushed line-by-line) — but that is the example choosing to
narrate itself, not anything the engine or a real test framework provides. A consumer whose loop
body is otherwise silent (extremely plausible: most test bodies are assertions, not narration) gets
exactly the two lines quoted above and nothing more.

### 2. `RUST_LOG=trace` only helps if the embedding remembers to install a subscriber

The kernel ships only the `tracing` *facade* (`engine/kernel/Cargo.toml`'s own comment: "a
subscriber ... is installed by whatever embeds the kernel"). `Engine::dispatch` already traces
every frame (this session's earlier work), but `careless_consumer.rs` — playing the role of an
in-process Rust consumer, exactly as a native SDK binding would embed the kernel — never installs
one. The result: `RUST_LOG=trace` around the same run changes nothing at all, silently:

```
$ RUST_LOG=trace cargo run -p pact_janus_kernel --example careless_consumer
selected variants: ["base", "response.body.shippedAt#presence=absent"]
variant base: status=200
  shippedAt = 2026-07-30
variant response.body.shippedAt#presence=absent: status=200

thread 'main' panicked at engine/kernel/examples/careless_consumer.rs:147:8:
shippedAt should always be present
```

No error, no warning that `RUST_LOG` had no effect — a developer who reaches for it (a very
reasonable first move, and exactly what this project's own `CLAUDE.md` recommends) gets nothing and
no explanation why. This is specific to in-process embeddings; the subprocess embedding
(`janus-engine`) installs the subscriber itself, so the same knob works there (finding 3).

### 3. The exchange loop itself was completely silent — closed in this session

Before this task, `engine/kernel/src/protocol/exchange.rs` — the background loop plan task 4.5
built, which is the one place that actually knows "this inbound request matched (or didn't) this
variant" — had zero `tracing` calls. Even a consumer that *did* wire up logging on the subprocess
side had nothing from the engine's own matching decision to correlate against.

Fixed directly (`exchange.rs`'s `handle_inbound`): the match/mismatch verdict is now logged with
the handle and variant id (`debug` on a match, `warn` with the mismatches on a failure), and an
arrival with nothing armed at all — a race, or a variant id that never got served — is now named
too (`warn`), where before it vanished into `dispose()` with no trace anywhere. Verified against
the real subprocess:

```
DEBUG pact_janus_kernel::protocol::exchange: request matched the armed variant instance="t-1" handle=i-1 variant=base
DEBUG pact_janus_kernel::protocol::exchange: request matched the armed variant instance="t-1" handle=i-1 variant=response.body.shippedAt#presence=absent
```

This is now the one place a `RUST_LOG=debug`'d subprocess session names the variant a request
belonged to, independent of whatever the consumer's own code does or doesn't print. It does not
close finding 1 or 2 — a consumer's crash still has to go look at a *separate* log stream and
correlate by timing, since nothing hands the variant id to the code that's about to fail — but it
means the information exists at all, which it did not before.

### 4. Rust's parameterized-test macros can't parameterize over a runtime-computed variant list

This project's own convention (`CLAUDE.md`) names `rstest` as the preferred test-parameterization
helper. `rstest`'s `#[case(...)]` (and `test-case`'s equivalent) are attribute macros: every case is
written as a literal at the call site and expanded at compile time. `consumer-session/variants`
selects its variants at *run* time — the sampler decides how many and which, per interaction, per
sampling policy — so there is no way to hand a `Vec<String>` of variant ids to `#[case]` the way
`it.each(variantIds)` takes a runtime array in Vitest, or `@ParameterizedTest`/`@MethodSource` and
`@TestFactory` returning `Stream<DynamicTest>` do in JUnit 5. Rust's built-in test harness has no
mechanism for registering tests dynamically either. A naive Rust consumer (or a straightforward
generated SDK binding) is therefore structurally pushed toward exactly the flat loop finding 1
describes — not by an oversight, but because the obvious idiomatic alternative isn't available in
the language's own testing ecosystem the way it is in JavaScript's or the JVM's.

### 5. A parameterized test framework names the variant for free

The same reproduction, restructured as one Vitest `it.each` case per variant (driving the real
`janus-engine` subprocess, same widened interaction, same careless `body.shippedAt.toUpperCase()`)
fails like this:

```
❯ test/careless.test.ts (2 tests | 1 failed)
  ❯ careless consumer: one it.each case per variant (2)
    × variant response.body.shippedAt#presence=absent 3ms

FAIL  test/careless.test.ts > careless consumer: one it.each case per variant > variant response.body.shippedAt#presence=absent
TypeError: Cannot read properties of undefined (reading 'toUpperCase')
 ❯ test/careless.test.ts:82:38
```

The variant id is right there in the failing test's own name — the *other* variant (`base`) is
reported passing in the same run, with no correlation effort required at all. This is not a
protocol or engine property; it is entirely a consequence of Vitest's (and, per finding 4, likely
JUnit 5's dynamic tests') support for data-driven case registration. The identical reproduction
written as a single `it()` with an internal loop would degrade to finding 1's experience even in
TypeScript — the framework only helps if the SDK-generated test code is structured to use it.

### 6. The sampled matrix itself is well-designed and reads cleanly

`consumer-session/variants`' own result, unmodified, for the interaction both reproductions use:

```json
{
  "variants": [
    { "id": "base", "label": "base", "origin": "base",
      "assignment": [ { "dimension": "response.body.shippedAt#presence", "point": "present" } ] },
    { "id": "response.body.shippedAt#presence=absent", "label": "shippedAt=absent", "origin": "boundary",
      "assignment": [ { "dimension": "response.body.shippedAt#presence", "point": "absent" } ] }
  ],
  "report": { "space": { "size": 2, "exact": true, "dimensions": 1 },
              "strategy": "exhaustive", "algorithm": "janus-ipog-v1", "selected": 2,
              "coverage": { "targets": 0, "covered": 0, "removed": 0, "dropped": 0 },
              "budgets": { "exhaustive-threshold": 8, "max-variants": 50 }, "boundaries": true }
}
```

Read cold, with no prior context: `id` names exactly what varies and how
(`response.body.shippedAt#presence=absent` is legible even to someone who has never seen the shape
language before); `label` gives the same thing shortened for a human; `origin: "boundary"` says
*why* it's in the selection, not just that it is; `report` says the whole space was two variants
and both were taken (`strategy: "exhaustive"`), so there is no ambiguity about coverage. This part
of the design holds up — the matrix is not the sharp edge. The sharp edge is that nothing hands a
consumer this document, or the one entry in it that matters, at the moment something fails.

## Recommendations (RFC drawbacks)

1. **The width of a variant's failure signal depends on the target language's test framework, not
   on anything the protocol guarantees.** This is worth stating plainly in the RFC's drawbacks
   section: a data-driven framework (Vitest, Jest, JUnit 5 dynamic tests) gives correlation for
   free; a framework whose parameterization is resolved at compile time (Rust's `#[test]`, `rstest`)
   structurally cannot, for a variant list that is only known at run time.
2. **The SDK specification (design 2.9) and binding generation (Phase 6) should treat this as a
   requirement, not an afterthought.** For languages with dynamic test registration, generate one
   case per variant, named with its `label`/`id`. For languages without it (Rust), the generated
   harness should wrap each variant's execution (`std::panic::catch_unwind` or equivalent) and
   re-attach the variant id/label to whatever it caught, producing a structured per-variant result
   list rather than letting the first panic take down the whole run anonymously.
3. **The exchange loop's match/mismatch decision should probably become an event, not just a log
   line**, once `events/*` exists (still unimplemented — `protocol/mod.rs`'s own note): a host
   polling events during a run could surface "variant X just failed" while the run is still going,
   independent of whatever the consumer's own code does with the response. Finding 3's `tracing`
   calls are a cheap first step, not a substitute for that.
4. **Document the in-process subscriber trap** (finding 2) somewhere an SDK author will actually
   see it before they burn time on it — a native embedding that doesn't install a `tracing`
   subscriber gets no diagnostic feedback and no error saying so.

## Reproducing

Rust (committed):

```bash
cargo run -p pact_janus_kernel --example careless_consumer            # bare failure
RUST_BACKTRACE=1 cargo run -p pact_janus_kernel --example careless_consumer
RUST_LOG=trace cargo run -p pact_janus_kernel --example careless_consumer   # note: unchanged output — no subscriber installed
```

TypeScript (exploratory; not committed — recreate under `cli/tests/janus-engine-node/` using its
existing `test/engine-client.ts`): submit the same widened order interaction, call
`consumer-session/variants`, then

```ts
it.each(variantIds)("variant %s", async (variantId) => {
  await engine.send("consumer-session/serve-variant", { session, handle, variant: variantId });
  const { body } = await httpGet(baseUrl, "/orders/66");
  const shippedAt = body.shippedAt.toUpperCase(); // careless: assumes presence
  expect(typeof shippedAt).toBe("string");
});
```

against a `janus-engine` spawned per finding 5's `EngineClient`. `RUST_LOG=debug` on the spawned
subprocess reproduces finding 3's log lines.
