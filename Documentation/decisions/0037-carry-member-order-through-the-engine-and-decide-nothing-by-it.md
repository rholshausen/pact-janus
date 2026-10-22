# 0037 — Carry member order through the engine, and decide nothing by it

- **Status**: accepted
- **Date**: 2026-10-22
- **Plan tasks**: S1.20 (the build); S1.21 and S1.22 (the form and XML components that need it)
- **Evidence**: [phase-9 findings](../prototype/phase-9-findings.md) §11; the
  [third-party component report](../prototype/third-party-component-report.md) item 10; the
  [kernel-boundary review](../prototype/kernel-boundary-review.md) finding 5; shape-language spec
  §5.4, §6.2; contract-file spec §2.4; the SDK behavioural specification's `literal` rule

## Context

The engine loses an object's member order. `serde_json` runs without `preserve_order`, so every object
the kernel parses has its members sorted, and the runtime value's objects and an `object` shape's
`members` are `BTreeMap`s. So a mock serves a CSV body with its columns in alphabetical order, whatever
order the consumer declared them in. Nothing fails, because the CSV component's consumers read by
name. A consumer that reads by position has no way to say so, and the mock is serving a different body
from the one the provider sends. The SDKs already keep order: the behavioural specification has a plain
map become an `object` shape "in the order written", and both SDKs build their maps in insertion order.
The engine is where it is lost.

Form and XML bodies (S1.21, S1.22) have order too. A form's repeated values and an XML element's
children are sequences, which the document model's arrays carry. But a mock serving either would also
reorder whatever reached it as an object.

The model cannot simply become ordered, because several things already depend on *not* reading order.
The shape language fixes dimension order, a `one-of`'s default and dimension ids by name, "because JSON
object member order is not preserved by every parser the document will pass through" (shape spec §5.4,
§6.2). A contract's bytes are deduplicated by a broker on a hash (contract-file spec §2.4), and the
canonical writer relied on sorted maps for that. A v1–v4 JSON object is a set of members, and its plan
should not change because a broker or a formatter wrote the pact out again.

Separately, the legacy compiler walks a body as `serde_json::Value` (kernel-boundary review finding 5),
so a document a content component decodes cannot reach it. Form and XML need that too.

## Decision

1. **An object's members keep the order they arrived in**, from wherever a document enters the engine
   (a frame, a pact, a contract, a decode) to wherever it leaves (an encode, a frame, a recorded
   document). `serde_json`'s `preserve_order` is on: the kernel declares it, and every crate that holds
   a document depends on the kernel, so Cargo turns it on for all of them. The runtime value's objects
   are insertion-ordered maps, and so is an `object` shape's `members`.
2. **Order is carried, never compared.** Two objects with the same members are equal in any order.
   Nothing the engine decides depends on order: not a verdict, a variant space, a dimension id, a
   default, or a subsumption finding. Compilers walk an object's members by name, the order shape spec
   §6.2 already gives dimensions. So a plan is the same however its source was written, and the v1–v4
   compiler still compiles a pact's JSON body by name. A content type whose verdict depends on order
   needs an action of its own, contributed by its component (ADR 0022). The core vocabulary does not
   gain one.
3. **A generated document's members are in declared order.** The body a mock serves and the values a
   contract records, which a verifier later sends as a request, all follow the order the shape
   declares its members in. So `content/encode` sees the author's order, and a CSV's columns come out
   as the consumer wrote them.
4. **A recorded document writes the author's order and nothing else's.** In a contract or a provider
   shape, a *record* — the selection report, a variant's assignment, a variant-param binding's own
   members — is written by name, as before. A *shape* is written with its own members by name and an
   `object` shape's `members` in the author's order. A *document* — an example, an `any-of` option, a
   recorded slot's content, a state's parameter values — is written as it arrived. What the kernel
   does not interpret, a component operator's node and a `generator`, is kept as written. So a
   contract whose members were already in name order writes the same bytes as before. A contract
   whose members were not changes once, to say what its author wrote.
5. **The legacy compiler compiles a document, not JSON.** A v1–v4 body is `LegacyBody::Document`,
   holding the runtime value: a JSON document, a text body as a string, a binary body as bytes. The
   compiler walks that value, so a document decoded by a form or XML component reaches it as a JSON
   pact's body does.

## Alternatives considered

- **Keep the model unordered, and declare order loss as a degradation** (finding 11's option B;
  `unordered-members` is already in the vocabulary). It names the problem and leaves it. A mock would
  still serve a body the provider never sends.
- **Give `content/encode` the shape, which carries the author's order** (finding 11's option C, and
  finding 12's fix). That would fix encoding only. It needs a protocol change, and a decoded document
  would still lose order, so `explain --executed` would still show a reordered body. Finding 12 can
  still add it for its own reason: an empty CSV naming its columns.
- **Make order part of equality.** That breaks every JSON consumer whose provider changed serialiser,
  and it is not how today's verifier compares JSON. A content type that means order says so itself.
- **Write everything in a contract as it arrived.** That is simpler, but every contract's bytes would
  change once, and then depend on the order an SDK builds a shape node in. Two SDKs writing the same
  shape would write different bytes, and a change of SDK version would publish a new contract version
  with nothing in it changed.
- **Model order separately, with a sorted map plus a list of names.** That makes the safe default (by
  name) automatic, but every producer of a document would have to keep two structures in step. An
  ordered map with explicit by-name walks costs one helper, `members_by_name`, at the few places that
  decide.

## Consequences

- A mock serves a JSON or CSV body with its members in the order the consumer declared them. The
  third-party CSV component's columns come out as declared, which its tests and the TypeScript SDK's
  CSV consumer now check.
- `explain --executed` and mismatch messages show a captured value in the order it arrived. Five
  corpus snapshots changed for this reason alone, with no plan and no result changing. A new corpus
  case, `shapes/member-order`, shows an example and a body with the same members in different orders
  compared as equal.
- A recorded contract's bytes change only where an `object` shape's members, an example or a
  recorded body was not already in name order.
- Every place that decides by an object's members must walk them by name. Two places did not: the
  subsumption check's "do these two shapes admit the same values" zipped two members maps, and
  `janus upgrade` sampled an `each-entry` map's first member. Both now go by name. A test pins each
  rule (`engine/kernel/tests/member_order.rs`).
- Engine output built from the engine's own records (`--output json`, a run summary) is no longer
  sorted at every depth. It is written in the order the code builds it, which is just as
  deterministic.
- Ordered objects cost something. An ordered map is larger than a sorted one, so a `Value` is too,
  and the two error types that carry one (`ComponentError`, `EngineError`) now box their `details`.
  The protocol does not change. One run of `benchmarks/janus` on one machine found 100 KB documents
  13–22% slower to explain, upgrade, match and verify, and small ones unchanged within noise. The
  Benchmarks workflow's trend will show whether that holds.
- S1.21 and S1.22 can hand the legacy compiler a decoded document.
- **Tripwire**: if a content component needs order to *decide* something, and an action of its own
  cannot express it, then order belongs in the core vocabulary. That would supersede decision 2.
