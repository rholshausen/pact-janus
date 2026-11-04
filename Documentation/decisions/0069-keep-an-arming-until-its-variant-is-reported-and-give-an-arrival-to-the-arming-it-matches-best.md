# 0069 — Keep an arming until its variant is reported, and give an arrival to the arming it matches best

- **Status**: accepted
- **Date**: 2026-11-04
- **Plan tasks**: S2.13
- **Evidence**: engine-protocol spec §8.2 (`serve-variant`: "Multiple interactions may be armed
  concurrently"), variant-semantics spec §4.1–§4.2; [ADR 0064](0064-report-each-variants-test-verdict-to-the-engine-and-answer-with-its-result.md)
  decision 1 (a report ends a variant's run); [ADR 0066](0066-let-a-compatibility-facade-submit-todays-builder-calls-and-convert-them-as-upgrade-does.md)'s
  "Not decided here"; pact-core-mock-server's `match_request` at `a891c2e8`
  (`pact_mock_server/src/matching.rs`) and pact-reference's `RequestMatchResult::score`
  (`pact_matching/src/lib.rs`), which choose an interaction for a request; pact-js's `PactV3` and
  `PactV4`, which serve every interaction added before an `executeTest`; pact-jvm's `@PactTestFor`
  with several `pactMethods`

## Context

The protocol lets several interactions be armed at once, and the kernel arms one. Each transport's
exchange loop holds a single arming, and the first request that arrives takes it: a second request
finds nothing armed and is refused. So the Janus DSL's loop works, one variant served and called at a
time, but a test that sets up several interactions and then runs its client does not.

Today's libraries are written that way. A pact-js `PactV3` test adds its interactions and then calls
`executeTest` once, and a `PactV4` test serves every interaction added since the last `executeTest`.
pact-jvm's `@PactTestFor` takes several `pactMethods`. A client may call one endpoint twice, to page
or to poll, and the mock serves it each time. The facade refuses both until the kernel does them
(S2.5a).

Today's mock server holds every interaction of its pact. For each request it matches every one,
scores each match, and takes the best: the first of the best when several tie. If the best matched
fully, the request is answered with its response. If its method or path did not match, the request
was "not expected"; otherwise the request is a mismatch against that interaction. At the end, an
interaction no request matched is a "missing request". The score counts what matched against what
did not, field by field: method, path, each query parameter, each header, each body value. A more
specific interaction scores higher than a looser one that also matches.

Three things the kernel does not say yet: how long an arming lasts, which arming an arrival goes to
when several are armed, and what an arrival that matches none of them fails.

## Decision

1. **An arming lasts until its variant is reported.** `serve-variant` arms the variant on every
   started transport of the interaction's kind, and it stays armed until `report-variant` reports it,
   the handle is armed again with another variant (which replaces it, as the protocol says), or the
   session ends. Each arrival it takes is one more exchange of that variant. A variant is exercised
   when at least one of its exchanges completed. It is failed when any exchange failed, with every
   failed exchange's mismatches, whatever the others did, as today's mock server fails a test with a
   mismatch in it.
2. **An arrival goes to the arming it matches, the most specific first.** A transport holds every
   arming of its kind, in the order they were armed: an arming that replaces another goes last. An
   arrival is matched against each one's request plan. It goes to an arming it matches, and among
   several to the one whose match passed the most plan actions, then to the first armed. That is
   today's preference for the highest score, the first of equals, measured on the plan rather than
   on HTTP's fields: a plan action is the kernel's unit of a check, and an interaction that checks a
   header the other leaves open passes one more of them. One arming matched is today's behaviour.
3. **An arrival no arming matches fails the one it came closest to.** The closest is the one with
   the fewest mismatches, then the most actions passed, then the first armed. Its variant fails with
   those mismatches, and the transport rejects the arrival as it does a mismatch now (HTTP answers a
   500 naming them). With one arming, this is what happens today. An arrival with nothing armed is
   still refused as `unmatched` and fails nothing, since it proves nothing about any variant.
4. **A report waits for an exchange that could be its variant's.** An arrival is in flight for every
   arming it is matched against until its outcome is recorded, so `report-variant` for any of them
   waits for it, as it now waits for the one (ADR 0064 decision 1).
5. **A facade serves what its library serves.** pact-js's `executeTest`, on `PactV3` and on
   `PactV4`, adds every interaction its library would serve to one session, arms each one's base
   variant, runs the test once, and reports its verdict on each. An interaction the test never
   called is `not-exercised`, so its contract is withheld and the facade names it, as pact-js names a
   missing request. The Janus DSL's `execute` is unchanged: it serves one variant at a time.

## Alternatives considered

- **Score HTTP's fields, as pact does.** The kernel names no method, path or header (S1.11); a
  transport's parts are its own. Counting the plan's actions measures the same thing, what was
  checked and passed, for any transport.
- **Call an arrival that matches nothing "not expected", failing no variant.** pact tells
  "not expected" from a mismatch by whether the method or path matched, which is HTTP knowledge.
  And the contract has to be withheld either way, which needs a failed variant: nothing else the
  engine records withholds it. The closest arming is where pact charges any other mismatch.
- **Fail every armed variant with the arrival.** Every interaction the test called correctly would
  fail too, and the report would say each was wrong.
- **Let the first arrival end an arming, as now, and arm the variant again for a second call.** A
  test cannot know how many times its client calls, and today's libraries never ask it to.
- **Choose by arming order alone.** Two armings that both match, a general one and a specific one,
  would serve the general one's reply to a request the test wrote the specific one for. Today's mock
  server chooses the specific one.

## Consequences

- The facade's `executeTest` runs a test with several interactions, and one that calls an
  interaction more than once, as pact-js runs them. pact-jvm's facade (S2.5b) arms its
  `pactMethods` the same way.
- A Janus DSL test that calls its mock twice under one variant is served twice where its second call
  was refused. Nothing in the behavioural spec asked for the refusal.
- The variant-semantics spec's §4.1 and §4.2, and the protocol's `serve-variant`, say how long an
  arming lasts and what an exchange of it means.
- **Tripwire**: two armings the same arrival matches equally, where the test meant the second. Then
  a test needs a way to say which, and decision 2's "first armed" is the place to change.
