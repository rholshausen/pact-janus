# 0047 — Upgrade a body into its component's document, and check its type on the header

- **Status**: accepted
- **Date**: 2026-10-25
- **Plan tasks**: S1.25
- **Evidence**: `engine/kernel/tests/upgrade_agreement.rs`, which compiles every v1–v4 case in
  `corpora/legacy/` both ways and finds the same verdicts; the ten `corpora/legacy/upgrade-*-body*`
  cases; `cli/tests/cli.rs`'s `a_pact_of_every_body_kind_and_its_upgraded_contract_verify_the_same_provider`;
  ADRs 0036, 0038, 0039 and 0040, which say how each body kind is read and compared

## Context

`janus verify` reads text, binary, form, XML and multipart bodies (S1.6, S1.21 to S1.23). `janus upgrade`
still carried each of them across as the pact's octets, with a `body-not-parsed` finding, and asserted
nothing about them. So a contract verified less than the pact it came from, which plan-grammar spec §4.4's
two-path agreement forbids.

The shapes are not the hard part. Each body kind already has a document (ADR 0036 to ADR 0040), and the
legacy compiler says exactly how a v1–v4 pact compares one. Four things are not settled by those ADRs:

- **The reply's type.** A pact that names no `Content-Type` header still checks that the reply's type
  agrees with its body's (`legacy:body-type`, ADR 0036 decision 4). A contract declares a slot's type
  as a decode instruction, not a constraint (ADR 0020 decision 2): a `text/plain` contract decodes a
  `text/html` reply as text, and passes it. The pact fails it.
- **The boundary.** A multipart type carries the boundary of the octets it labels. A contract that
  declared the pact's type, boundary included, would decode a reply with another boundary as nothing.
- **Bytes.** No component claims `application/octet-stream` or `image/png`, and a declared type no
  component claims fails the interaction as `component-unavailable` (contract-file spec §5.5).
- **Text read as a number.** Since ADR 0039 decision 5, a number or boolean rule on text (an attribute,
  a form or query value, a header, a path) asks whether the text parses as one (`legacy:parses-as`).
  The upgrade still wrote `integer`, which no string satisfies. The built-in text component refuses an
  `integer` body outright.

## Decision

1. **A body a built-in component reads declares its type.** A text body that is UTF-8, a form, XML and
   multipart declare the pact's type in `content-types` and require `content/text`, `content/form`,
   `content/xml` or `content/multipart`. So a reply is decoded as the pact's verification decodes it,
   whatever it is labelled. JSON stays undeclared, as before.
2. **A multipart type is declared without its boundary, and the multipart component takes a boundary
   from the slot's label when the declared type names none.** The boundary frames the octets; it is
   not part of what the type says. A reply names its own boundary, as ADR 0040 decision 5 already
   reads it on the legacy path.
3. **Octets are left undeclared.** A binary body, or text that is not UTF-8, is shaped as bytes, with
   a base64 example, and has no `content-types` entry. An undeclared slot of a type nobody claims is
   bytes (ADR 0036 decision 2). Text that is not UTF-8 also gets a `body-not-parsed` judgement: a reply
   labelled as text is decoded as text, which those octets are not.
4. **Each body is shaped as the legacy compiler compares it.**
   - Text and bytes are one value at `$`: the rule there, or `equality`.
   - A form is the query slot's model, an object of arrays of strings, each name's values in order, as
     many as the pact has.
   - XML is ADR 0039's document. For each element: its namespace, its attributes, its text (`forbidden`
     where the pact has none and no rule; left out where a type rule reaches it, since any text
     satisfies one) and its children. A group a type rule reaches is `each-like`, `min: 1`, over the
     first child; any other group is a fixed `array`. A child a type rule names and the pact lacks is
     `forbidden`.
   - Multipart is ADR 0040's document. Each name holds a fixed array of parts. Each part has its
     headers (`content-type`, `content-disposition` and `content-transfer-encoding` `optional`), its
     kind (a field's `filename` is `forbidden`, a file's is a `type`) and its body: a field of text by
     the rules at `$.<part>`, anything else by its own type under the rules below `$.<part>`.
5. **A number or boolean rule on text becomes the pattern `legacy:parses-as` accepts.** `integer` is
   `^\+?[0-9]+$`, `number` and `decimal` are Rust's float grammar, and `boolean` is `^(?:true|false)$`.
   This applies wherever v1–v4 read text, so a query, header or path rule converts correctly too.
   The integer pattern also admits digits too large for 64 bits, which v1–v4 refuse. A
   `rule-as-pattern` note per slot says so.
