# 0077 — Match a multipart part's content type against its own label

- **Status**: accepted; extends [ADR 0049](0049-match-a-content-type-against-the-arrivals-own-octets-and-labels.md)
  (from a whole slot to a member of its document) and the content interface (component-interfaces
  spec §6, an optional operation)
- **Date**: 2026-11-09
- **Plan tasks**: S2.20
- **Evidence**: pact-js's multipart example (`sdks/typescript/test/examples/multipart.test.ts`) and
  `conformance/cases/facade/pact-js-v3`, whose multipart interactions converted with `body-not-parsed`;
  `engine/kernel/tests/multipart_part_types.rs`; today's verifier's `get_multipart_boundary` and
  `match_file_part` (`pact_matching/src/binary_utils.rs`)

## Context

`pact_ffi` writes a multipart body's type as `multipart/form-data`, without the boundary its
`Content-Type` header names. The legacy reader split a body at the boundary its *body's* type names, so
it could not read one `pact_ffi` built. `janus upgrade`, and both facades, which convert a pact as it
does, wrote such a body with `body-not-parsed` and asserted nothing about its parts. Today's verifier
reads a multipart body's boundary from the header (`get_multipart_boundary`).

Reading the header's boundary is not enough on its own. `pact_ffi` puts a `contentType` rule on each
file part (`$.file`), which `upgrade` writes as a `content-type` shape on the part's body. On the
contract path `match:content-type` reads a whole slot's declared or labelled type (ADR 0049), but a
part's body is a member of the slot's document, so its type was detected from the value as it stands.
Text is not detected from octets (component-interfaces spec §6.5): any UTF-8 is plausibly text. So a
`text/plain` part that pact-js's mock accepts was refused, and pact-js's own multipart example failed.

A part's type is known: the multipart component decoded its body by the part's own `Content-Type`
(ADR 0040 decision 3). But the kernel knows nothing about multipart, so it cannot read that header
from the document itself.

## Decision

1. **A multipart body whose type names no `boundary` is split at the one its `Content-Type` header
   names**, when the header is the same type with one, as today's verifier splits it. The legacy reader
   does this, so `upgrade` and the facades shape the parts. The declared type stays without its boundary
   (ADR 0047), and the multipart component already took a missing boundary from the slot's label.
2. **The content interface gains `content/label`**: `{ content-type, value, at }` → `{ media-type? }`.
   It asks the component that decodes a slot what the member at `at`, a JSON pointer into the decoded
   document, is labelled. It is optional: a component whose documents carry no labels of their own
   answers `operation-unsupported`, which means the member has none. The built-in multipart component
   answers a part's `Content-Type` for its `body` (`/<name>/<index>/body`), and nothing for any other
   member or for a part with no type. The registry routes it as it routes `decode`.
3. **On the contract path, `match:content-type` on a member of a slot reads the member's label first**,
   as it reads a whole slot's (ADR 0049 decision 3), and detects from the member as it stands only when
   it has none. A part labelled `text/plain` is `text/plain`; a part labelled `text/plain` whose content
   looks like JSON is not `application/json`. Types are still compared by essence.
4. **The v1–v4 path is unchanged**: a pact's `contentType` matcher sniffs the octets whatever their
   label (ADR 0049 decision 4).

## Alternatives considered

- **Have the text component detect UTF-8 as `text/plain`, last in the registry.** It is what today's
  verifier does (`tree_magic_mini` answers `text/plain` for text). But detection would then claim every
  body no other component recognises, and it would answer on the contract path where a label should, so
  a part labelled `application/octet-stream` whose bytes happen to be UTF-8 would be `text/plain`.
- **Have `upgrade` write a part's `contentType` rule as a check on the part's `content-type` header.**
  It needs no new operation. But it moves the check from the body to the header, so a part with no type
  of its own would pass whatever its content, and it changes what every upgraded contract says rather
  than how a `content-type` shape is read.
- **Teach the kernel ADR 0040's document**, reading `headers.content-type` beside a part's `body`. It
  needs no new operation either, but it puts multipart knowledge in the kernel, which only
  `engine/kernel/src/legacy/` may hold, and only for v1–v4.
- **Put the label in the shape**, a `content-type` shape naming where its label is. It makes a contract
  say something about a content component's document that the component already knows.

## Consequences

- pact-js's multipart example and `facade/pact-js-v3`'s upload record their parts and run, and
  `multipartBody` and `PactV3`'s multipart uploads are `kept`.
- `content/label` is an additive operation, within version 1 of the component protocol (§11.2). An
  out-of-tree component that does not implement it answers `operation-unsupported`, and its members are
  detected as before. A `content-type` shape on a member of a slot whose component does not label costs
  one extra call, answered with that error.
- The multipart component decodes the slot again each time it is asked for a label: once per
  `content-type` shape below the slot, and `pact_ffi` writes one per file part.
- On the v1–v4 path, a multipart reply's text part with a `contentType` rule still fails, where today's
  verifier sniffs `text/plain`. No fixture has one: `pact_ffi` builds request bodies, which a provider's
  verification does not match. ADR 0049's tripwire, a built-in detector for the types today's verifier
  sniffs, would close it.
- **Tripwire**: if another component's documents need labels (an XML element's `xsi:type`, an embedded
  document in a message), it answers `content/label` with pointers into its own document, and nothing in
  the kernel changes.
