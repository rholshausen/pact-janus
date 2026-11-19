# 0101 — A slot that cannot be decoded fails at the slot, a decode's losses are warnings, and encode is given the shape

- **Status**: accepted
- **Date**: 2026-11-20
- **Plan tasks**: S5.3
- **Evidence**: Phase 9 findings [12, 13 and 14](../prototype/phase-9-findings.md#12-contentencode-is-handed-a-document-but-not-the-shape-it-was-generated-from);
  the [third-party component report](../prototype/third-party-component-report.md) §2.2, findings 11–13;
  [ADR 0010](0010-plans-are-renderings-the-grammar-is-the-record.md), `engine/kernel/tests/decoding.rs`, `engine/component-host/tests/csv_component.rs` (§ content that is
  not JSON), `corpora/shapes/undecodable-slot/`, `corpora/legacy/v3-undecodable-body-mismatch/`;
  [ADR 0020](0020-a-content-slots-type-is-declared-on-the-interaction.md),
  [ADR 0036](0036-read-text-bodies-through-a-text-component-and-unclaimed-bodies-as-bytes.md),
  [ADR 0043](0043-check-a-pacts-content-type-once-on-the-header.md),
  [ADR 0079](0079-keep-each-likes-minimum-at-one-and-exercise-a-list-that-may-be-empty-with-an-element-first.md),
  [ADR 0100](0100-a-content-component-says-what-a-value-operator-means-for-its-type-and-the-engine-compiles-every-slot.md)

## Context

Task 8.1 built the first content component that is not JSON, `third-party/janus-csv`, and found three
places where the content interface works for JSON only because JSON almost never fails:

- **Finding 12: `encode` never sees the shape.** An `each-like` with `min: 0` has an empty variant
  (ADR 0079), whose document is `[]`. An empty CSV document names no columns, so the component wrote an
  empty body, which its own `decode` refuses ("no header row"). The mock served a body its own contract
  calls malformed.
- **Finding 13: a wrong content type is reported as the wrong size.** A provider answering
  `application/json` where `text/csv` was declared is decoded as declared (contract-file spec §5.5), and
  the JSON parses as a one-line CSV with no rows. The run says `Expected at least 1 item(s)`, which is
  true and sends the reader to the wrong place.
- **Finding 14: a component's errors and degradations never reach the user.** `wire::decode_slot` turned
  a decode error into an absent value, so a ragged CSV row became "expected a value", and a component that
  trapped did too, where component-interfaces spec §11.3 says `component-failed`. Per-decode degradations
  were logged at `debug` and dropped, where spec §6.4 says an engine SHOULD surface them.

ADR 0100 decided that decoding is not a plan step, and left to S5.3 where a decode failure and a
degradation are shown. Member order, finding 11, was S1.20's (ADR 0037).

## Decision

1. **`content/encode` is given the slot's shape.** It has an optional `shape` member: the slot's shape as
   the interaction declares it, the form `content/compile` is given, whenever the engine produced the
   document from one (a mock's reply, a verifier's request, a message a session delivers). It is absent
   otherwise. A component writes from it only what the document cannot say, such as the header of a CSV
   document with no rows. It never changes what the document is, so `decode(encode(d))` must still equal
   `d`, and an `encode` without a `shape` must still work. The CSV component writes an empty document's
   header from its rows' object shape, in declared order, so the empty variant is served as
   `id,status,items\r\n`, which decodes back to `[]`.

2. **A slot that could not be decoded has no document, and fails at the slot.** Component-interfaces spec
   §6.1 already says a shape never applies to a guess. Now the plan says so. The resolver marks the slot
   undecodable, with the component's message, and the interpreter does not execute a node whose value
   under test is read from it. The innermost container holding every node that reads the slot reports it
   instead: its result is an `error` at the slot's path, `could not be decoded as 'text/csv': …`, once
   however many nodes read it (plan-grammar spec §2.4). Nodes that read something else still run, so a
   v1–v4 body's check of its `Content-Type` header is still reported beside it. `match:content-type` reads
   a slot's octets and labels, not its document, so it runs too. The mismatch has no action, since no
   action failed. The executed form shows the container's error, and its unexecuted children with no
   result. A corpus case can now say a slot was undecodable (`undecodable` in `case.json`).

3. **No octets are no document.** An empty slot whose component cannot decode it is absent, as before, and
   not undecodable. An HTTP reply always has a body slot, so an empty body labelled `application/json`, a
   204's say, would otherwise fail every interaction whose shape admits no body.

4. **A component that died is `component-failed`, not a mismatch.** A decode error the binding
   synthesised (`source: engine`: `component-trapped`, `component-timeout`, `component-exited`, spec §11.2)
   is the component's failure, not the provider's. A verification fails the variant with
   `component-failed`, naming `content/<type>` and the slot, carrying the binding's error verbatim, as a
   transport's failure already does. An error the component produced is its verdict on the octets, and
   decision 2 applies. On the consumer side an arrival has no error of its own to carry, so the slot is
   undecodable there too, its message saying the component failed.

5. **What a decode says beside the document is a warning.** A verification emits, before the result it
   qualifies, a `verification/warning` with:
   - `content-degraded` for each degradation a decode reported (spec §6.4): `target`, the slot's path then
     the path the component gave inside its document (`$.response.body[*].items`), `content-type`, the
     component's `degradation` as it gave it, and a `message`;
   - `content-type-differs` for a slot the contract declares one type for that arrived labelled with
     another (`declared`, `labelled`), compared by essence as a component's `media-type` is. A
     declaration is a decode instruction, not a constraint (contract-file spec §5.5), so this changes no
     verdict, but it tells the reader of a size mismatch why the document was empty. A v1–v4 pact's type
     is checked on its header (ADR 0043), so a pact's replay is not told it twice.

   A warning never changes a verdict. The CLI and both SDKs already print a warning by its code, `target`
   and `message`, so neither changes.

## Alternatives considered

- **Pass the shape in `options`** (finding 12's option B). Rejected: `options` is the interaction author's
  channel to the component, and the engine writing into it would make the two collide.
- **Declare an empty CSV a degradation and tell authors to avoid `min: 0`** (option C). Rejected: ADR 0079
  made `min: 0` the honest default for a list that may be empty, and the mock would still serve a
  malformed body.
- **Fail a slot labelled with another type than declared** (finding 13's option A, a `check:content-type`
  at the top of a declared slot). Rejected: contract-file spec §5.5 makes the declaration a decode
  instruction, a consumer that cares about the label shapes the header (`http:media-type`), and an
  upgraded pact already checks it there. A failure would double-report every such pact.
- **Report decode outcomes outside the plan** (finding 14's option B, beside `mismatches` only).
  Rejected for failures: the plan is the record (ADR 0010), and `--explain-failures` should show why the
  body's checks did not run. Degradations do not fail anything, so they are warnings, not plan nodes.
- **Keep running the slot's nodes against an absent value, and suppress their mismatches.** Rejected: the
  executed plan would still show `Expected at least 1 item` against a body that has no document.
- **A core `content:decode` action.** Rejected as ADR 0100 rejected it: decoding is not a plan step, and
  the container's error says everything a node would.

## Consequences

- An empty CSV variant round-trips, and the CSV consumer's lists can be `min: 0`.
- A body a component refuses reads as the component's message at the body, once. Every body kind gets
  this, JSON included: a provider answering an HTML error page where JSON was declared now fails with
  `could not be decoded as 'application/json': expected value at line 1 column 1` at the body, where it
  failed with a mismatch per member. The verdict is the same, so the S1.1 harness agrees.
- A content component's per-decode degradations are visible in every `janus verify` and SDK report.
- The consumer side does not report degradations or relabelled slots of the requests it receives: those
  requests are the consumer's own client's, and a mismatch already fails the test that sent one. `explain`
  does not print a component's declared degradations either. Neither is planned; a user asking for them is
  the evidence to add them.
- `matcher/apply`'s errors still land as the failing node's error, not `component-failed`: a plan node
  has no engine error to carry (`contributions.rs`), and a dead matcher fails the variant either way. A
  component that traps in `apply` is the case that would change this.
- `content/encode` gains a member. The interface is not yet declared stable (S5.1d), and the member is
  optional on both sides.
