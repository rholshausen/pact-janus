# 0103 — A content component asks for embedded content in its answer, and the registry decodes it

- **Status**: accepted
- **Date**: 2026-11-22
- **Plan tasks**: S5.6; S5.8 (substitutions in embedded content, left open here)
- **Evidence**: [ADR 0040](0040-read-multipart-bodies-by-part-name-through-a-built-in-multipart-component.md)
  decision 3 and its tripwire; component-interfaces spec §3.1 ("the engine is the only caller") and §9.2
  (the frozen pipe world); spike [1.4](../../spikes/1.4-engine-hosting-plugins/FINDINGS.md) on wasmtime's
  re-entry policing; `engine/kernel/src/component/registry.rs`'s tests,
  `engine/component-multipart/tests/multipart_content.rs`, `engine/component-host/tests/embedded.rs`
  (with `tests/fixtures/wrapping`) and `engine/component-host/tests/csv_component.rs` (§ a CSV part of a
  multipart body); [ADR 0077](0077-match-a-multipart-parts-content-type-against-its-own-label.md),
  [ADR 0100](0100-a-content-component-says-what-a-value-operator-means-for-its-type-and-the-engine-compiles-every-slot.md),
  [ADR 0101](0101-a-slot-that-cannot-be-decoded-fails-at-the-slot-a-decodes-losses-are-warnings-and-encode-is-given-the-shape.md)

## Context

The built-in multipart component decodes each part's body through the built-in JSON, XML, text and form
components it holds itself (ADR 0040 decision 3). That was enough for stage 1a, since today's verifier
compares a part with its own built-in matchers too. But a Janus contract cannot shape a part only a
declared component reads, such as a CSV or protobuf part, and a project that declares its own JSON
component has it passed over for a JSON part. ADR 0040's tripwire said S5.6 would replace the delegation
with the registry's and leave the document as it is.

The plan item named the mechanism ADR 0040 rejected for stage 1a: a host callback in the native binding,
an import in the WASM world, and a request on the subprocess pipe, so a component could call the engine
while decoding. That conflicts with two rules the component interface was built on. The engine is the
only caller (spec §3.1), because wasmtime polices instance re-entry, and a part that is itself of the
caller's type (multipart inside multipart, or a declared component that embeds its own type) would
re-enter the instance that is mid-call. And the WIT world is frozen (spec §9.2): a new function there
is what ADR 0002 and ADR 0012 say never to add, and S5.1d is about to declare the interfaces stable.

ADR 0040 also rejected having the kernel find embedded content in a decoded document, because `encode`
had no counterpart: the kernel would need to know which members of a document are a part's body before
asking the component to write it. That objection holds against the kernel guessing. It does not hold if
the component says.

## Decision

1. **A content component asks for embedded content in its answer.** A `content/decode` result may carry
   `embedded`, a list of slots it needs decoded, each with an `id` it chose, unique in the list, a
   `content-type` and a `value`. Such a result is a question: the engine does not read its `document`.
   The engine decodes each through the session's registry, then repeats the original request with
   `embedded`, an object of the answers by id, and the component answers that one. `content/encode` asks
   the same way, its entries carrying a `document` and an optional `at`. The engine stays the only caller.
   Nothing about the bindings changes: the WIT world is untouched, the subprocess pipe needs no reverse
   frame, and the native binding carries the same members as values (spec §6.6).
