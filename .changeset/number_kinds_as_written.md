---
engine: patch
---

# `integer` and `decimal` read a number's kind from how it is written

A number is an integer when it is written with no fraction or exponent and fits in a signed or
unsigned 64-bit integer, and a decimal otherwise (ADR 0072, staged plan S1.38), as today's verifiers
read a v1–v4 `integer` or `decimal` rule. `match:integer` and `match:decimal`, so the shapes
`integer` and `decimal`, now refuse `100.0` and `100` respectively, where they decided by value and
admitted them. A v1–v4 pact verifies as today's verifier verifies it, and so does the contract
`janus upgrade` writes from it, and a mock built from either accepts what today's mock does. Equality is
unchanged: `100` and `100.0` are one value. Subsumption's value membership and the provider-shape
recorder read the same classification, so a recorded `100.0` is a `decimal`. A provider whose
verification relied on the old reading, answering `100.0` to an `integer` shape, now fails it.
