# Spike 7.3 — Type-derived shapes (provenance #2)

Plan task 7.3. **The deliverable is [FINDINGS.md](FINDINGS.md)**; the code here is disposable.

## The question

The RFC lists over-broadness as a drawback rather than a footnote: a provider shape derived from
types tends to overstate the real response space — "every field nullable in the ORM ≠ every field
absent in practice" — and a strict policy "would drown teams in findings and teach them to
rubber-stamp". [ADR 0016](../../Documentation/decisions/0016-subsumption-defaults-to-warn-with-mandatory-reason-exemptions.md)
defers one decision to this spike by name: whether `provenance` should be a policy selector.

So: import OpenAPI into provider shapes, run the real checker, and **count**.

## Method

One API — this repo's own [sample provider](../../samples/order-service) — one consumer contract,
and three provider shapes of it, checked by the same engine:

| Shape | Provenance | Where it comes from |
|---|---|---|
| recorded | `recorded` | task 7.2's recorder, against the **live** provider over HTTP |
| derived, hand-authored | `derived` | an OpenAPI document written by someone who knows the API |
| derived, ORM-generated | `derived` | the same API, emitted the way a schema generator that walks an ORM model emits one |

Holding the API, the consumer and the checker fixed isolates the one variable the RFC worries
about: **where the provider shape came from**. A second run applies each generator behaviour to the
hand-authored document *one at a time*, which turns "derived shapes are noisy" into a per-behaviour
cost.

```sh
cd importer
cargo run              # the three-way measurement, with every report rendered
cargo run -- pathologies  # each generator behaviour, applied alone
cargo run -- gaps         # the constructs the mapping cannot carry across
cargo run -- unmapped     # the same derivation with no operation-map file (FINDINGS §2)
cargo run -- derive order-service.orm-generated.openapi.json   # one derived document, to look at
```

## What is in here

- `importer/src/derive.rs` — OpenAPI 3.0/3.1 → provider shape. Every place the mapping loses,
  widens or cannot express something is recorded as a `Gap` rather than silently absorbed; the
  count and kind of those gaps is half of what is being measured.
- `importer/src/main.rs` — the measurement harness.
- `corpus/` — the consumer contract, the two OpenAPI documents, a `mapping-gaps.openapi.json` that
  exercises one construct per mapping gap, and `operation-map.json`, whose *existence* is
  FINDINGS §2.

## Honest limits of the corpus

The two OpenAPI documents are written for this experiment, not harvested from a real service — no
public spec on hand described this repo's own API, and using a different API would have confounded
"derived vs recorded" with "a different payload". Every pattern in the ORM-generated document is
one real generators emit, and `x-note` in each file says which. The measurement is therefore a
controlled comparison rather than a field study, and FINDINGS says so where it matters.

The corpus is also small (one operation, six fields). That bounds the *absolute* numbers, not the
shape of the result: the per-field costs in FINDINGS §4 are linear in field count by construction,
which is the part that generalises.
