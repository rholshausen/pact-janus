---
default: minor
---

# Display text can carry segments, and `janus` colours from them

Staged plan S1.34, ADR 0054.

- **The `segments` capability.** The engine now declares it in `engine/hello`. A host that declares
  it too gets each display string with a `<member>-segments` sibling: pieces whose texts, joined,
  are exactly the string, each with a `kind` such as `pass`, `fail`, `path`, `string` or `action`.
  A kind says what the piece is, never how it looks. Without the host's declaration, nothing on the
  wire changes. See engine-protocol spec §2.7 and `display.schema.json`.
- **Members that carry segments:**
  - `verification/explain`'s `text` and `executed.text`;
  - the `verification/executed-plan` event's `text`;
  - `subsumption/decide`'s `text`;
  - each mismatch's `message`, in explain's result and in `verification/interaction-result` events,
    with the values it quotes marked.
- **The bindings** gain `Segment`, `SegmentsCapability` and a typed `Mismatch` for explain's
  `executed.mismatches`.
- **`janus`** colours plans, the `check` page and the values in mismatch messages from segments,
  where it used to parse the text. Its `--output json` documents carry no segments.
