---
default: minor
---

# `values`, `eachKey`, `eachValue` and `arrayContains` rules verify as today

A v1–v4 pact whose body used one of these four matching rules used to fail every interaction that had
one: the legacy compiler compiled each to an error. They now compile as today's verifier applies them
(ADR 0042):

- `values` and `eachValue` on an object compare every entry of the reply with the pact's value for that
  key, or the pact's first value for a key it does not give. Keys are not compared. `eachValue` uses only
  its own rules, and on an array holds every element to them.
- `eachKey` holds every key of the reply to its rules, and compares the pact's own members where the reply
  has them.
- `arrayContains` requires each variant to match some element of the reply's array, in any order; one
  element may satisfy two variants. A variant with no rules is its element compared by equality. A
  variant that nothing matched fails with today's message,
  `Variant at index 0 (…) was not found in the actual list`.

Plan grammar v1's `legacy:array-contains`, which no compiler emitted before, now runs a template against
the array's elements instead of comparing one value: its children are the array, the template, and
optionally the variant's index and element. These rules on a path, header, query, form value or XML
element are still refused. `janus upgrade` still converts them approximately.
