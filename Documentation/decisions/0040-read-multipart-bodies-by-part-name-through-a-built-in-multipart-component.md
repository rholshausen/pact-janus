# 0040 — Read multipart bodies by part name, through a built-in multipart component

- **Status**: accepted
- **Date**: 2026-10-22
- **Plan tasks**: S1.23 (the build); S1.25 (`janus upgrade`, which this leaves as it was); S5.6 (a
  part reaching a declared component)
- **Evidence**: pact-reference's matcher (`pact_matching::binary_utils::match_mime_multipart` and its
  `parse_multipart`, `match_mime_part`, `match_field`, `match_file_part` and `match_headers`;
  `compare_bodies`' `BODY_MATCHERS`), at the version S1.1 pins (pact_verifier_cli 1.3.5, which builds
  pact_matching 2.0.11 with its `multipart` feature and `multer` 3.1.0); `pact_matching::generators::bodies`;
  S1.1's first run (`compatibility-suite/v1/25-…`);
  [ADR 0020](0020-a-content-slots-type-is-declared-on-the-interaction.md)'s tripwire;
  [ADR 0036](0036-read-text-bodies-through-a-text-component-and-unclaimed-bodies-as-bytes.md);
  [ADR 0039](0039-read-xml-bodies-by-element-name-through-a-built-in-xml-component.md)

## Context

A v1–v4 pact with a multipart body is refused as `component-unavailable`. It is the last body kind the
legacy layer refuses. Today's verifier passes the compatibility suite's multipart case, so S1.1's ledger
carries it.

Today's verifier splits both bodies with `multer`, at the boundary each side's `Content-Type` header
names. It reads each part's name from its `Content-Disposition`. A part with a `Content-Disposition` and
no file name is a *field*; any other part is a *file*. For each part of the pact's, in order, it takes
the first part of the reply with the same name, or, when that part has no name, the one at the same
position. A missing part fails, and so does a field where the pact has a file, or the reverse. A part
the pact does not have is allowed, in a request as in a response.

Then it compares the part's headers: each header the pact's part has, by a rule at
`$.<part>.<header>` or by equality. `Content-Disposition` is compared only by a rule, and a missing
`Content-Type`, `Content-Disposition` or `Content-Transfer-Encoding` is allowed. A `base64`
`Content-Transfer-Encoding` is undone on both sides. A field is compared as text, by the rules at
`$.<part>` or by equality. A file is compared as a body of its own `Content-Type` is, after the same
type check a body gets: the rules under `$.<part>` move to the part's root, and JSON, XML, form,
octets and text each go to their own matcher. Generators are not applied to a multipart body.

The plan item says each part is compared "through the components S1.6, S1.21 and S1.22 add". The
content interface has no way for one component to ask another to decode, and ADR 0020's tripwire names
a multipart body as the case that might need two content types in one slot.

## Decision

1. **Multipart is a built-in content component, `multipart`**, in `engine/component-multipart`. It
   claims every `multipart/*` type, as today's verifier does. Every embedding registers it after form,
   and `engine/hello` lists `content/multipart`. It splits a body with `multer`, as today's verifier
   does, so both accept the same bodies, at the boundary its content type names, read with `mime` as
   today's verifier reads it. A content type with no `boundary` does not decode. It contributes no
   plan fragments and detects nothing.
2. **A multipart body's document is keyed by part name.** It is an object with one member per part
   name, in the order the names first appear. Each member is an array of the parts with that name, in
   body order. Parts with no name are under `""`. A part is an object of:
   - `headers`, its headers: each lower-cased name with its value, as a string. A repeated header
     keeps its last value, which is all `multer` keeps;
   - `filename`, the file name its `Content-Disposition` gives, when it gives one;
   - `body`, its content, decoded as decision 3 says. It is absent when the part's type claims it and
     it does not decode.

   `encode` writes the inverse. It uses the boundary the content type names, or writes one the body
   does not contain and labels the slot with it. A named part with no `Content-Disposition` gets
   `form-data; name="…"`, with its `filename`. The order between differently named parts is not kept,
   and today's verifier does not read it either.
