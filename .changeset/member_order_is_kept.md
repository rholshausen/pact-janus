---
default: minor
---

# An object's members keep the order they were written in

The engine used to sort every object's members, so a mock served a CSV body's columns, or a JSON
body's members, in alphabetical order whatever order the consumer declared them in. It now keeps
the order they arrived in (ADR 0037). Order is never compared: two objects with the same members are
equal in any order, so no verdict changes.

- **A mock serves members as the shape declares them**, and so does every document generated from a
  shape: the values a contract records, which a verifier sends as a request. A content component's
  `content/encode` is handed the author's order. A decoded body keeps the order the provider sent.
- **Plans, variant spaces and subsumption go by name, as before.** A plan does not change when a
  shape or a pact is written in another order. Dimension ids, the variants selected and every
  subsumption finding are unchanged.
- **`explain --executed` and mismatch messages show a captured value in the order it arrived**, not
  sorted.
- **A contract's bytes change only where its author's order was not alphabetical.** The canonical
  writer sorts the records and a shape node's own members by name as before. It keeps an `object`
  shape's `members`, examples, `any-of` options, recorded slot content and state parameter values
  as written (contract-file spec §2.4). A contract re-recorded with this engine is a new version at a
  broker only if one of those was out of name order. Provider shapes follow the same rule.
- **Engine output built from its own records** (`--output json`, a run summary, the `variants`
  result) is no longer sorted at every depth. It is just as deterministic.
- The v1–v4 compiler compiles a body as a document rather than as JSON, which is what lets form and
  XML bodies reach it later. It changes no plan.
