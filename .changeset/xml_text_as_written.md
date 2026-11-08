---
engine: major
---

# A contract's XML text is its text as written

The built-in XML component's document gives an element's `#text` as written: every text node joined,
a CDATA section as its contents and a reference as its character, nothing trimmed (component-interfaces
spec §6.1, ADR 0075, staged plan S2.10). So a contract that declares `<to>  Ada  </to>` matches only
text with that whitespace around it, and its mock serves the whitespace. An element whose text only
indents its child elements has no `#text`, so a pretty-printed body and a compact one decode alike.

Beside it, every element now has `#trimmed`, the text as today's verifier reads it: each text node
trimmed, then joined. The v1–v4 compiler compares `#trimmed`, so no pact's verdict moves, though its
plans and mismatches name `#trimmed` where they named `#text`. `janus upgrade` shapes an XML body's
text as `#trimmed` too. A contract it wrote before this release shapes `#text`, which whitespace
around a reply's text now fails: upgrade the pact again. `encode` writes `#text`, or `#trimmed` when an
element has no `#text`, and writes a CDATA section's contents as escaped text.