3. **A part's body is decoded through the built-in JSON, XML, text and form components, in their
   registry order, by the part's own `Content-Type`.** A `base64` `Content-Transfer-Encoding` is undone
   first, and any other is ignored, as today. A part with no `Content-Type` is text, the default RFC
   2046 gives it. Text that is not UTF-8, with a type or without, is bytes, as the legacy layer reads a
   text body that is not (ADR 0036 decision 3). A part of any other type, including another multipart
   body, is bytes. A part never reaches a component a project declares: the multipart component holds
   the built-in ones itself, and the content interface is unchanged. Reaching the registry's
   components, declared ones included, is S5.6.
4. **ADR 0020's tripwire does not fire.** A part's type travels in the part's own headers, so the
   slot still declares one type: the body's multipart type.
5. **The legacy layer reads a pact's multipart body into parts, and the legacy compiler compares them
   as today's verifier does.** The pact's side is split with `multer` by the legacy layer, at the
   boundary of the body's content type. Each part is read by its own type, as a body is (ADR 0036
   decision 3). A test holds that reader to the component's document for every multipart body in the
   repository. A reply is decoded under its own label, not the pact's type (ADR 0036 decision 4), because
   its boundary is its own. `legacy:body-type` and the `Content-Type` header check compare the two types
   as today, boundary included.

   For each part of the pact's, the plan finds the reply's first part of that name, or the unnamed
   part at the same position among the unnamed parts. A new legacy action, `legacy:mime-part`, fails
   a missing part or the wrong kind with today's messages. The headers are compared as today. A field
   of text is compared as text, by the rules at `$.<part>`. A file, or a field with a type of its own,
   is compared as a body of its type is, by the rules under `$.<part>` moved to the part's body. A
   file is first type-checked with `legacy:body-type` against the part's `Content-Type`, and a file of
   another type is not compared further, as today. Parts the pact does not have are allowed.
6. **Request generators on a multipart body are not applied**, as today. The body is sent as recorded,
   and each generator on it says so (ADR 0034).

## Alternatives considered

- **Delegate through the registry.** The content interface would gain a host callback (a native
  callback, a WIT import, a reverse frame on the subprocess pipe), so a part would reach every
  registered component, declared ones included. It is the general answer and S5.6 builds it. Stage 1a
  needs only the built-in types, because today's verifier compares a part with its built-in matchers
  too. A change to all three bindings is not worth making for no verdict.
- **Kernel-side embedding.** `decode` would return where it left octets of another type, and the
  kernel would decode them through the registry. That works for `decode` but has no `encode`
  counterpart: the kernel would have to know which members of a document are a part's body before
  asking the component to write it.
- **An array of parts in body order.** It keeps the order between differently named parts, and suits
  `multipart/mixed`. But finding a part by name would need an action at every step, as ADR 0039 found
  for XML, and a `multipart/form-data` shape is naturally keyed by name.
- **Compare a field as text whatever its type, as today.** A field with `Content-Type:
  application/json` would have to be kept as text by the component too, so a Janus contract could not
  shape it.

## Consequences

- A v1–v4 pact with a multipart body verifies as it does today. S1.1's ledger entries for the
  compatibility suite's multipart case are gone, and no body kind is refused any more.
- A Janus contract can declare a multipart body and shape it in decision 2's model. A mock serves it
  with its parts grouped by name.
- Deliberate differences from today's verifier, each found only where today's own behaviour is
  accidental:
  - a field with a type of its own, such as JSON, is compared as that type, where today compares its
    text;
  - a part with no `Content-Type` is text on both sides. Today compares the pact's as octets, which
    differs only under a rule, and the reply's as the pact's part's type;
  - unnamed parts are matched by position among the unnamed parts, where today counts every part. The
    two agree unless a body mixes named and unnamed parts, and a named part of the pact's is never
    found as an unnamed part at its position;
  - a part that is itself multipart is compared as octets. Today never matches one, because no
    boundary reaches its parser;
  - the pact's boundary may come from its body's content type, where today reads only its headers.
- Every part of the pact's with one name is compared with the reply's first part of that name, as
  today. Two files under one name both meet the reply's first.
- Mismatch messages are the engine's, except `Expected a MIME part '…' but was missing` and the two
  kind messages.
- `janus upgrade` still carries a multipart body across with `body-not-parsed`. S1.25 converts
  multipart bodies too.
- **Tripwire**: when a project needs a part of a type only a declared component reads, S5.6 replaces
  decision 3's delegation with the registry's, and the document does not change.