6. **The pact's check of the reply's type becomes a shape on the reply's `content-type` header.** When
   the pact names no `Content-Type` header and its body has content of a type with no wildcard, the
   converted part's headers gain `content-type`: `optional` (a reply with no type passes), of a pattern
   that admits what `legacy:body-type` admits. That means the same type and subtype in any case, a
   suffix type when the pact's has no parameters, and the same parameters in any order when it has some.
   A label that is not a type, or has a wildcard, also passes `legacy:body-type`, and the pattern refuses
   it. A `body-type-on-header` judgement says so. This applies to JSON bodies too, which had the same gap.
7. **A converted body's evidence is the pact's octets**, recorded as `text` when they are UTF-8 and
   `base64` otherwise, with the pact's type. A verifier sends a slot already carried as octets as it
   stands, so the provider gets the bytes the pact's verification sends. A document re-encoded by its
   component would lose what the component does not keep: XML comments and prefixes, a form's
   interleaving, a multipart body's boundary and part order.
8. **Where the shape language cannot say what v1–v4 do, the conversion reports it.** It cannot close a
   response's form names or an element's children where a type rule reaches them, which
   `response-body-opened` (lossy) reports. A fixed array of children or parts is narrower than the
   reply v1–v4 allow, which may have more, and `rule-narrowed` (judgement) reports it once per body. A
   size or `notEmpty` rule on an element, which counts its children of every name, has no shape, and
   neither does the kind of an unnamed part with no `Content-Disposition`. Both are `rule-unmapped`.
   `body-not-parsed` is now only for a body that is not what its type says.
9. **The shape compiler quotes a member name a path cannot hold**, as the legacy compiler does
   (`member_path`): `['@{urn:x}rev']`, `['']`, `['a.b']`. A namespaced attribute, an unnamed part and a
   JSON member with a dot in its name were resolved as other members before.

## Alternatives considered

- **Check a declared slot's label in the kernel, for every contract.** Every contract would then refuse
  a reply its type does not agree with. But ADR 0020 decided a declaration is not a constraint, and this
  would change every native contract to close a gap that only converted ones have. The header shape
  says the same thing where the pact said it, and an author can read it and edit it.
- **A required `content-type` header, with no `optional`.** It needs no presence dimension, so a
  converted contract's variant space would stay at one. But a reply with a body and no type passes
  today and would fail, and a false failure is what an upgrade must not introduce.
- **A pattern that also admits a label that is not a type, or has a wildcard.** That would be exact,
  but it would be about three times longer, every author would read it, and it would only admit labels
  no real provider sends.
- **Record a declared body as its document**, as contract-file spec §5.5 has a native contract record
  one. It would be easier to read and diff. But a replayed request would then be the component's
  re-encoding, not the pact's bytes, and variant-semantics spec §5.2 says the provider sees the bytes
  the consumer sent.
- **Declare a binary type anyway.** It would be shown in `content-types`. But no component claims it,
  so the interaction would fail as `component-unavailable` before anything ran.

## Consequences

- No conversion of a well-formed body has a `body-not-parsed` finding, and every v1–v4 case in
  `corpora/legacy/` reaches the same verdict compiled directly and upgraded. There are two exceptions,
  both reported: a closed request body (`request-body-opened`), and a header with no rule
  (`rule-narrowed`).
- A converted JSON interaction whose pact names no `Content-Type` header gains a `content-type` header
  shape. Its variant space grows by that `optional`'s presence dimension, and its report says `selected:
  1` of a larger space, as contract-file spec §8.3 already allows.
- An `eachValue` rule's values are shaped by its own rules, and an `arrayContains` variant's element by
  the variant's, as ADR 0042 compiles them. `contains` claims a distinct element per entry, where v1–v4
  let one element satisfy two variants, which is reported as `rule-narrowed`.
- The multipart component's `decode` reads a boundary from a slot's label when the declared type has
  none. A declared boundary is still the one used.
- A header with no rule is still converted to `equality`, so a reply that adds a parameter to its
  `Content-Type` (`application/xml; charset=UTF-8`) fails a converted contract and passes the pact.
  That is the existing `rule-narrowed` judgement, and staged plan S3.4 closes it, once S3.3's
  `http:media-type` exists.
- **Tripwire**: if a pilot finds a converted contract failing where its pact passes, and no finding
  explains it, the conversion has a gap these rules do not describe. The fix is in the upgrade, with a
  case in `corpora/legacy/` that holds it.
