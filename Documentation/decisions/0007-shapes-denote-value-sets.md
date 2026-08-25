# 0007 — Define a shape by the set of values it admits, over a domain that includes absence

- **Status**: proposed
- **Date**: 2026-08-25
- **Plan tasks**: 2.2 (feeds 2.3, 2.4, 2.5, 2.8, 3.2, 3.3)
- **Evidence**: [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146) ("The shape
  language", "Provider shapes and the subsumption check"), [spike 1.1
  findings](../../spikes/1.1-idl-bakeoff/FINDINGS.md) 10–12 (unknowns must arrive named),
  [Engine Protocol specification](../specs/engine-protocol/spec.md) §2.2, §2.4–2.5

## Context

The RFC replaces matching rules with a shape language and then asks three things of it at once:
matching (does this value fit?), variant testing (what are the ways this shape is wider than one
case?), and subsumption (`admits(provider) ⊆ admits(consumer)`). The third is the demanding one — the
RFC's own unresolved-questions list flags decidability — and it is what stops the language from being
"JSON Schema with examples". Two smaller forces bound the design: shapes are recorded in pact files
that outlive the engine that wrote them, and they cross the engine boundary inside protocol frames, so
they inherit the document-first open-world regime of [ADR 0002](0002-document-first-protocol-over-frozen-pipes.md).

The FAQ objection this whole line of work answers — "optional means untested" — is really a claim
about presence, and today's model cannot even *express* the difference between a member that is absent
and one that is present and null without a per-field convention.

## Decision

**A shape denotes a set — `admits(S)` — and every other operation is defined in terms of it.**
Matching is membership; producing a value is choosing an element; subsumption is containment; a
variant dimension is a place where the set is deliberately wider than one case. One semantics, four
consumers, no second definition to drift.

Five commitments follow, and they are the contested ones:

1. **Absence is an element of the domain.** `⊥` ("this slot holds nothing") joins the protocol's
   document values, so `optional`, `forbidden` and `nullable` are ordinary operators with ordinary
   sets, and the "nullable column" production break is decided by set containment rather than by a
   special rule. `⊥` is confined to slot positions, which keeps cardinality and array positions
   countable.
2. **One canonical node form, no literal shorthand.** Every shape is `{ "shape": "<operator>", … }`.
   Shape documents contain user data, so any rule for telling "this object is a shape" from "this
   object is an example" fails on the payload with a member called `shape`. Sugar lives in the DSLs.
3. **Unions must be tagged; enums are literal sets.** `any-of` holds values, not shapes; a union of
   structures is `one-of` with a discriminator bound to distinct literals in every alternative. An
   undiscriminated union makes matching a search, variant points unnameable, and containment a
   pairwise problem over operators whose containment is already only conservative.
4. **Objects are must-ignore, with `forbidden` per member and no way to close them.** Extra members
   are admitted and ignored; a member declared `forbidden` must be absent. A closed-object operator
   would assert something about fields nobody has named and would make the RFC's "extra fields are
   fine" subsumption rule unstatable.
5. **Comparability is declared, and `unknown` is a first-class answer.** Each operator belongs to a
   class — exact, conservative, or opaque — and a checker answers yes/no/unknown, never a guess.
   Structural identity always decides `yes`, which gives every operator, including operators the
   kernel has never heard of, a floor. Component operators declare their own class; a component that
   declares nothing is opaque, and the check degrades locally and visibly.

Two consequences of the same reasoning, recorded here because they will be re-argued otherwise: an
unknown *operator* is a named failure, not an ignored member (dropping a constraint weakens a contract
while still reporting success), and an operator's `admits`, once published, is frozen forever, because
redefining it silently changes the meaning of pact files nobody can re-run.

Spec text: [Shape language specification](../specs/shape-language/spec.md); schemas
`shape.schema.json` and `variant-space.schema.json`.

## Alternatives considered

- **JSON Schema as the shape language.** Familiar, tooled, and immediately fatal: it is a schema
  language, so it invites declaring what was never demonstrated, and containment over its keyword set
  (`allOf`/`not`/`patternProperties`) is exactly the "schema-format gymnastics" the RFC says shapes
  exist to avoid.
- **Keep matching rules, add optionality flags.** Cheapest migration, but it preserves the parallel
  rules map, its cascading/precedence semantics, and the per-language reimplementations of both —
  three of the failures the RFC is a response to.
- **Literal shorthand with inferred `equality`.** Nicer to read by hand; ambiguous exactly where the
  stakes are highest (the plan compiler, the subsumption checker walking trees it did not author).
- **Undiscriminated shape unions.** More expressive, and it would make both variant identity and
  subsumption guesswork. `one-of`'s tag requirement is cheap to satisfy and buys exact comparison.
- **Per-element dimensions inside collections.** Rejected as unusable: the element count is itself a
  dimension, so the dimension set would depend on another dimension's point — and `eachLike` means
  every element is *like* the others.

## Consequences

Easier: subsumption becomes a structural walk with a decision procedure per operator; variant spaces
are computed from the same tree as matching, so a declared width is always a promise to exercise;
`explain` has one thing to explain; SDKs stay thin because they emit canonical nodes and hold no
matching logic.

Harder: the v1–v4 converter must expand the old recursive `type` matcher into explicit structure and
must carry `arrayContains` as an opaque `contains` operator (design 2.5); authors of binary-content
components must decide and declare their comparability class; and the variant space grows faster than
the RFC's arithmetic suggested — the order example is 24 variants, not 12, once `each-like`'s
cardinality dimension is counted, which pushes the sampling budget onto design 2.3.

Committed to: the operator vocabulary is append-only forever, and dimension ids and point names are as
stable as the operators, because recorded pacts and pinned variants refer to them by name.

**Tripwire** — revisit if any of these show up in Phase 3/4: the conservative class is where most real
provider shapes land (making subsumption mostly "unknown" and therefore ignorable); `one-of`'s
discriminator requirement forces authors into contortions for real polymorphic payloads; or the
computed variant spaces are so large that 2.3's caps, not the declarations, decide what gets tested.
