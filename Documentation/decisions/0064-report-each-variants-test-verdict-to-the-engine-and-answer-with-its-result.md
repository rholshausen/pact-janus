# 0064 — Report each variant's test verdict to the engine, and answer with the variant's result

- **Status**: accepted
- **Date**: 2026-11-02
- **Plan tasks**: S2.2; closes the decision backlog's `report-variant` entry
- **Evidence**: Phase 9 findings 4 and 5 ([findings](../prototype/phase-9-findings.md) §4, §5);
  `sdks/typescript/src/janus.ts`, whose `#failedTests` withholds a contract the engine returned;
  `sdks/jvm/sdk`'s `Janus.failed`, the same list kept again; behavioural spec `finalise`, which calls
  it "the one verdict the SDK holds that the engine does not"; `engine/kernel/examples/careless_consumer.rs`,
  task 4.6's client that crashes when `shippedAt` is absent while every exchange verifies;
  `engine/kernel/src/protocol/exchange.rs`, where an exchange's outcome is recorded after its reply is
  handed to the transport, so a client can hold the response before the engine holds the outcome

## Context

The engine verifies each exchange: the request the consumer sent matched the armed variant, and the
reply reached it. It cannot know whether the consumer then handled the reply. That is the test's
verdict, and today only the SDK holds it.

- **Finding 4: the SDK withholds a contract the engine returned.** Both SDKs keep a list of the
  executes that failed, and `finalise` writes nothing when it is not empty. That is correct, and it is
  decided twice, in two languages. An SDK that forgets writes a contract claiming variants its own
  tests failed on, with no error anywhere.
- **Finding 5: the engine's verdict arrives at the end of the suite.** `consumer-session` reports
  nothing before `finalise`. A request the mock could not match, or a variant whose closure never
  called the mock, fails the suite's `afterAll`, not the test that caused it. The mock's 500 usually
  fails the test anyway. A client that swallows errors, or a closure that skips a variant, does not.
- **Both lists hold more than closures.** An `add-interaction` the engine refused (`interaction-invalid`)
  also withholds the contract in both SDKs, so a suite with an invalid interaction does not write a
  contract without it. The engine knows it refused one, and keeps no record of it.
- **An outcome lands after its reply.** The exchange loop answers the request and then records what
  happened. A closure that awaited the response can finish before the outcome exists, so anything
  that asks for a variant's result when its closure ends has to wait for an exchange still under way.

## Decision

1. **A new operation, `consumer-session/report-variant`.** Body `{ session, handle, variant, status,
   reason? }`. `status` is the test's verdict on that variant, `passed` or `failed`. Any other value
   counts as `failed`, so an unknown verdict never verifies anything. `reason` is what the test failed
   with, as text. The engine records it and never reads it. It answers `{ result }`, the variant's
   result as `finalise` would report it now (decision 3).

   A report ends the variant's run. The engine waits for an exchange under way for that variant to be
   recorded, then disarms the variant if it is still armed, so a later request is answered as one with
   nothing armed. The latest report for a variant is its verdict. A variant that is not in the
   interaction's selection is `variant-not-found`, as for `serve-variant`. Reporting a variant that was
   never served is allowed, and its result says `not-exercised`.
2. **The result says whose verdict it is.** A variant result gains `test: { status, reason? }` once a
   report arrives. The variant's `status` is, in order: `failed` when the test failed; `failed` when
   the exchange failed, with the engine's mismatches; `not-exercised` when nothing exercised it;
   `not-reported` when reports are required and none arrived (decision 4); otherwise `verified`. An
   interaction's status is `failed` if any of its variants failed, else `not-exercised`, else
   `not-reported`, else `verified`. The contract is still present only when every interaction is
   `verified`, so a failed test withholds it in the engine.
3. **One call answers both findings.** The SDK reports every variant it ran, as soon as its closure
   ends, and reads the answer. When the result is not `verified`, that variant failed, and `execute`
   names it with the closure's cause, the engine's mismatches, or both. A closure that never called the
   mock fails its own variant with `not-exercised`. The SDK still decides nothing: it reports what the
   test did, and fails on the engine's status.
4. **A capability, `test-verdicts`, declared by both sides** (engine-protocol spec §5.3), value `{}`.
   The engine declares that it has `report-variant`. A host declares that it reports every variant it
   runs, and asks the engine to hold the suite's whole verdict. In a session such a host creates:
   - an exercised variant with no report is `not-reported`, which withholds the contract. An SDK that
     forgets to report fails closed, naming the variants, instead of writing a dishonest contract.
   - an `add-interaction` the session refused withholds the contract too. `finalise` lists each refusal
     as `refused: [{ description?, code, message }]`, the description being the submitted document's
     when it had one.

   A host that did not declare it gets protocol version 1's results, as today. It may still report, and
   a reported failure still withholds the contract. The node test client, the benchmarks and the
   kernel's own tests are such hosts and keep working unchanged.
5. **The SDKs' withholding is deleted.** Both declare `test-verdicts`. `execute` reports after each
   closure and fails on the result, and an engine error from `report-variant` ends the loop as one from
   `serve-variant` does (ADR 0019). `finalise` writes the contract when the engine answered one and
   otherwise fails, naming the interactions the results did not verify and the ones the engine refused.
   The TypeScript `ContractWithheldError` loses `failedTests` and `engineWithheld`, and the JVM
   `ContractWithheldException` loses `failedInteractions` and `engineWithheld`: the engine withheld
   every contract an SDK does not write. JUnit's per-variant dynamic tests report too, so a variant the
   engine failed fails its own dynamic test.
6. **What never reached the engine is not in the contract.** A chain the DSL refuses at the call
   (ADR 0019) sends nothing, so the engine cannot withhold for it. The contract then claims nothing
   about it, and the test that built it has already failed. The honesty rule (contract-file spec §2.2)
   is about what a contract claims, and it still holds.

## Alternatives considered

- **A `failed` member on the next `serve-variant`.** It reports the previous variant only when there is
  a next one, so the last variant of every execute needs another call anyway, and nothing comes back
  per variant.
- **A separate `consumer-session/results` operation, beside the report.** Two calls per variant, where
  the report's answer is exactly what the second would return.
- **Consumer-session events on the poll stream** (engine-protocol spec §9). A stream says what the
  engine saw, but the SDK needs the result at one point, after its closure, and would have to poll
  until the outcome of an exchange it cannot name had arrived.
- **Require reports from every host.** Every existing host that does not report would stop getting
  contracts. A capability keeps protocol version 1's answer for them (§5.2).
- **Leave refused interactions to the SDK.** Then the SDK keeps a list of failed executes after all,
  for the one case the engine knows about as well as the SDK does.

## Consequences

Easier: the honesty rule is enforced once, in the engine, and an SDK that forgets a report gets
`not-reported` instead of a contract. A variant fails in the test that ran it, whether the mock
rejected its request, its closure never called the mock, or its closure threw. A third SDK reports a
verdict and reads a status, and keeps no list.

Harder: every variant costs one more round trip. A closure that never calls the mock used to pass its
`execute` and fail `finalise`; now its `execute` fails. Two conformance cases that pinned a variant
space without exchanging anything no longer expect `execute` to pass.

Committed to: `consumer-session/report-variant`, the `test` member and the `not-reported` status of a
variant result, `finalise`'s `refused`, and the `test-verdicts` capability.

Tripwires:

- **A host runs variants in parallel** (variant-semantics spec §4.3). Reports are per variant, so they
  still work, but the wait in decision 1 assumes one exchange per variant at a time.
- **Protocol version 2.** Requiring reports becomes the only behaviour, and the capability is dropped.
