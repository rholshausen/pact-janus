# Spike 7.3 findings — Type-derived shapes (provenance #2)

Status: **complete**. OpenAPI 3.0/3.1 imports into provider shapes and the result is usable — but
only after a consumer-side naming file that nothing in the design provides, and only under a policy
that does not block. The RFC's over-broadness worry is **confirmed and quantified**: on the same API
and the same consumer contract, a generator-emitted spec produces **4.5× the findings** of a
hand-authored one, and essentially all of the difference is one finding per field from two specific
generator behaviours. Method: [README.md](README.md).

Headline numbers, one API, one consumer contract, three provider shapes of it:

```
provider shape                       findings  reviews     gaps  by kind
recorded (task 7.2, live provider)          2        0        0  wider-cardinality=1 wider-values=1
derived, hand-authored OpenAPI              2        0        0  wider-cardinality=1 wider-values=1
derived, ORM-generated OpenAPI              9        0        1  broader-type=1 weaker-presence=6
                                                                 wider-cardinality=1 wider-values=1
```

The first row is the ground truth: the provider really can produce `CANCELLED`, and really can
produce an empty list. **A hand-authored spec reproduces it exactly** — same count, same two
findings, and (§4) the recorded shape's cardinality claim is even slightly *tighter* than the
derived one's. Derivation is not inherently noisy. Generators are.

---

## 1. The result in one paragraph

Derivation is worth having, and the RFC is right to rank it below recording. A derived shape from a
hand-authored document is as good as a recorded one for finding real incompatibilities, costs
nothing to produce, and covers operations no test exercised — which recording structurally cannot.
A derived shape from a code-first generator is a different artifact: it describes the *storage
model*, not the response, and the checker faithfully reports every place the storage model is wider
than what a consumer tested. That is not a checker bug and not a mapping bug; the findings are all
true. They are just not *useful*, and there are one-per-field of them.

## 2. The blocking finding: a derived shape cannot be matched to a consumer contract

**This is the finding that would have stopped the spike, and it is not about noise at all.**

Design 2.8 §2.2 matches a provider-shape interaction to a consumer contract's interaction by
**`description` plus state names**. A consumer's description is prose its author wrote — `"a request
for an order"`. An OpenAPI document contains no such thing: it has paths, methods and
`operationId`s (`getOrder`), and no generator will ever emit a string a consumer team happened to
type in a test. So every derived interaction gets `"verdict": "not-published"` — §6.3's *no check
ran* — and the whole mechanism silently produces nothing.

What a team that simply points the importer at its own spec gets (`cargo run -- unmapped`):

```
derived interaction description: "getOrder"
verdict: not-published
findings: 0
--- rendered ---
? web-app was not checked against order-service: no shapes published
  interaction 'a request for an order': the provider has published no shape for it
```

Zero findings, and a line that reads like reassurance.

The spike works around it with `corpus/operation-map.json`, a hand-written
`operationId → { description, states }` file. **Needing that file is the finding.** It cannot be the
answer: it has to be written per consumer (two consumers of one provider name their interactions
differently), it has to be maintained as either side renames things, and a stale entry fails the
way the original problem fails — silently, as `not-published`.

This is not specific to `derived`. It is a property of §2.2's identity, and it affects three of the
four provenances:

