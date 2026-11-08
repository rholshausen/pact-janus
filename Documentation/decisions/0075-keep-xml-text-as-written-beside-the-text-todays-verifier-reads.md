# 0075 — Keep XML text as written, beside the text today's verifier reads

- **Status**: accepted; amends [ADR 0039](0039-read-xml-bodies-by-element-name-through-a-built-in-xml-component.md)
  decision 2
- **Date**: 2026-11-08
- **Plan tasks**: S2.10; S1.25 (`janus upgrade` of an XML body, which now shapes `#trimmed`)
- **Evidence**: `sxd-document` 0.3.2's parser (`DomBuilder::consume`, which makes a text node of each
  run of character data, each CDATA section and each character or entity reference, and its tests
  `element_with_entity_reference` and `element_with_cdata`); `pact_matching::xml`'s `compare_text`,
  which trims each text node and joins them; XML 1.0 §2.4 and §2.7 (character data and CDATA sections);
  the XML Infoset, in which a CDATA section's boundaries are not information; S1.1's XML fixtures and
  the legacy harness's 156 XML specification cases

## Context

ADR 0039 decision 2 gave an element's `#text` as today's verifier reads it: each text node trimmed,
then joined. A Janus contract inherited that reading. So a contract that declares `<to>  Ada  </to>`
cannot be matched by the body its own mock serves, since the reply's `#text` is `Ada`. Whitespace that
XML counts as character data is lost, and so are the literal contents of a CDATA section: `sxd-document`
reads `<![CDATA[ x < y ]]>` as one more text node, which is trimmed too.

The reading is odder than "trimmed". `sxd-document` starts a new text node at every reference, so
`<a>1 &lt; 2</a>` is three nodes (`1 `, `<`, ` 2`) and today's verifier reads it as `1<2`. A v1–v4
verdict depends on that reading, the pact's side and the reply's alike, so the legacy compiler must keep
it exactly. It needs the node boundaries, and text joined as written has lost them.

What a contract's text means has to settle before contracts with XML bodies are published to a broker,
since changing it later changes what every published contract admits.

## Decision

1. **`#text` is an element's text as written**: every text node joined, nothing trimmed. A CDATA
   section counts as its contents and a reference as its character. It is absent when it is empty.
2. **`#trimmed` is the text as today's verifier reads it**: each text node trimmed, then joined, where a
   CDATA section and each reference are nodes of their own. It is absent when it is empty. The
   component writes it into every element it decodes, beside `#text`, from the same parse.
3. **Whitespace that only indents child elements is not text.** When an element has child elements
   and its text is only whitespace, it has no `#text`. A pretty-printed document and a compact one
   decode alike, and `encode` writes an element's text before its children, so indentation could never
   be served where it was written. An element with no children keeps whitespace-only text as written.
4. **The legacy compiler compares `#trimmed`**, where it compared `#text`, and the legacy reader of a
   pact's body writes both members, as the component does. A pact's rules still name the text
   `['#text']`. Since `#trimmed` is exactly the old `#text`, no v1–v4 verdict moves.
5. **`janus upgrade` shapes `#trimmed`**, as it shaped `#text`, so whitespace around a pact's text
   counts in its contract no more than it did in its pact. A pact's generator on text fills the
   element's `#text`, which `encode` writes. Upgrade adds a `#text` for it to fill: optional and
   admitting any text, so it constrains nothing the `#trimmed` beside it does not.
6. **`encode` writes `#text`, or `#trimmed` when an element has no `#text`**, as an upgraded contract
   has. When an element has both, `#trimmed` is ignored.
7. **`encode` writes a CDATA section's contents as escaped text.** A CDATA section is not part of what
   an XML document says: every XML reader reads `<![CDATA[a < b]]>` and `a &lt; b` as the same
   characters. So the document records no CDATA, and the component keeps `sxd-document`, which does
   not report one.

## Alternatives considered

- **`#text` as written, plus a list of the parser's text nodes for the legacy compiler.** The document
  stays smaller, and a new legacy action trims and joins the list. But `janus upgrade` would have to
  shape `#text` so that whitespace still does not count: each equality and pattern on text becomes a
  pattern padded with `\s*`, and a pact's empty text becomes a whitespace pattern in place of
  `forbidden`. That pattern cannot reproduce trimming at a reference, so an upgraded contract could
  disagree with its pact where `#trimmed` cannot.
- **Keep `#text` as today's reading and add a member for text as written.** Nothing in the legacy
  layer or the corpus changes. But a contract's natural member would stay the one its mock cannot
  serve, which is the defect S2.10 exists to fix.
- **Keep indentation as written.** Strictly as written, but every parent element of a pretty-printed
  reply would carry a `#text` such as `"\n  \n"`. That text is not where it was written, and no
  contract means to compare it.
- **Record CDATA sections and write them back.** A mock's body would then look like the consumer's
  example byte for byte. That needs a second parser to find CDATA (quick-xml or xml-rs), since
  `sxd-document` does not report it. It also needs a member a contract author has to know about, and
  it carries a distinction no XML reader can see. `sxd-document` stays because today's verifier
  parses with it, so the two accept and reject the same documents (ADR 0039).

## Consequences

- A contract's XML text matches only text written the same way: whitespace around it counts, and a
  CDATA section matches text that is its contents as written, however that text is written.
  `corpora/shapes/xml-text-as-written/` and its `-mismatch` twin show this. A contract that does not
  care about whitespace around text shapes `#trimmed`.
- Every decoded element with text carries both members, so an executed plan or a mismatch that shows
  an element shows the text twice.
- The legacy compiler's plans and mismatches name `#trimmed` where they named `#text`, and an upgraded
  contract's XML shapes are over `#trimmed`. A contract `janus upgrade` wrote before this compares the
  member that is now as written, so whitespace around a reply's text fails it. Upgrade the pact again.
- A CDATA section's contents with `<` or `&` in them, written back escaped and read again, have the
  same `#text` and a different `#trimmed`, since the reference starts a new node. Only the legacy
  reading sees the difference, and nothing compares a mock's own output by it.
- The engine change is `major`: what a contract's `#text` admits has changed.
- **Tripwire**: if a pilot's consumer compares XML bodies byte for byte, recording CDATA is the
  additive change that would follow. quick-xml would find the sections in a document `sxd-document`
  has accepted. Nothing else in this decision changes.
