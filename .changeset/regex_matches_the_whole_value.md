---
default: patch
---

# A regex in a v1–v4 pact matches the whole value, as it does in today's verifiers

Janus matched a pact's regex anywhere in the value, so `\d{4}-\d{2}-\d{2}` accepted
`2026-07-30 or later`, and `PENDING|SHIPPED` accepted `NOT SHIPPED`. Today's verifiers match it
against the whole value (pact-reference and pact-jvm both do), and now Janus does too:

- **Verifying a v1–v4 pact**, the pattern is compiled anchored, `^(?:…)$`, and `explain` and the
  executed plan print it that way. A provider that passed only because a pattern matched part of a
  value now fails, as it does under today's verifier.
- **`janus upgrade`** writes the anchored pattern into the contract, so a converted pact keeps its
  verdicts. A contract upgraded by an earlier release still has the unanchored pattern, which a
  shape matches anywhere; upgrade it again to get the pact's behaviour.

A regex in a shape (`"shape": "regex"`) is unchanged: it matches anywhere, and an author anchors
it with `^` and `$` (shape-language spec §4.2).