| Provenance | Does it know consumer descriptions? |
|---|---|
| `recorded` | Only if the provider's own tests use the consumer's words. Task 7.2's demonstration does — because it was written to. A real provider suite would not. |
| `derived` | No. Structurally impossible. |
| `authored` | Only by copying them. |
| `observed` | No — verification traffic has operations, not descriptions (though a verification *run* does know the contract's descriptions, so this one is recoverable). |

### Options

- **(a) Match on the operation, not the description.** For HTTP, `method` + `path-template`. This is
  what both sides genuinely share, and it is what every other API tool keys on. Design 2.8 §2.1
  forecloses it deliberately — a provider shape "never carries request-direction slots" — but the
  reasoning there is about *subsumption having nothing to add* on the request side, which is not an
  argument against carrying an identity. The cost is that the identity becomes transport-specific,
  which the kernel is not allowed to know: it would have to be an open `operation` document the
  transport component interprets, in the same spirit as the endpoint descriptor (spike 1.5).
- **(b) Match on states plus a `source` cross-reference.** Weak: states are not unique per operation.
- **(c) Keep §2.2 and require the mapping file.** Honest about the cost, but pushes a maintenance
  burden onto exactly the teams the mechanism is meant to help, and fails silently when stale.
- **(d) Let the *checker* fall back**: match by description first, then by an optional `operation`
  member when descriptions do not match, and report which rule matched in the report. Additive to
  §2.2, keeps today's behaviour for recorded shapes, and makes derived shapes work.

**Recommendation: (d), with (a)'s open `operation` document as the member it falls back to.** It is
additive, it leaves the recorded path untouched, and "which rule matched this interaction" is a
thing a report should say out loud anyway — a derived shape matched by path when the descriptions
disagree is a fact a reader wants.

Whatever is chosen, one thing should change regardless: a report whose interactions are *all*
`not-published` currently renders as a polite "no shapes published". For a team that just imported
a spec and expected a check, that line is indistinguishable from success. It should be loud.

## 3. What the mapping can and cannot carry

Exact, no loss:

| OpenAPI | Shape |
|---|---|
| `type: string/integer/number/boolean` | the kind predicate |
| `enum` | `any-of` — the mapping that makes the whole mechanism worth having |
| `type: object` + `properties` + `required` | `object`, non-required members wrapped in `optional` |
| `type: array` + `items` + `minItems`/`maxItems` | `each-like` with the interval |
| `nullable: true` (3.0), `type: [X, "null"]` and `anyOf: [X, null]` (3.1) | `nullable` |
| `format: date-time` / `date` | `datetime` / `date` — OpenAPI means RFC 3339, and the shape language's format-less form means ISO-8601 |
| `pattern` | `regex` (both RE2-subset, both unanchored) |
| `oneOf` **with** `discriminator` | `one-of` |
| `allOf` | merged object |

Cannot be carried, with what it costs:

| Construct | What happens | Why |
|---|---|---|
| `oneOf`/`anyOf` **without** a discriminator | the **whole subtree** becomes `any` | The shape language refuses undiscriminated unions (shape spec §4.4), on purpose and with good reasons. This is the single most expensive gap: one missing `discriminator` keyword erases every constraint beneath it. Code-first generators omit it routinely. |
| `additionalProperties: false` | dropped, with a gap recorded | ADR 0007 commitment 4 refuses closed objects and shape spec §4.3 says no operator will be added. Correct, and worth knowing it is a *silent* widening unless the importer says so. |
| a schema with no `type` | `any` | Nothing to map. Generators emit these for `Any`/`object`/JSON columns. |
| `type: object` with no `properties` | `object` with no members — admits every object | Same. |
| a non-JSON response | the operation is skipped entirely | A content component would own this (§5), and this importer has none. |
| `minimum`/`maximum`/`minLength`/`multipleOf` | dropped | The shape language has no numeric or length range operator. Not a gap this spike hit in practice, but worth recording: the language cannot express a numeric bound at all. |

Run `cargo run -- gaps` for one operation per row.

## 4. The measurement

### 4.1 Each generator behaviour, applied alone to the hand-authored spec

```
generator behaviour, applied alone         findings  by kind
none (hand-authored)                              2  wider-cardinality=1 wider-values=1
no required list                                  7  weaker-presence=5 wider-cardinality=1 wider-values=1
nullable on every field                           8  weaker-presence=6 wider-cardinality=1 wider-values=1
enums flattened to their storage type             2  wider-cardinality=1 wider-values=1
integer reported as number                        3  broader-type=1 wider-cardinality=1 wider-values=1
```

Four things fall out of that table, and they are the substance of this spike:

1. **The two presence pathologies dominate, and they cost exactly one finding per field.** Dropping
   `required` added 5 findings across the 5 consumer-declared fields that were not already optional;
   marking everything `nullable` added 6, one per compared field. Noise from a generator is
   **O(fields)**, not O(endpoints) and not O(complexity) — which is why it is unmanageable on a real
   payload: a 40-field response produces ~40 findings per pathology, and the two compose.
2. **Enum flattening costs nothing in count and everything in quality.** It stays at 2 findings —
   because the enum finding *already existed* — but the finding degrades from
   `provider may produce: 'PENDING' | 'SHIPPED' | 'CANCELLED'` to `provider may produce any string`.
   A team reading the first one learns the provider has a state they never tested. A team reading the
   second learns nothing they can act on. **Counting findings understates the cost of derivation**,
   and any future measurement of this should say so.
3. **Widening `integer` to `number` costs one finding and is genuinely true.** Cheap, honest, fine.
4. **Zero `review`-severity results in every run.** The conservative and opaque classes never fired:
   OpenAPI's formats and patterns map onto the shape language's own, so there was nothing to be
   undecidable about. **The `on-review` axis is untested by this experiment** and this spike is no
   evidence about it either way.

### 4.2 One finding fires even on a perfect spec, and it is a default mismatch

Every row above carries `wider-cardinality=1`, including the hand-authored one. The cause:
**JSON Schema's default `minItems` is 0, and the shape language's `each-like` default `min` is 1.**
A consumer that wrote `eachLike(...)` declared "at least one", and a spec that omitted `minItems`
declared "possibly empty" — so *every* array in *every* derived shape disagrees with *every*
consumer that used the default, forever, whatever the provider actually does.

The recorded shape gets this right for the same API (`min: 0, max: 3` — it saw an empty list *and*
never saw more than three), which is a neat illustration of what evidence buys over declaration.

This one is worth fixing rather than exempting, and it is cheap: an importer should treat an absent
`minItems` as **no cardinality claim at all** rather than as `min: 0`. The shape language has no way
to say "unknown cardinality" — `each-like` always has a `min` — so the honest import is
`min: 0` *with the knowledge that it was never stated*, which argues for the importer suppressing
the claim rather than the checker learning about provenance. **Recommendation to 7.4: an importer
flag, defaulting to "omit cardinality claims the source document did not make".**

### 4.3 Does the noise separate from the signal?

No — not by any selector ADR 0016 has today. In the ORM run, the nine findings include the two true
ones, at the same paths, with the same kinds, in the same interaction. A team cannot write a
`path`-scoped exemption that keeps the real `wider-values` on `$.status` while silencing the
generated `weaker-presence` on `$.status`, because both are on `$.status`. Scoping by `interaction`
or `consumer` silences everything, including the finding that would have caught a real break.

**The only axis on which the noise separates cleanly is the one the report already records:
`provenance`.**

## 5. "Via a content component" does not fit, and probably should not

The plan says to import "into provider shapes via a content component". It does not fit the
interface as designed. Design 2.6 §6 gives a content component four operations —
`decode`, `encode`, `compile`, `detect` — and all four are about *octets and documents at match
time*. "Turn a type schema into shapes" is none of them, and OpenAPI is not a content type: the
JSON content component has no business knowing what `components/schemas` means.

Three places it could live:

- **(a) A fifth operation on the content interface** (`content/derive`). Cheap to add, wrong shape:
  it would put schema-language knowledge behind a content-type key, and `application/json` is not
  the thing that knows OpenAPI.
- **(b) A new component interface** — a *shape source*, with one operation, `derive(document) ->
  { interactions }`. Fits the component model properly: distributed, versioned, sandboxed and
  resolved by the machinery design 2.6 already has, and a Protobuf or Avro importer would be a
  second implementation of the same interface. Cost: a fifth interface in a design that has been
  careful to have exactly four.
- **(c) Out of the engine entirely.** A provider shape is a *file*. The engine already consumes one
  without caring who wrote it, `provenance` is an open vocabulary, and an importer is a
  document-in/document-out program with no engine state — which is precisely the shape of thing
  that should not be a plugin.

**Recommendation: (c) for the prototype and, I think, for the real build.** The interface is the
artifact, which is the whole point of having specified the artifact. An importer that is a separate
tool can be written in any language, ship on its own cadence, and be replaced by a vendor's own
generator that emits the document directly — none of which is true of a component. (b) becomes
worth revisiting only if `janus check --from-openapi` is wanted as a first-class flow, and that is a
CLI convenience that can also be had by shelling out.

This spike's importer is therefore a standalone binary, and nothing in `engine/` knows it exists.

## 6. What this means for the warn/block default (ADR 0016's question)

ADR 0016 defers one decision here by name: whether `provenance` should become a policy selector.
The measurement says **yes, and it is the only selector that works** (§4.3).

1. **The `warn` default is confirmed, and by a wider margin than expected.** A team that imports a
   generator-emitted spec under `on-finding: block` gets a deploy blocked by ~9 findings on a
   six-field payload, of which 2 are real. Extrapolated to a realistic response, that is tens of
   findings on the first run of a mechanism nobody trusts yet. ADR 0016's reasoning — "teaches the
   wrong lesson before the mechanism has earned any trust" — is exactly what the numbers show.
2. **Add `provenance` as a selector on both the policy and its exemptions.** The natural
   configuration the evidence supports is `block` on `recorded`, `warn` on `derived` — a team can
   act on evidence while still seeing what its type schemas claim. Additive, and design 2.8 §7.3
   already describes it as the next step. Without it, the only way to stop derived noise from
   blocking is to stop blocking entirely.
3. **Do not weight or filter by provenance inside the walk.** Design 2.8 §2.3 forbids it and this
   spike found no reason to revisit that: every one of the nine findings is *true*. The problem is
   which true things are worth a person's attention, which is a policy question, which is where the
   selector belongs.
4. **Counting is the wrong headline metric on its own** (§4.1, point 2). A dashboard that shows
   "9 findings" hides that enum flattening silently degraded a precise finding into a vague one.
   Task 7.5's broker notes should carry this: show provenance next to the count.

## 7. A checker bug this measurement found

The first ORM run reported **14** findings, not 9. Five of them were duplicates: a node that admits
both absence and `null` where the consumer admits neither produced *two* `weaker-presence` findings
at the same path, with almost identical text. Design 2.8 §4.2 has one `weaker-presence` row, and a
reader looking at one path wants one answer.

Fixed in the kernel in the same commit as this spike (`subsumption::compare`'s presence
decomposition now emits one finding naming both containments). It is worth recording *how* it
surfaced: the bug was invisible on hand-authored shapes and on recorded ones, because only a
document that marks everything both optional *and* nullable hits the path — which is to say the
noisiest input found a real defect, and would not have been found by the corpus that existed.

## 8. Threats to validity

- **The corpus is synthetic** (README's "honest limits"). Every pattern in the ORM document is one
  real generators emit, but the document was assembled for this experiment. The per-pathology
  decomposition in §4.1 is the part that does not depend on the corpus being representative: it
  measures one behaviour at a time against a fixed baseline.
- **One operation, six fields.** Absolute counts are small. The linearity claim in §4.1 rests on the
  per-field counts matching the field counts exactly, not on a large sample.
- **No `review`-severity results** at all (§4.1, point 4), so half of ADR 0016's surface is
  unexercised by this evidence.
- **JSON OpenAPI only.** Real specs are usually YAML; parsing is not where the interest is, and a
  YAML front end changes nothing in the mapping.