2. **An embedded slot is routed as a slot is.** It goes to the first component in the scope that handles
   its type, declared ones first. Octets no component claims are bytes (spec §6.1), and octets need no
   component to encode. Each answer is that operation's result or `{ error }`, so the asking component
   decides what a refusal means. The registry does the asking and answering: it is itself a content
   component (spec §6's routing), so every path that decodes a slot gets embedded content without
   knowing it exists.
3. **One round, bounded depth.** The repeated request must be answered; asking again is
   `component-malformed`, and so is an id given twice. Content inside embedded content is resolved by the
   embedded decode's own round. Content more than eight levels below its slot is answered
   `embedding-too-deep`, an error the engine produces (`source: "engine"`), so a component that finds its
   own type inside itself without end stops.
4. **`encode` gives embedded documents their shape.** `at` is a JSON pointer to where the embedded
   document sits. When the request had a `shape` (ADR 0101), the engine finds the part of it describing
   the value at `at`, through objects, arrays, each-like items, each-entry values and the modifiers that
   wrap them, and encodes the embedded document with it. Where no one part does, through an alternative
   or a component's operator, it is encoded with none. So an empty CSV part is written as the header its
   shape declares, as an empty CSV body is.
5. **Required members stay required.** The schema's additive-evolution rules do not let a result's
   `required` set change within v1. A question carries the members an answer needs, as placeholders the
   engine does not read: `document: null`, and `value: { "content": null }`. An engine says it answers
   embedded content in `component/hello`'s `capabilities`, as `embedded-content: {}`, and a component asks
   only of one that does. An engine without it would read the placeholder as the answer.
6. **The multipart component asks the registry for every typed part** (superseding ADR 0040 decision 3).
   Each part with a `Content-Type` that is not multipart is asked for, by its index in the body; a part
   with no type is still text, or bytes when it is not UTF-8, and a part that is multipart is still bytes.
   An answer the registry decoded is the part's `body`, with its degradations reported as the multipart
   component's, located at the part's body (spec §6.4). A refusal leaves the body out, except that text a
   text type cannot read is bytes, as before. Encoding asks for every part body of a type that is not
   multipart and is not already octets. A string the registry has no component for is written as its
   text, as before. With only the built-in components registered, every document is what it was, and
   `engine/kernel/tests/multipart_bodies.rs` still holds the legacy reader to it. The component holds no
   other component, and `label` (ADR 0077) reads the parts' headers without decoding a body.

## Alternatives considered

- **A host callback, a WIT import and a reverse request** (the plan's wording, ADR 0040's first
  alternative). Re-entering the instance at work is exactly what wasmtime forbids and spec §3.1 rules out,
  and the WIT world would change in the stage that is about to declare it stable. Every binding would
  carry a new mechanism where this decision carries two members.
- **A new operation that asks what a component needs before `decode`** (`content/embedded`, then
  `decode` with the answers). It keeps `document` required without placeholders, but adds a call per
  slot for every component, or a handshake capability to avoid it, and a second operation per direction,
  to say what one member says.
- **The engine finds embedded content in the decoded document** (ADR 0040's kernel-side embedding). The
  kernel would have to know a multipart document's model to find a part's body and its type, which is the
  content-specific knowledge the kernel must not have. It also leaves `encode` without a counterpart.
- **Keep the built-ins inside the multipart component, and add the registry beside them.** Two ways to
  reach a JSON part, which disagree as soon as a project declares a JSON component.

## Consequences

- A contract can shape a part only a declared component reads. The CSV component decodes a `text/csv`
  part of a multipart body into rows, the mock serves that part as CSV written from the part's shape, and
  a verification checks a provider's part column by column (`csv_component.rs`). A declared component for
  JSON, XML, text or form is used for a part of its type.
- A v1–v4 verification that declares a component for a part's type has the reply's part decoded by it,
  as a whole body of that type already is, while the pact's side is read by the legacy layer's own
  readers. That was already true of a whole body, and no v1–v4 pact needs a declared component.
- A component author gets embedded content without a callback API: the same two operations, asked twice.
  `tests/fixtures/wrapping` is the smallest one, and decodes its own type inside itself without re-entry.
- The cost is a second call for a slot with typed parts, and parsing the outer octets twice. A multipart
  body is split twice; for the sizes a contract test carries that is noise beside the decodes it saves
  holding.
- Not done, on purpose: substitutions (ADR 0100) apply to the slot, not to embedded content, so
  `integer` of a CSV column inside a multipart part is the engine's `match:integer` and fails against
  CSV's text. Making them reach embedded content needs the engine to know, at compile time, which part of
  a shape is decoded as which type, which a multipart shape says only in a header's example. That is
  S5.8. `content/compile` is not asked for an embedded document's shape either.
- Nested bytes in an embedded answer's document travel as JSON over the WASM pipe, where a byte string
  inside a document is its base64 text, as it already is for any document (spec §4's tagging is of the
  whole document). Only a top-level byte document keeps its tag.
- **Tripwire**: a component that needs a second round (what to ask for depends on what an answer said),
  or a real cost from the repeated parse. Either would make the one-round limit, or the
  nothing-retained rule, the thing to revisit.
