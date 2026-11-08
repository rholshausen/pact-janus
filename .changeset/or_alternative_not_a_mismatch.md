---
engine: patch
---

# An `OR` that passes is not failed by an alternative it did not need

A v1–v4 `combine: "OR"` passed by a later alternative failed `janus verify` whenever an earlier one
failed, where today's verifier passes it: the alternative that failed was reported as a mismatch,
though the `OR` succeeded (staged plan S2.18). It is kept in the executed plan, which `explain
--executed` shows, and is no longer a mismatch; an `OR` none of whose alternatives passes still
reports each one's. The same held for an XML element's `notEmpty` rule, satisfied by the element's
text when it has no children, which now passes.
