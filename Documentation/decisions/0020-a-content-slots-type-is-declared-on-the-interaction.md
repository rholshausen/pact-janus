# 0020 — Declare a content slot's type on the interaction, beside its parts, not inside them

- **Status**: accepted
- **Date**: 2026-09-23
- **Plan tasks**: 8.1 (amends designs 2.5 and 2.6; binds 2.9's `json` primitive, 6.2, 6.3)
- **Evidence**: [third-party component report](../third-party-component-report.md) finding 9;
  `Documentation/kernel-boundary-review.md` finding 1's resolution, which left "a second content
  type" to this task; SDK spec `behavioural-spec.json`'s `json` entry, which anticipated "a per-slot
  content-type declaration in the interaction spec"

## Context

Task 8.1 put a second content component — a third-party `text/csv` handler — behind the engine, and
found that nothing could tell the engine a slot is CSV. An interaction specification's slots hold
bare shapes, and a shape denotes a set of *decoded* values (ADR 0007): it says what the document must
be, not how octets become one. The engine had filled the gap by assuming: a structured value in a
content slot was encoded as `application/json`, in the one place a served body is produced.

On the decoding side the gap is smaller but real. A transport labels an arriving body with the type
the peer declared, and a content registry can route on that label. But the consumer's shape is about
the document *as the consumer declared it*, so a provider that answers JSON where CSV was agreed would
be decoded by the wrong component and could match anyway.

So the type is information the author has and the documents cannot carry. The question is where it
goes. Contract spec §5.1 fixes a slot's position as "a shape document owned by design 2.2", and every
reader of a contract depends on that position: the plan compiler, the variant-space walk, the
subsumption checker, `janus upgrade` and the generated SDK bindings.

## Decision

**1. An interaction MAY carry `content-types`: part name → slot name → media type**, nested like
`parts` and beside it, in both the interaction specification and the recorded interaction:

```json sketch
{ "description": "orders as CSV",
  "content-types": { "response": { "body": "text/csv" } },
  "parts": { "response": { "body": { "shape": "each-like", "items": { } } } } }
```

**2. It is a decode instruction, not a constraint.** The declared type selects the content component
that encodes the slot when the engine produces it — a mock's reply, a verifier's request — and the one
that decodes it when it arrives. The shape then applies to that document, exactly as before.
Subsumption does not compare it. An entry that names a part or slot with no shape is
`interaction-invalid`.

**3. Absent means today's behaviour.** A slot with no declared type is produced as `application/json`
when its value is structured and passed through plainly otherwise, and decoded by whatever type the
arriving slot is labelled with. Existing contracts, SDKs and corpora keep meaning what they meant.

**4. A recorded variant's slot value carries the declared type** in its `content-type` member
(contract spec §5.3). The document stays recorded as JSON, which keeps a CSV body's evidence readable
and diffable; the tag says which component turns it back into octets.

## Alternatives considered

- **A slot wrapper — `{ "content-type": …, "shape": … }` in the slot's position.** It keeps the
  declaration next to its shape, which is its one real advantage. It was rejected because it is not
  additive. An old reader meeting it finds an object where it expects an operator name and fails,
  where an unknown interaction member would have been ignored under the open-world rules. It is also
  in-band tagging: a reader must inspect a slot's members to know which of two forms it holds, which
  contract spec §5.3 and ADR 0007 already refused.
- **Derive it from the transport — the HTTP body's type is whatever `headers.content-type` says.**
  No new member, but it makes the engine read a header shape's example to decide how to encode a body,
  which is HTTP knowledge in the kernel by the back door. It also has no answer when the header is
  written as a regex or not written at all.
- **Put it on the shape.** It would make a decoding instruction part of a value-set denotation, and
  two shapes that admit the same documents would compare differently by how the bytes were spelled.

## Consequences

- The kernel still knows no content type. It routes a declared or labelled media type to a registered
  content component, and the component decides what the octets mean.
- A declaration can drift from its parts (it may name a slot that does not exist). That costs a check
  at `add-interaction`, which is where the author is looking.
- The declaration and the evidence live in different places: the interaction declares, the recorded
  slot value repeats it. A verifier reads the declaration, and a disagreement between the two is the
  contract's problem, reported as `contract-invalid`.
- SDKs gain a spelling for it: a `content(type, document)` primitive that records a declaration.
  `json(document)` stays sugar that declares nothing, because an undeclared structured slot already
  means JSON (decision 3). Making it declare would change the interaction every existing consumer
  test submits, for no difference in behaviour.
- **Tripwire:** if a slot ever needs *two* content types in one interaction — a multipart body, say,
  whose parts are themselves typed — a flat map from slot to one media type is the wrong shape. Revisit
  then, rather than nest this.
