# Shape language specification (v1, final)

Plan task: **2.2**. Status: **final**.

This document specifies the shape language: the operator set, how shapes are written as JSON inside
interaction specifications and pact files, what a shape *means* (`admits`), which variant dimension
each operator contributes, and how much of `admits(P) ⊆ admits(C)` a checker can decide. It replaces
the matching-rules model as the user-facing surface — matching rules remain readable, as v1–v4 pact
input compiled to shapes and plans (designs 2.5, 3.5), but nothing new is authored in them.

The schemas under [`schemas/v1/`](schemas/v1/) are the **specified surface**, on the same terms as the
Engine Protocol's: this prose defines their semantics, the schemas define their shapes, and a
disagreement between them is a bug to file rather than a precedence question. Shape documents cross
the engine boundary inside protocol frames, so they inherit the Engine Protocol's open-world
authoring rules ([protocol spec §2.2](../engine-protocol/spec.md#22-open-world-authoring-rules)) and
its document model, bytes included ([§2.4–2.5](../engine-protocol/spec.md#24-the-document-model-json-values-plus-bytes)).
The design decision behind this specification is
[ADR 0007](../../decisions/0007-shapes-denote-value-sets.md).

Source: the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146), "The shape language"
and "Provider shapes and the subsumption check". The RFC gives seven operators and a variant-dimension
column; this specification turns them into a value-set semantics, adds the structural operators the
RFC's own example needs but does not name (`object`, `each-entry`, positional `array`), and fixes the
encoding.

## Contents

1. [Scope and conformance](#1-scope-and-conformance)
2. [The value domain](#2-the-value-domain)
3. [Shape documents](#3-shape-documents)
4. [The operator set](#4-the-operator-set)
5. [Composition rules](#5-composition-rules)
6. [Variant dimensions](#6-variant-dimensions)
7. [Matching, producing, recording](#7-matching-producing-recording)
8. [Comparability: what a subsumption checker can decide](#8-comparability-what-a-subsumption-checker-can-decide)
9. [Evolution and compatibility](#9-evolution-and-compatibility)

Worked examples — the RFC's order payload with its full variant space, and a tour of the composition
edge cases — live under [`examples/`](examples/). Every shape document in them, and in this
specification, is validated against the schemas by `cargo test -p pact_janus_schema_compat`, so the
examples cannot drift from the specified surface.

---

## 1. Scope and conformance

**MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT** and **MAY** are to be interpreted as described in
RFC 2119.

Conformance roles:

- An **engine** compiles shapes into plans (design 2.4), matches values against them, produces values
  from them, and computes their variant space.
- An **author** — a DSL, a converter from v1–v4 pacts, a provider-shape importer — writes shape
  documents.

In scope: the operator set and its semantics, the JSON encoding, well-formedness, the variant
dimensions, and the comparability classes that a subsumption checker consumes.

Out of scope, with owners:

| Question | Owner |
|---|---|
| which variants to actually run: sampling, thresholds, caps, pinning, `whenVariant` | design [2.3](../variant-semantics/spec.md) |
| how a shape becomes an executable plan; the node grammar and action set | design 2.4 |
| how shapes and exercised examples are written into a pact file; v1–v4 → v5 conversion | design 2.5 |
| how content components decode bodies; generator components | design [2.6](../component-interfaces/spec.md) |
| the subsumption walk, finding severities, warn/block policy | design 2.8 |
| the interaction-specification document that carries parts and shapes | designs 2.5, 3.2 |

The last row matters for reading the examples: this specification says what a *shape* is, not what an
*interaction* is. Where an interaction spec appears below it is an illustrative sketch, marked as such.

## 2. The value domain

### 2.1 Values, and absence

A shape denotes a set of values. The values are those of the Engine Protocol's document model
(protocol spec §2.4) — `null`, boolean, number, string, array, object, and **bytes** — plus one
element that is not a value at all:

- **absent** (written `⊥`): the distinguished element meaning *this slot holds nothing*.

`⊥` exists because presence is the question the shape language was created to answer honestly, and
because `null` is not it: a member present with value `null` and a member that is not there are
different observations about a provider, and today's matching-rule model cannot tell them apart
without a per-field convention. Making absence an element of the domain lets `optional`, `forbidden`
and `nullable` be ordinary operators with ordinary value sets, and lets a subsumption checker compare
presence the same way it compares anything else (§8).

`⊥` is not a value: it can appear only where the surrounding document has a **slot** that may be empty
— an object member, and whatever slot-like positions a part exposes (§3.6). §5.1 makes that a
well-formedness rule.

### 2.2 Bytes and decoded content

A body arrives as octets. Whether a shape sees octets or a structured document is the content
component's business (design [2.6](../component-interfaces/spec.md)): the component decodes the part's content into a document in this
model, and the shape applies to *that document*. Two consequences, both normative:

- A shape that addresses into structure (`object`, `array`, `each-like`, `each-entry`) requires a
  decoded document. If the content component cannot decode the content — no component for the content
  type, or content that does not parse — the interaction fails with a component or matching error; the
  shape is never applied to a guess.
- A shape over undecodable or deliberately opaque content operates on the octet sequence itself. The
  operators that accept bytes are `any`, `equality`, `not-empty`, `content-type`, and `regex`/`include`
  where the octets are valid text in the part's charset. This is what lets a contract test assert on a
  malformed payload, which protocol §2.4 identifies as the case that decides the document model.

Byte-valued examples are written with the protocol's tagged-content form (protocol §2.5): the node's
`example` member carries the value, and a sibling `encoded` member tags its representation. Absent tag
means the natural JSON value; `"base64"` means the base64 of an octet sequence.

### 2.3 `admits`

For a shape `S`, **`admits(S)`** is the subset of the domain (§2.1) that `S` accepts. `admits` is the
whole semantics: everything else in this document is defined in terms of it.

- **Matching** a value `v` against `S` succeeds iff `v ∈ admits(S)` (§7.1).
- **Producing** a value from `S` means choosing an element of `admits(S)` — the shape's examples pick
  which one (§7.2).
- **Subsumption** asks whether `admits(P) ⊆ admits(C)` for a provider shape `P` and a consumer shape
  `C` (§8, design 2.8).

Shapes are *not* schemas, and `admits` is where the difference is visible. A schema is a claim about
what a system does; a shape is a claim that has been demonstrated for at least one element of
`admits(S)` per exercised variant, and is only ever recorded in a pact file alongside those exercised
examples (§7.3). The RFC's discipline — nothing is declared that is not demonstrated — survives
because `admits` and the variant space (§6) are computed from the same tree: every way a shape can be
wider is a dimension, and every dimension is something a test either exercised or did not.

## 3. Shape documents

### 3.1 One node form

Schema: [`schemas/v1/shape.schema.json`](schemas/v1/shape.schema.json). Every shape is a JSON object
with a `shape` member naming its operator:

```json shape
{ "shape": "integer", "example": 42 }
```

`shape` is an **open discriminator** in the protocol's sense (protocol spec §2.2 rule 1): a plain
string with the core vocabulary listed in `x-known-values`, never an `enum`. Operator-specific members
sit alongside it — `pattern`, `format`, `members`, `items`, `of`, `options`, `alternatives` — and
unknown *members* are ignored, exactly as anywhere else in the protocol.

Unknown *operators* are not ignored: see §3.7.

### 3.2 There is no literal shorthand

A shape is always a node object. A bare JSON value is never a shape, and there is no "value here means
equality to this value" rule:

```json shape
{ "shape": "object",
  "members": {
    "id":     { "shape": "equality", "example": 42 },
    "status": { "shape": "equality", "example": "PENDING" } } }
```

The alternative — allow literals and infer `equality` — reads better in a hand-written file and is
what today's pact files do, with the matching rules in a parallel map. It is rejected here for one
reason: a shape document contains user data (examples, enum options, discriminator literals), so a
form in which *some* objects are shapes and others are data needs a rule for telling them apart, and
every such rule fails on the payload that happens to have a member called `shape`. A single node form
has no ambiguity to resolve, which matters most exactly where the stakes are highest — the engine
compiling a plan, and the subsumption checker walking two trees it did not author.

Shorthand belongs in the DSLs, where the RFC's `integer(42)` and `optional(datetime(...))` produce
these nodes. SDKs stay thin by construction: sugar in, canonical nodes out, no matching logic.

Operator names are kebab-case (`each-like`, `any-of`, `one-of`, `not-empty`), following the protocol's
convention for operation names and error codes. DSLs use their language's idiom — `eachLike`,
`anyOf` — and map to these names; the mapping table is part of each SDK's specification (design 2.9).

### 3.3 Examples and producibility

A node's `example` member carries a value the shape admits. It is what makes a shape demonstrable
rather than declarative, and it is the value a mock serves or a replayed request carries.

- Every **value operator** (§4.2) SHOULD carry an `example`; for `equality` the example *is* the
  operator's parameter and MUST be present.
- **Structural operators** derive their example from their children: an `object` example is the
  object of its members' examples, an `each-like` example is `min` copies of its item example. A
  structural node MAY carry an explicit `example`, which then MUST be admitted by the node.
- A shape is **producible** under a variant assignment (§6) iff every node reachable under that
  assignment can yield a value: a value operator with an `example` or a `generator` (§3.4), a
  structural operator all of whose reachable children are producible, `forbidden` and an absent
  `optional` yielding `⊥`.

An engine MUST reject a shape that is not producible under every variant it is asked to serve, with
the protocol's `interaction-invalid` error naming the offending path (protocol spec §10.2). This is
the check that keeps "declared but never demonstrated" out of a pact file at the earliest possible
moment — before the test runs, not after it passes.

Provider shapes (RFC, design 2.8) are the deliberate exception: a shape imported from a protobuf
descriptor or an OpenAPI document has no examples and is never produced from. It is matched against
and compared, and the pact file never carries it.

### 3.4 Generators

A node MAY carry a `generator` object naming a generator component and its configuration (design 2.6):
where a value must be fresh rather than replayed — a timestamp that must be recent, an id that must be
unique per run — the generator produces it and the shape still governs what is admitted. The
generator's output MUST be admitted by the node's own shape; an engine SHOULD check this and report a
generator whose output its own shape rejects as a component error, not as a test failure. Generator
configuration is design 2.6's surface; this specification only reserves the member.

### 3.5 Component operators and namespacing

The core operator vocabulary is unnamespaced. A component — content handler, matcher component, plugin
— contributes operators under its own namespace, `<component>:<name>`:

```json shape
{ "shape": "protobuf:enum", "example": "SHIPPED", "generator": {} }
```

Namespacing rules, matching the plan grammar's (design 2.4):

- An unnamespaced operator name is reserved for this specification, forever. A component MUST NOT
  define one, and an engine MUST NOT resolve an unnamespaced name to a component.
- The namespace is the component's identifier; the requirement that pulls it in
  (`content/protobuf >= 2`) travels in the interaction spec's component requirements, not in the
  shape node.
- A component operator's semantics — its `admits`, its variant dimension if any, its comparability
  class (§8) — are declared by the component. The kernel treats them opaquely and MUST NOT guess.

The HTTP status matcher of today's model is the worked case: `status-code` is *not* a core operator,
because the kernel knows nothing about HTTP. It arrives as `http:status-class`, contributed by the
transport component that knows what a status class is.

### 3.6 Where shapes attach

An interaction specification (designs 2.5, 3.2) describes parts; each part exposes named **slots**;
each slot holds one shape. Which slots a part has is the transport and content components' business,
not this specification's. The shape language cares about exactly one property of a slot: whether it
may be empty (§5.1).

Illustrative only — an HTTP response part, with the interaction frame belonging to designs 2.5/3.2:

```json sketch
{ "status": { "shape": "equality", "example": 200 },
  "headers": { "shape": "object",
               "members": { "content-type": { "shape": "each-like",
                                              "items": { "shape": "include", "substring": "application/json",
                                                         "example": "application/json; charset=utf-8" } } } },
  "body": { "shape": "object", "members": {} } }
```

Note what is *not* special: a header slot holds a list because HTTP headers are multi-valued, and
`each-like` handles it with no header-specific operator. Query parameters are the same. The kernel
boundary holds (plan task 3.8) because the shape language never learned what a header is.

### 3.7 Unknown operators are a named failure

The protocol's rule for an unknown member is *ignore it*. The rule for an unknown **operator** is the
opposite, and deliberately so: an operator is a constraint, and silently dropping a constraint turns a
contract into a weaker contract that still reports success. An engine that meets an operator it does
not implement MUST fail the interaction and name the operator:

| Operator | Engine's response |
|---|---|
| unnamespaced, not in this specification | `interaction-invalid`, `problems[].path` at the node, message naming the operator |
| namespaced, no such component loaded | `component-unavailable`, `details.component` naming the requirement |
| namespaced, component loaded, operator unknown to it | `component-failed`, the component's own error passed through |

"Degrade with the unknown named" (protocol spec §11.3) is preserved — the unknown arrives named. What
changes is the policy applied to it, because the safe direction for a vocabulary is *ignore*, and the
safe direction for a constraint is *stop*.

### 3.8 Inherited authoring rules

Shape documents travel inside protocol frames and are recorded in pact files. They therefore follow
protocol spec §2.2 in full: open vocabularies are strings with `x-known-values`, unknown members are
ignored and preserved where practical, discriminators are open, no `additionalProperties: false`, no
remote `$ref`, type-shaped schema titles. The same CI checker enforces them
([`tools/schema-compat`](../../../tools/schema-compat/README.md)) and applies the same
additive-evolution rules to `schemas/v1/` (§9).

## 4. The operator set

### 4.1 Overview

| Operator | Members | `admits` | Variant dimension (§6) | Comparability (§8) |
|---|---|---|---|---|
| `any` | — | every value (not `⊥`) | — | exact |
| `equality` | `example` | `{ example }` | — | exact |
| `type` | `example` | every value of the example's kind | — | exact |
| `string` `number` `integer` `decimal` `boolean` `null` | — | every value of that kind | — | exact |
| `not-empty` | — | non-empty string, array, object or bytes | — | exact |
| `regex` | `pattern` | strings the pattern matches | — | conservative |
| `datetime` `date` `time` | `format` | strings parsing under the format | — | conservative |
| `include` | `substring` | strings containing it | — | conservative |
| `content-type` | `content-type` | octets detected as that content type | — | conservative |
| `semver` | — | strings that parse as a semantic version | — | exact |
| `object` | `members` | objects whose named members match; others ignored | — (its members' dimensions) | exact |
| `array` | `entries` | arrays of exactly that length, positionally matched | — | exact |
| `each-like` | `items`, `min`, `max` | arrays sized in `[min, max]`, every element admitted | cardinality | exact |
| `each-entry` | `keys`, `values`, `min`, `max` | objects sized in `[min, max]`, every key and value admitted | cardinality | exact |
| `contains` | `entries` | arrays containing a distinct match for each entry | — | opaque |
| `optional` | `of` | `admits(of) ∪ { ⊥ }` | presence | exact |
| `forbidden` | — | `{ ⊥ }` | — | exact |
| `nullable` | `of` | `admits(of) ∪ { null }` | nullability | exact |
| `any-of` | `options`, `example` | the listed literals | value | exact |
| `one-of` | `discriminator`, `alternatives`, `default` | the union of the alternatives | alternative | exact |

### 4.2 Value operators

**`equality`** admits exactly its example, compared structurally: for objects and arrays, deep
equality; for bytes, octet equality; for numbers, numeric equality (`1` and `1.0` are the same value,
their lexical difference being the content component's, not the value's).

**`type`** admits every value of its example's kind, where the kinds are `null`, boolean, number,
string, array, object and bytes. Its example is therefore required, not merely recommended — it *is*
the parameter, the way `equality`'s is. On a composite example it constrains the kind only — an `object`
example does *not* make `type` recurse into members, because structure is declared with `object`, not
inferred. This is a deliberate narrowing of today's `type` matcher, whose recursive behaviour is the
single largest source of "why did this match?" questions; the compiler for v1–v4 pacts (design 3.5)
expands the old recursive form into the explicit structural shape it means, which is exactly the
transformation `janus upgrade` should be showing its work for (design 2.5).

**Kind predicates.** `string`, `boolean` and `null` are unambiguous. For numbers: `number` admits any
JSON number, `integer` admits a number with no fractional part, `decimal` admits a number written with
one. Where a content component cannot distinguish lexical forms — a binary encoding with one numeric
type — `decimal` degrades to `number`, and the component MUST say so rather than pretend; the
comparability class is unaffected, since the domain is the decoded document.

**`not-empty`** admits a value of any kind whose length is non-zero: a non-empty string, a non-empty
array, an object with at least one member, a non-empty octet sequence. `null` and `⊥` are not
admitted.

**`regex`** admits strings the pattern matches. Two things are fixed here because leaving them open
produced years of cross-language divergence:

- **Dialect**: the RE2-compatible subset — character classes, alternation, repetition, anchors,
  non-capturing and capturing groups, but no backreferences and no lookaround. It is what the engine's
  regex library implements natively, it is available in every SDK language, and it is what makes
  linear-time matching and (in principle) language containment decidable.
- **Anchoring**: a pattern matches if it matches *anywhere* in the string, exactly as v1–v4 pacts
  behave. Authors anchor explicitly with `^` and `$`. Choosing implicit full-match here would be the
  better default in isolation, and would silently change the verdict of every upgraded pact file,
  which is a worse thing to be right about.

**`datetime`, `date`, `time`** admit strings that parse under `format`, whose pattern language is the
one v3/v4 pact files already use (the Java `DateTimeFormatter` subset), so conversion is
character-for-character (design 2.5). A shape MAY omit `format`, in which case the operator admits any
string parsing as an ISO-8601 datetime, date or time respectively.

**`include`** admits strings containing `substring`. **`semver`** admits strings parsing as Semantic
Versioning 2.0.0. **`content-type`** admits octet sequences detected as the named content type by the
content-detection rules of the content components in play; it is the one operator that inspects octets
rather than a decoded document by design.

### 4.3 Structural operators, and the must-ignore default

**`object`** admits objects. For each name in `members`, the corresponding slot of the candidate value
— the member's value if present, `⊥` if not — must be admitted by that member's shape. A member whose
shape does not admit `⊥` is therefore **required** by construction, with no separate "required" flag.

**Members not named in `members` are admitted and ignored.** This is the must-ignore default, and it
is normative: an object shape says nothing whatever about members it does not name, and a value
carrying extra members is admitted unchanged. There is no operator to close an object, and none will
be added. A closed object asserts something about fields nobody has named — that they will never
exist — which is the assertion that turns every additive provider change into a broken consumer build,
and it makes the subsumption rule "extra fields are fine" (RFC) unstatable.

**`forbidden` is the override, and it is per member.** Naming a member `forbidden` asserts its absence:

```json shape
{ "shape": "object",
  "members": {
    "id":    { "shape": "integer", "example": 42 },
    "ssn":   { "shape": "forbidden" },
    "notes": { "shape": "optional", "of": { "shape": "string", "example": "gift wrap" } } } }
```

`id` must be present and an integer; `ssn` must be absent; `notes` may be either; anything else the
provider sends is ignored. Per-member `forbidden` is enough for the case that motivates closing an
object — asserting that PII is not leaked — and it says the useful thing (*this* field must not
appear) instead of the unusable one (*no* field may appear).

**`array`** admits arrays of exactly `entries.length` elements, element `i` admitted by `entries[i]`.
Extra elements are *not* ignored, in deliberate asymmetry with objects: an object member is addressed
by name, so a new one is additive, while an array element is addressed by position, so an extra
element changes what every later position means. Ignoring it would silently reinterpret the contract.

**`each-like`** admits arrays whose length is in `[min, max]` (`min` defaults to 1, absent `max` means
unbounded) and whose every element is admitted by `items`. This is today's array matcher plus explicit
cardinality, and it is the operator that carries a cardinality dimension (§6.4).

**`each-entry`** admits objects whose member count is in `[min, max]`, whose every key is admitted by
`keys` (absent: any key) and whose every value is admitted by `values`. It covers today's `values`,
`eachKey` and `eachValue` matchers, and it is the one structural operator where the must-ignore
default does not apply — there are no unnamed members, because the shape speaks about all of them.
That is the difference between "an object with these fields" and "a map of these things", and the
language should let an author say which they mean.

**`contains`** admits arrays for which each entry shape can be assigned a *distinct* element that
admits it; other elements are ignored. It exists because v3/v4 `arrayContains` exists and must survive
conversion (design 2.5); it is the one core operator whose comparability is opaque (§8), which is
itself a reason to prefer `each-like` when authoring fresh.

### 4.4 Presence and choice operators

**`optional(of)`** admits `admits(of) ∪ { ⊥ }`. **`forbidden`** admits `{ ⊥ }` and nothing else — it
takes no `of`, because a constraint on a value that must not exist is not a thing to write.

**`nullable(of)`** admits `admits(of) ∪ { null }`. `nullable` and `optional` are independent and
compose in one order only (§5.2): `optional(nullable(S))` describes a member that may be absent, null,
or an `S`, and gives two dimensions — presence and nullability — so a test that never sends `null`
never records that it did.

**`any-of`** admits the literal values in `options`:

```json shape
{ "shape": "any-of", "options": ["PENDING", "SHIPPED", "DELIVERED"], "example": "PENDING" }
```

`options` holds **values, not shapes**. A union of arbitrary shapes is not expressible here, and that
is the design: an undiscriminated union makes matching a search, makes the variant space's points
unnameable, and makes `admits(P) ⊆ admits(C)` a pairwise-containment problem over operators whose
containment is already only conservative (§8). A finite literal set is exactly comparable, exactly
enumerable as variant points, and is what enums — the overwhelming real case — actually are.

**`one-of`** is how a union is expressed when the alternatives are structured: a `discriminator`
member name, and `alternatives` mapping a name to an `object` shape that binds the discriminator to a
distinct literal (§5.4):

```json shape
{ "shape": "one-of",
  "discriminator": "type",
  "alternatives": {
    "card":    { "shape": "object",
                 "members": { "type":  { "shape": "equality", "example": "card" },
                              "last4": { "shape": "regex", "pattern": "\\d{4}", "example": "1234" } } },
    "invoice": { "shape": "object",
                 "members": { "type":    { "shape": "equality", "example": "invoice" },
                              "dueDate": { "shape": "date", "format": "yyyy-MM-dd", "example": "2026-08-30" } } } } }
```

`admits` is the union of the alternatives' sets, and the distinct discriminator literals make that
union *tagged*: a value belongs to at most one alternative, decided by reading one member. Matching
therefore reports "payment.type was `cheque`, which is no known alternative" instead of "nothing
matched"; the variant space gets one point per alternative; and subsumption compares alternative to
alternative by name rather than searching for a covering. Requiring the tag is what buys all three.

## 5. Composition rules

A shape document is **well-formed** iff it satisfies the schema and the rules below. An engine MUST
validate well-formedness before compiling, and MUST report violations as `interaction-invalid` with a
`problems[]` entry per violation carrying the path of the offending node (protocol spec §10.2) — the
error quality an SDK user sees is a stated goal of plan task 3.2, and these are the messages they will
see most.

### 5.1 Absence belongs to slots

A shape whose `admits` contains `⊥` — `optional`, `forbidden`, or a component operator that says so —
MAY appear only in a **slot** position: a member of an `object` node, or a part slot that its
transport declares nullable (§3.6). It MUST NOT appear as the root of a part's shape, as the `items`
of an `each-like`, as the `values` or `keys` of an `each-entry`, as an entry of an `array` or
`contains`, or as the `of` of a `nullable`.

The reason is the domain, not taste: `⊥` says "this slot holds nothing", and an array element or a map
value has no slot that can hold nothing — an array of three elements where the second is absent is an
array of two elements, a different value. Allowing it would make cardinality and position ambiguous
exactly where the variant space needs them to be countable.

### 5.2 Presence modifiers stack in one order

`optional` may wrap `nullable`; `nullable` MUST NOT wrap `optional`. Neither may wrap itself, and
neither may wrap `forbidden`. So `optional(nullable(S))` is well-formed and the other three orderings
are not.

The order is fixed rather than normalised so that a shape has one spelling and dimension ids are
stable (§6.2). `optional` outermost is the one that matches the domain: presence is asked first,
because if the slot is empty there is no value to ask about.

### 5.3 `any-of`

`options` MUST be non-empty, its values MUST be distinct under the equality of §4.2, and none may be
`⊥` (absence is `optional`'s job, not an enum member's). `null` MAY be an option — an enum genuinely
containing null is different from a `nullable` enum, and the difference is visible in the variant
space: as an option it is one point among the values, as `nullable` it is a separate dimension.

If `example` is present it MUST be one of `options`; it names the dimension's default point (§6.1). A
single-option `any-of` is well-formed and contributes no dimension; it is what a converter produces
from a one-value enum, and it should not need special-casing.

### 5.4 `one-of`

- `alternatives` MUST hold at least two entries, and each MUST be an `object` shape (possibly wrapped
  in `nullable`).
- Every alternative MUST declare the `discriminator` member, and its shape MUST be an `equality` or an
  `any-of` — the tag must be a literal or a finite set of literals, never an open matcher.
- The alternatives' discriminator value sets MUST be pairwise disjoint. This is what makes the union
  tagged, and it is checkable at authoring time, which is where an ambiguous union should be caught.
- `default` MAY name the alternative that is the dimension's default point; absent, it is the
  lexicographically first alternative name. Lexicographic, not document order, because JSON object
  member order is not preserved by every parser the document will pass through, and a dimension's
  default must not depend on that.

### 5.5 Cardinality

`min` defaults to 1 and `max` is unbounded when absent; `min ≤ max` when both are present. `min` MAY
be 0 — an `each-like` with `min: 0` admits the empty array, and its cardinality dimension has the
empty case as a point, which is the honest way to declare "this list is sometimes empty" (and, like
every dimension, it is then a promise to exercise it).

### 5.6 Depth and size

An engine MAY impose limits on shape depth and node count and MUST report exceeding them as
`interaction-invalid` rather than failing in some other way. A shape tree is authored by a DSL, not
generated adversarially, but an engine that receives one over a pipe still owes its host a named
error instead of a stack overflow.

## 6. Variant dimensions

### 6.1 Dimensions and points

A **dimension** is an axis along which a shape's admitted set is deliberately wider than one case, and
the operators that create width are exactly the operators that carry dimensions (§4.1, last column but
one). A dimension has:

- an **id**, stable and derivable from the shape tree (§6.2);
- an ordered list of **points**, each with a name unique within the dimension;
- a **default point** — the one the shape's own examples exercise;
- optional **gates**: points of other dimensions that must be selected for this one to be active
  (§6.3).

An operator whose width collapses to a single case contributes **no dimension**: a one-option `any-of`,
an `each-like` whose `min` and `max` are equal. A dimension with fewer than two points is not a
dimension, and an engine MUST NOT emit one.

A **variant** is an assignment of one point to every active dimension. Computing the variant space is
this specification's job; choosing which variants to run — pairwise sampling, exhaustive thresholds,
caps, explicit pinning, provider-state linkage — belongs to
[design 2.3](../variant-semantics/spec.md), which consumes the document in §6.6.

### 6.2 Dimension ids

A dimension id is `<path>#<facet>`:

- **`<path>`** locates the operator in the part: the slot name, then a segment per structural step —
  `.<member>` for an object member, `[*]` for `each-like` items, `{*}` for `each-entry` values,
  `{key}` for `each-entry` keys, `[i]` for the `i`th entry of an `array`, and `@<alternative>` on the
  segment where a `one-of` alternative is entered.
- **`<facet>`** names the axis: `presence`, `nullability`, `value`, `alternative`, `cardinality`, or a
  component-declared facet for a component operator.

So `response.body.shippedAt#presence`, `response.body.items#cardinality`,
`response.body.payment@invoice.dueDate#presence`. The facet suffix is what keeps stacked operators
distinct:
`optional(nullable(S))` at one path gives `…#presence` and `…#nullability`.

Ids MUST be stable across engine versions and across a pact-file round trip — design 2.5 records which
variants were exercised by their assignments, and design 2.3 lets users pin a variant by name, so an
id that moves invalidates recorded history. Two rules follow: an id depends only on the path and facet
of its own operator, never on what else the tree contains (adding a member elsewhere renames nothing);
and dimension order in the emitted document is deterministic — depth-first over the tree, object
members and `one-of` alternatives visited in lexicographic order of name, array entries in index
order.

### 6.3 Gating

Dimensions nested under a point that is not selected are **inactive** for that variant, and an
inactive dimension takes no point at all. Two operators gate:

- `optional` — dimensions inside `of` are gated by the `present` point. If the member is absent there
  is nothing inside it to vary.
- `one-of` — dimensions inside an alternative are gated by that alternative's point.

`nullable` gates the same way in principle (`null` has no interior), but its `of` subtree's dimensions
are gated by `non-null`.

Gating is the difference between a variant space that is a product and one that is a sum of products,
and 2.3's sampler needs it as input rather than as a discovered surprise: the number of variants is
not `∏|dimension|` whenever any gate exists, and an assignment that gives a point to an inactive
dimension is not a variant at all.

### 6.4 Per-operator dimensions

| Operator | Facet | Points | Default | Gates its subtree? |
|---|---|---|---|---|
| `optional` | `presence` | `present`, `absent` | `present` | yes, on `present` |
| `nullable` | `nullability` | `non-null`, `null` | `non-null` | yes, on `non-null` |
| `any-of` | `value` | one per option, named by its canonical rendering | the `example`, else the first option | no |
| `one-of` | `alternative` | one per alternative, named by the alternative | `default`, else lexicographically first | yes, per alternative |
| `each-like`, `each-entry` | `cardinality` | `min`; `min+1` when `max > min`; `max` when `max` is finite and distinct from both | `min` | no (§6.5) |
| everything else | — | — | — | — |

Point names for `any-of` are the canonical rendering of the option: a string option is its own text, a
number, boolean or `null` option is its JSON text. Where two options would render alike they are
already not distinct (§5.3), so the names are unique.

The RFC gives `eachLike` "boundary variants (min, min+1)". A finite `max` is added here as a third
point, because a declared upper bound that is never exercised is precisely the sort of undemonstrated
claim the shape language exists to prevent — and because it is the boundary a provider is most likely
to actually break. 2.3 owns the caps that keep this affordable.

### 6.5 Dimensions inside a collection are shared

Dimensions inside `each-like` items or `each-entry` values are **shared across all elements**: one
dimension, one point per variant, applied to every element the variant produces.

The alternative — a dimension per element — is unusable and wrong. Unusable because the element count
is itself a dimension, so the number of dimensions would depend on the point selected for another
dimension. Wrong because `eachLike` means *every element is like this*: an array whose elements differ
in their optional members is a claim about heterogeneity that the operator does not make. An author
who wants heterogeneous elements has `array` (positional) or `contains`.

Sharing is also what keeps the RFC's promise affordable: a list of ten items with two optional fields
contributes two dimensions, not twenty.

### 6.6 The variant-space document

Schema: [`schemas/v1/variant-space.schema.json`](schemas/v1/variant-space.schema.json). This is the
document an engine computes from a shape tree and hands to design 2.3 (and, through
`consumer-session/variants`, to a host):

```json variants
{ "dimensions": [
  { "id": "response.body.status#value", "path": "response.body.status", "facet": "value",
    "operator": "any-of", "default": "PENDING",
    "points": [ { "name": "PENDING", "value": "PENDING" },
                { "name": "SHIPPED", "value": "SHIPPED" },
                { "name": "DELIVERED", "value": "DELIVERED" } ] },
  { "id": "response.body.shippedAt#presence", "path": "response.body.shippedAt", "facet": "presence",
    "operator": "optional", "default": "present",
    "points": [ { "name": "present" }, { "name": "absent" } ] },
  { "id": "response.body.payment@invoice.dueDate#presence",
    "path": "response.body.payment@invoice.dueDate", "facet": "presence",
    "operator": "optional", "default": "present",
    "points": [ { "name": "present" }, { "name": "absent" } ],
    "gated-by": [ { "dimension": "response.body.payment#alternative", "point": "invoice" } ] } ] }
```

The third entry shows a gated dimension: `dueDate` varies only in variants that selected the
`invoice` alternative.

## 7. Matching, producing, recording

### 7.1 Matching is membership

Matching a value `v` against a shape `S` succeeds iff `v ∈ admits(S)`. That is the entire rule, and it
holds in both directions of a contract test: the mock matching an incoming request, and the verifier
matching a provider's response.

Two consequences worth stating because today's model does not have them:

- **No cascading, no precedence.** There is no rule about which of several matchers wins at a path,
  because a path has exactly one shape. The cascading and precedence semantics of v1–v4 live in one
  place only — the compiler that turns old matching rules into shapes and plans (design 3.5).
- **Failures are structural.** A mismatch is reported against the node that rejected the value, with
  the path that located it; the plan (design 2.4) is what makes that inspectable, and `explain` is
  what prints it.

Matching against a variant is narrower: when an interaction is being served or verified under a
variant assignment, each dimensional operator is *pinned* to its selected point — an `optional` pinned
to `absent` admits only `⊥`, an `any-of` pinned to `SHIPPED` admits only that value. This is what
makes variant testing meaningful in both directions: the consumer's mock serves that variant, and the
provider's response is checked against that variant, not against the whole shape.

### 7.2 Producing is choosing

Producing a value from `S` under a variant assignment walks the tree: value operators yield their
`example` (or their generator's output), structural operators assemble their children, a pinned
dimensional operator yields the value its point selects — the option's literal, the alternative's
object, `min`/`min+1`/`max` copies of an item example, `⊥` for an absent `optional`, `null` for a
`nullable` pinned to `null`.

The produced value MUST be admitted by the shape under that assignment. An engine SHOULD assert this
on every production; it is cheap, and it catches the class of bug where a shape and its example
disagree — which is otherwise discovered as a mysterious provider-side failure much later.

### 7.3 What is recorded

A pact file (design 2.5) records the shape once, plus the concrete example produced for each
*exercised* variant. Neither half is sufficient alone: the shape is what the verifier matches against,
and the exercised examples are the evidence that the consumer demonstrably worked against them. A
variant that was declared but not exercised is not recorded as passing — the consumer session withholds
the pact exactly as it does for a failure (protocol spec §8.2), which is the mechanism that makes a
declared dimension a promise rather than a wish.

### 7.4 From shapes to plans

The engine does not interpret shapes directly at match time: it compiles them to plans (design 2.4)
and executes the plans, which is what makes matching inspectable via `explain`. This specification
constrains the compiler in one way only — the plan compiled for a shape MUST accept exactly
`admits(S)` — and says nothing about which actions it uses to do so. The golden corpora (task 3.7)
are where that correspondence stops being a claim: each case is a shape, its expected plan, and the
expected verdicts against captured values.

## 8. Comparability: what a subsumption checker can decide

Design 2.8 owns the subsumption walk, its finding severities and its warn/block policy. What the shape
language owes it is an honest answer to *which comparisons this language can decide*, per operator.
The RFC's phrase is "decidable enough to be useful"; this section is where "enough" gets its edges.

A checker asks `admits(P) ⊆ admits(C)` and MUST answer with one of **yes**, **no**, or **unknown**.
`unknown` is a first-class answer, never a guess in either direction — reporting "review manually" is
useful, reporting a wrong "yes" is how a checker loses its users.

**The identity floor.** If `P` and `C` are structurally identical nodes, `admits(P) ⊆ admits(C)` is
**yes**, whatever the operator, including a component operator the kernel knows nothing about. Every
comparison therefore has a floor, and an unknown operator degrades a check to "unknown only where the
shapes actually differ".

**Comparability classes**, per operator:

| Class | Operators | What the checker can do |
|---|---|---|
| **exact** | `any`, `equality`, `type`, kind predicates, `semver`, `not-empty`, `object`, `array`, `each-like`, `each-entry`, `optional`, `forbidden`, `nullable`, `any-of`, `one-of` | decide `yes`/`no` for every pair of operators in this class, by set containment: literal sets, kind lattice (`integer ⊂ number ⊂ any`), cardinality intervals, presence sets (`{v} ⊂ {v, ⊥}`), member-wise recursion, alternative-wise recursion keyed on discriminator literals |
| **conservative** | `regex`, `datetime`, `date`, `time`, `include`, `content-type` | `yes` on identity, and `yes` where the container is exactly wider (`regex ⊆ string ⊆ any`); otherwise `unknown`. Two different regexes, or two different datetime formats, are `unknown` — not `no` |
| **opaque** | `contains`, component operators with no declared comparability | `yes` on identity; `unknown` otherwise |

The asymmetric findings the RFC lists fall out of the exact class directly: extra provider members are
`yes` (must-ignore, §4.3) unless the consumer marked the member `forbidden`, in which case
`admits(P) ⊄ admits(C)` is decided `no`; a provider `any-of` with more options than the consumer's is
`no`; provider `number` against consumer `integer` is `no`; provider `optional` against a consumer
member that is not optional is `no`, because `⊥ ∈ admits(P) \ admits(C)`. That last one is the
"nullable column" production break, and it is decided by set containment rather than by a special
rule — which is the payoff for putting `⊥` in the domain in the first place (§2.1).

Two notes for 2.8:

- The conservative class is conservative by *policy*, not by mathematics. Containment of RE2-subset
  languages is decidable, and an engine that decides more of it stays conformant — `unknown` is a
  permission, not a requirement, and narrowing it is a pure improvement that changes no recorded
  contract. What an engine MUST NOT do is answer `yes` or `no` by heuristic.
- Component operators SHOULD declare a comparability class and, where they can, a containment
  procedure (design 2.6's interface). A component that declares nothing is opaque, and the check
  degrades locally and visibly rather than globally and silently.

## 9. Evolution and compatibility

The shape vocabulary grows; shapes are recorded in pact files, which are read years later by engines
that were written after them. Two rules govern:

**New operators are additive.** A new core operator is a new value in an open vocabulary
(`x-known-values` grows) and needs no version bump anywhere — but an engine that does not implement it
fails the interaction by name (§3.7), so a *pact file* using a new operator requires an engine new
enough to read it. That is a real constraint on the pact ecosystem and it belongs to design 2.5's
compatibility notes; the shape language's contribution is to make the failure loud and precisely
attributable.

**Operators are never removed or redefined.** An operator's `admits`, once published, is fixed:
redefining it silently changes the meaning of every pact file that already uses it, including files
that no longer have anyone to re-run them. Deprecation (a schema `deprecated: true`, a converter that
rewrites, a lint) is the only retirement path. The schemas here follow the Engine Protocol's
additive-evolution rules (protocol spec §11.2) and the same CI checker enforces them: within
`schemas/v1/`, vocabularies only grow, members are not removed or retyped, and required sets do not
change. A change the checker rejects is either a mistake or a `schemas/v2/`, and the checker forces the
choice to be explicit.

**Variant-space stability** is the third leg (§6.2): dimension ids and point names are as fixed as the
operators, because recorded pacts and pinned variants refer to them by name.
