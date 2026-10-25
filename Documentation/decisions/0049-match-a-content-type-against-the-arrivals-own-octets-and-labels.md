# 0049 — Match a content type against the arrival's own octets and labels

- **Status**: accepted
- **Date**: 2026-10-25
- **Plan tasks**: S1.10
- **Evidence**: `cli/tests/cli.rs`'s `a_content_type_shape_is_matched_against_the_replys_octets_through_janus_verify`,
  which runs `corpora/shapes/content-type`'s shape through `janus verify`;
  `engine/kernel/tests/legacy_verification.rs`'s `a_content_type_matcher_sniffs_the_replys_octets`;
  the third-party component report §4; kernel-boundary review finding 1

## Context

`content-type` is the one shape operator that inspects octets rather than a decoded document
(shape-language spec §4.2), and `match:content-type` asks a content component's `detect` for the answer
(component-interfaces spec §6.5). The interpreter took a single detector, which only the corpus runner
and the tests passed. Since plan task 8.1, decoding and encoding are routed through the session's content
registry, but detection was not, so in a live run every `content-type` shape failed with "no content
component is loaded".

Handing the interpreter the registry is not enough on its own. By the time a plan resolves
`$.response.body`, a reply labelled `application/json` has been decoded into a document, and its octets
are gone. And component-interfaces spec §6.5 says detection resolves an absent type and must not
override a declared one. A v1–v4 `contentType` matcher works differently: today's verifier sniffs the
body's octets whatever they are labelled.

## Decision

1. **`match:content-type`'s detector is the session's content registry**, which asks each of its
   components in registration order, a declared component's before the embedding's (the registry's own
   `content/detect`). The adapter, `ContentDetection`, is the only detector the live paths and the corpus
   runner use. The JSON component no longer carries a detector of its own.
2. **The detector is told the path a value was resolved from.** A path naming a whole slot
   (`$.response.body`) is read from the arrival's own parts, where its octets and its label still are.
   Any other value, such as a document's member or text with no slot behind it, is detected as it stands.
3. **On the contract path, a slot's type is the one the interaction declares for it, else the one it
   arrived labelled with, and detection answers only for a slot with neither.** A reply that labels JSON
   octets `text/plain` fails a `content-type` of `application/json`: its declared type is the finding,
   not something to correct (§6.5). The mock matches an inbound request the same way.
4. **On the v1–v4 path, only detection answers**, from the octets, whatever their label, as today's
   verifier's `contentType` matcher does.
5. **Types are compared by essence**, with `media_type_matches`: case and parameters aside, and
   `text/*` naming every text type.

## Alternatives considered

- **Detect on the decoded document.** It needs no path, but a decoded XML or form document is a JSON
  object, and the JSON component would answer `application/json` for it.
- **Keep a slot whose shape is `content-type` undecoded.** It keeps the octets, but it needs the
  resolver to know the shapes, and it loses the label, which §6.5 says decides.
- **Detection first, label second.** It is what the operator's name suggests, but it is what §6.5
  forbids, and a mock that let detection override a client's label would accept a request a provider
  would refuse.

## Consequences

- `corpora/shapes/content-type` runs through `janus verify`, and a `content-type` shape matches in a
  mock. A v1–v4 `contentType` matcher verifies as today wherever a built-in component can detect the type.
- Detection is only as good as the components: the built-in JSON component detects JSON, and nothing
  built in detects an image or a PDF yet. A v1–v4 `contentType` matcher for `image/png` fails to detect
  where today's verifier sniffs magic bytes. No fixture or specification case has one.
- `ContentDetector::detect` takes the value's path, a change to the kernel's Rust interface only.
- **Tripwire**: if a pilot's pact uses `contentType` on a binary body, a built-in detector for common
  binary types (`tree_magic_mini`, as today's verifier uses) comes next.
