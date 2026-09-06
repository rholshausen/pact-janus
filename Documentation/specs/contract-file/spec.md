# Janus contract file specification (v1, final)

Plan task: **2.5**. Status: **final**.

A **Janus contract** is the record of what a consumer demonstrated against a mock, written so that a
provider can be held to exactly that and no more. It carries each interaction's shape once and the
concrete evidence of every variant that was exercised, and it is the only thing a verifier reads.

It is deliberately **not** a pact v5. [ADR
0011](../../decisions/0011-contracts-as-self-identifying-json-documents.md) settles why: the Pact
specification is governed by the Pact community, and naming this file "v5" would both claim a mandate
this project does not have and bind that specification's next version to a prototype's needs. It also
settles the format — a single self-identifying JSON object — the identification rules, canonical
writing, the placement rule and the broker posture. The evidence behind those choices, including every
rejected alternative, is the [contract file format
review](../../contract-file-format-review.md). This specification does not re-argue them; it says what
goes in the document.

The schemas under [`schemas/v1/`](schemas/v1/) are the **specified surface**, on the same terms as the
other Phase 2 designs': this prose defines their semantics, the schemas define their shapes, and they
follow the Engine Protocol's open-world authoring rules ([protocol spec
§2.2](../engine-protocol/spec.md#22-open-world-authoring-rules)).

## Contents

1. [Scope and conformance](#1-scope-and-conformance)
2. [The document](#2-the-document)
3. [The header](#3-the-header)
4. [Interactions](#4-interactions)
5. [Shapes and evidence](#5-shapes-and-evidence)
6. [Provider states](#6-provider-states)
7. [Component requirements](#7-component-requirements)
8. [Converting v1–v4 pacts](#8-converting-v1v4-pacts)
9. [Publishing to a broker](#9-publishing-to-a-broker)
10. [Versioning and stability](#10-versioning-and-stability)
11. [Errors](#11-errors)

Worked examples — the RFC's order interaction recorded end to end, and a v3 pact converted with its
findings — live under [`examples/`](examples/), validated by
`cargo test -p pact_janus_schema_compat`.

---

## 1. Scope and conformance

**MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT** and **MAY** are to be interpreted as described in
RFC 2119.

Conformance roles:

- a **writer** produces contracts — in the prototype, the engine's consumer side at
  `consumer-session/finalise`;
- a **reader** consumes them — a verifier, the CLI, a subsumption checker, a broker-side tool;
- a **converter** turns a v1–v4 pact into a contract (`upgrade/pact`).

What this specification owns, and what it hands off:

| Concern | Owner |
|---|---|
| the document's identity, canonical writing and evolution | here |
| the interaction record: description, transport binding, states, parts, requirements | here |
| how shapes and exercised examples are written down | here |
| the encoding of variant-bound state parameters and their resolved values | here (semantics: design 2.3) |
| v1–v4 pact conversion rules and their findings | here |
| what a shape means, which operators exist, `admits` | design [2.2](../shape-language/spec.md) |
| which variants are selected, how they are named, the selection report's contents | design [2.3](../variant-semantics/spec.md) |
| plans — a contract contains none, and never should | design [2.4](../plan-grammar/spec.md) |
| transport and endpoint descriptors, component naming | design [2.6](../component-interfaces/spec.md) |
| the frames a contract travels in | design [2.1](../engine-protocol/spec.md) |

The **interaction specification** an author writes is this document's interaction record minus the
evidence: the same `description`, `transport`, `states`, `parts` and `requires`, with no `selection`.
Plan task 3.2 builds its parser. Keeping one document with one optional half is what stops an author's
input and the engine's output drifting into two schemas that describe the same thing differently.

## 2. The document

### 2.1 What a contract is

A contract is one JSON object. Its interactions each carry a shape — the full space of values the
consumer said it would accept — and, beneath it, the concrete values of every variant that was
actually exercised. Shape without evidence is a claim; evidence without shape is a recording. A
contract is both, which is what lets design 2.8 ask whether a provider's declared shape is admitted by
the consumer's while a verifier separately replays what really happened.

**A contract contains no plan.** Plan grammar §7 is explicit and this specification agrees: a plan is a
rendering of how *this* engine executes a shape, and freezing one into a durable artifact would make
the artifact depend on a compiler version. The shape is the record.

### 2.2 A contract is only written when everything passed

A writer MUST NOT produce a contract unless every interaction verified successfully on every variant
design 2.3's sampling required it to exercise. This is the protocol's rule at
`consumer-session/finalise` (protocol spec §8.2) restated where the artifact is defined, because it is
the property the whole format depends on: everything in the file is demonstrated. A variant that was
required and never ran is `not-exercised`, and withholds the contract exactly as a failure does.

Only exercised variants are recorded, in any form other than the report's counts (variant semantics
§4.4). A contract cannot claim a variant that never ran.

### 2.3 Identity

`$format` carries the format token, `janus-contract/<major>`. A writer **MUST** emit it as the
document's first member, so a conformant file begins with the byte prefix `{"$format":`. JSON Schema
cannot express member order; this rule is the writer's, and a conformance test enforces it.

A reader identifies a contract in one of two modes:

- **strict** — compare the eleven-byte prefix. For pipelines that control their writers, and for
  rejecting non-contracts cheaply.
- **tolerant** — scan a bounded window (**8 KB**) for the member name `"$format"`. For files that have
  round-tripped through a broker, a formatter or a script.

Both modes degrade to a clear "this is not a Janus contract", never to a misparse. Identification
selects *which* schema to validate against; it never substitutes for validating. That split is the
point: today's pact tooling infers a format from the shape of the content it finds, and the
`contract-version-unsupported` cases that produces are the ones this rule exists to remove.

The choice of a `$`-prefixed name is not decoration. `$` is U+0024, below every digit and letter, so a
tool that re-serialises with sorted keys — the Pact Broker's own content-hashing path does exactly this
— moves `$format` to the *front*, not the back. A `$schema` URL MAY accompany it (`$format` still sorts
first, `f` before `s`); it is an identifier and a reader **MUST NOT** fetch it.

Outside the file: media type `application/vnd.pact.janus.contract.v1+json`, filename convention
`*.janus.json`. Both are hints layered above the prefix. Neither is ever the decision, because neither
survives a copy.

### 2.4 Canonical writing

A writer MUST produce: UTF-8 with no BOM; LF only; **compact** — no insignificant whitespace, so no
space after `:` or `,` and no newlines except the one trailing the document; a trailing newline; no
trailing whitespace. Members are emitted in the order this specification lists them, `$format` first.
Arrays are emitted in their specified order — `interactions` as submitted, `variants` in recorded order,
which is run order (variant semantics §4.3, §5.1).

Compact, not pretty-printed: [ADR 0018](../../decisions/0018-canonical-contract-bytes-are-compact-not-pretty-printed.md)
found that a pretty form and §2.3's literal `{"$format":` byte prefix cannot both hold without a special
case for exactly one member, and that the special case buys nothing a machine reader needs — a
deserializer parses either form identically. `$format` first, with no whitespace inserted anywhere,
makes the byte prefix fall out of ordinary compact serialisation with no splice required. A human who
wants to read a contract reformats it with `jq`, an editor's format-on-save, or any JSON formatter;
that reformatting is lossless and is a display concern, never a second canonical form.

Determinism is not cosmetic. The same content written twice MUST produce the same bytes, because a
broker deduplicates on a content hash and a repository diffs on lines: a writer that reorders freely
turns every rebuild into a new contract version and every review into noise.

A reader MUST NOT assume its input is canonically formatted. This specification's own writer emits
compact bytes; a file that reached a reader by another path — round-tripped through a broker, a
formatter, git, or a hand edit — is exactly as valid, and parses the same, regardless of whitespace,
indentation or line breaks (§2.5's open-world rules already cover content; this is the same principle
applied to layout). `IdentifyMode::Tolerant` (§2.3) exists precisely for input whose formatting is no
longer this writer's own.

### 2.5 Reading

Readers follow the open-world rules (protocol spec §2.2): an unknown member is ignored, never rejected,
and SHOULD be preserved when re-encoding. This is not hypothetical here. A contract fetched back from a
Pact Broker has extra top-level members the broker merged in — its `_links` and timestamps — so a reader
that rejects what it does not recognise cannot read its own contracts back.

The one place the rule inverts is an unknown *shape operator*, which fails the interaction by name
(shape spec §3.7). Ignoring an unknown member loses nothing; ignoring an unknown constraint turns a
contract into a weaker contract that still reports success.

### 2.6 The exploded projection

A contract can also be written as a directory, for the one thing a single file does badly: reviewing a
large one.

```text
orders-ui--orders-api.janus/
  contract.json                    the contract with an empty "interactions"
  interactions/
    000-get-an-order.json          one interaction record per file
    001-cancel-an-order.json
```

**Packing** reads `contract.json`, then every `*.json` under `interactions/` in lexicographic filename
order, and uses them as the `interactions` array. **Unpacking** is the inverse. A filename is a
zero-padded index and a slug of the description, so lexicographic order is submission order and the
name says what the file holds; the index is authoritative and the slug is decoration.

The projection MUST be lossless and deterministic in both directions: packing an unpacked contract
reproduces the canonical bytes of §2.4, exactly. A round-trip test is the conformance requirement, and
it is the only one — an implementation that cannot round-trip has not implemented this.

**The directory is a rendering, never a second canonical form.** The engine returns a document, not a
directory: the contract crosses the boundary as the `pact` member of `consumer-session/finalise` and
inline in `verification/verify` (protocol spec §8.2–8.3), and the kernel has no filesystem on the WASM
path. Anything that reads a contract reads the packed form; the directory exists between a writer and a
code review and nowhere else.

Storing bodies as sidecar files — the other half of what a directory could buy — is **out of scope for
v1**. It needs a member that points out of the document and a naming scheme that survives packing, and
the size data behind [ADR 0011](../../decisions/0011-contracts-as-self-identifying-json-documents.md)
says nothing needs it yet: the tripwire that would justify it has a number, and until it fires this
would be a mechanism with no user.

## 3. The header

### 3.1 Consumer and provider

```json sketch
{ "consumer": { "name": "orders-ui" },
  "provider": { "name": "orders-api" } }
```

These member names and the path to `name` are fixed rather than chosen. Every broker deployed today
reads `consumer.name` and `provider.name` at exactly those paths and validates them against the names
in the publish request; keeping them costs two member names and buys correct storage, deduplication and
integration wiring on brokers that have never heard of Janus (§9). That is an affordance, not a
constraint on the design — nothing else in this specification is shaped by it.

### 3.2 Metadata, and the placement rule

**If it changes what a verifier does, it goes inside an interaction. If it only records how the file
came to exist, it goes in top-level `metadata`.**

That rule decides every placement question in this document, and it has a consequence worth stating
plainly: `metadata` is deliberately outside the contract's content-addressed identity. A broker that
hashes only the material affecting verification will not see a metadata change, so **an unchanged
consumer build produces no new contract version and triggers no provider verification**. Putting a
timestamp where it churned a version on every CI run would be a design error, not a convenience.

`metadata` names `writer` (a map of component name to version), `created` (RFC 3339) and `build-url`,
and is open beyond them. A writer aiming for reproducible output — contracts committed to a repository,
say — SHOULD omit `created`, since a per-run timestamp defeats §2.4 for no reader's benefit.

A contract **MUST NOT** carry `metadata.pactSpecification`. It would be untrue: a Janus contract
conforms to no pact specification version. Its absence is safe on today's brokers, which read it
defensively and fall through (§9).

## 4. Interactions

### 4.1 The record

```json sketch
{ "description": "get an order",
  "transport": { "kind": "http", "mode": "passive" },
  "states": [ { "name": "an order exists", "params": { "id": "42" } } ],
  "parts": { "request": { }, "response": { } },
  "requires": [ { "component": "content/protobuf", "min-version": 2 } ],
  "selection": { "variants": [ ], "report": { } } }
```

`description`, `parts` and `selection` are required; a record without evidence is not an interaction
this format can carry (§2.2).

### 4.2 Identity

An interaction is identified by its **`description` together with its `states`** — the pair today's
ecosystem already uses. A contract MUST NOT contain two interactions with the same description and the
same state list; a writer that would produce one fails with `contract-invalid` naming both.

Uniqueness is required rather than merely recommended because other designs address interactions by
name and cannot be ambiguous about which one they mean: variant semantics §6.7's `allow-state-unavailable`
waiver names an interaction and a variant, and design 5.5's `--interaction` filter selects one. A
duplicate description makes a waiver silently apply to two interactions, which is the kind of quiet
widening this project spends its exclusions machinery preventing.

### 4.3 Transport

`transport` is an open descriptor. `kind` is an open string vocabulary (`http`, `https`, `message`);
`mode` distinguishes an interaction the consumer answers (`passive`) from one it emits (`emissive`,
protocol spec §8.2). Everything else in it — host, port, broker and topic details, options — belongs to
the transport component (design 2.6) and this schema does not describe it.

An unknown `kind` is `component-unavailable` naming it, never a silent skip. A verifier that cannot
speak an interaction's transport has not verified that interaction, and saying so is the only honest
outcome available.

## 5. Shapes and evidence

### 5.1 Parts, slots and shapes

An interaction describes **parts**; each part exposes named **slots**; each slot holds one shape
(shape spec §3.6). A contract records that structure directly:

```json sketch
{ "parts": {
    "response": {
      "status": { "shape": "equality", "example": 200 },
      "body":   { "shape": "object", "members": { } } } } }
```

Which parts exist and which slots they expose is the transport and content components' business, never
the kernel's. This specification fixes only the nesting — part name, then slot name, then a shape
document owned by design 2.2 — which is what keeps HTTP out of the kernel (architecture rule B3, plan
task 3.8). There is no `request`/`response` pair in the schema, because a message interaction has
neither and would need a second shape of interaction record if there were.

### 5.2 The selection

`selection` **is design 2.3's variant-selection document** — `variants` plus `report` — with one
addition this design makes: each variant also carries the `parts` it produced. Nothing else about it is
restated here, and `contract.schema.json` deliberately constrains only the addition. Both hold, and the
worked examples are validated against both schemas to prove it.

That reuse is the point. The report is what makes a contract self-describing about its own coverage: a
reader sees that a 24-variant space was covered pairwise under `janus-ipog-v1` by six variants plus two
boundaries, rather than inferring it from the length of a list. Recording the selection *as 2.3's
document* means a contract cannot describe its coverage in terms the engine that computed it would not
recognise.

Each recorded variant carries `id`, `assignment`, its resolved `states` and its `parts`. The id is the
name and the assignment is the truth (variant semantics §2.2); both are recorded so that a variant from
an older contract stays interpretable against a shape that has since grown a dimension. A **label** is
never recorded.

### 5.3 Slot values

A recorded slot value is **always** a wrapper object:

```json sketch
{ "status": { "content": 200 },
  "body": { "content": { "id": "42", "status": "SHIPPED" },
            "encoded": "json",
            "content-type": "application/json" } }
```

`encoded` says how to read `content`: `json` (the default) is the value as it stands; `text` is a
string whose octets are its UTF-8 encoding; `base64` is RFC 4648 §4 octets, per [ADR
0006](../../decisions/0006-bytes-in-the-document-model-and-negotiated-frame-encoding.md).

The wrapper is unconditional, and that is a decision rather than an oversight. The compact alternative
— a bare value, wrapped only when it needs a tag — makes the wrapper *in-band*: a reader meeting an
object with `content` and `encoded` members cannot tell a tagged byte string from user data that
happens to have those members. That is precisely the trap ADR 0007 refused for shapes and variant
semantics §6.2 refused for state parameters, and the answer is the same one: the tag lives in its own
position, always. Uniformity costs a few characters in a machine-written file and removes an
ambiguity that would otherwise be undecidable.

Keeping `json` as the default is what preserves the property ADR 0006 protects: a JSON body is recorded
as JSON — readable in review, addressable by a matcher, diffable line by line — rather than as a base64
blob or an escaped string. Base64 is there for the payloads that need it, including the deliberately
malformed ones a contract test exists to catch.

An **absent** `content` member means the slot was empty. `"content": null` means the slot carried a
null. The distinction matters because shape spec §5.1 makes absence a value in the domain, and a
format that conflated the two would make `optional` unrecordable.

### 5.4 What the two halves are for

The shape and the evidence are read by different steps, and the contract carries exactly what each
needs:

| Half | Read by | Why |
|---|---|---|
| recorded **request** values, per variant | replay | the provider sees the bytes the consumer actually sent, not a re-derivation (variant semantics §5.2) |
| the **shape**, pinned to that variant | response matching | an `optional` pinned to `absent` admits only absence; matching against the whole shape would accept what the consumer never demonstrated |
| recorded **response** values, per variant | diffing, subsumption input, `explain` | evidence of what the consumer accepted, and the only record of it |
| the **assignment** | pinning | it is what "pinned to that variant" resolves against |

A verifier needs nothing else, and in particular re-samples nothing: it replays the recorded variants,
all of them, in recorded order (variant semantics §5.1).

## 6. Provider states

A state carries its literal parameters in `params` and its variant-bound parameters in a sibling
`variant-params` member, whose contents are design 2.3's binding document:

```json sketch
{ "states": [
    { "name": "an order exists",
      "params": { "id": "42" },
      "variant-params": [
        { "name": "shipped",
          "dimension": "response.body.shippedAt#presence",
          "cases": [ { "point": "present", "value": true },
                     { "point": "absent",  "value": false } ] } ] } ] }
```

**Bindings appear once, on the interaction; resolved values appear per variant.** A recorded variant's
`states` carry `name` and `params` only, with every binding already resolved (variant semantics §6.4).
Recording both halves in both places would create two answers to the same question and a way for them
to disagree.

Two rules this design ratifies rather than invents:

- a `dimension` is recorded as the **resolved dimension id** — `response.body.shippedAt#presence`, never
  the author's `shippedAt` — so a verifier resolves nothing and cannot resolve it differently (variant
  semantics §6.3);
- a parameter name MUST NOT appear in both `params` and `variant-params` (variant semantics §6.2).

A parameter with no matching case and no default is **absent** from the resolved `params` — not null,
not empty — so a state handler that treats a missing parameter as a default keeps working.

## 7. Component requirements

An interaction names the components it cannot be matched without:

```json sketch
{ "requires": [ { "component": "content/protobuf", "min-version": 2 } ] }
```

`component` is `<interface>/<name>`; `min-version` is a **major** version and nothing finer. Majors
only, because ADR 0002 made the protocol coarse-grained on purpose and a contract that pins a patch
version is a contract that breaks on a bug fix it wanted.

Requirements live on the interaction, not on the contract, under §3.2's placement rule: they change
what a verifier can do. A verifier collects the union across interactions before it starts, and an
unsatisfiable requirement is `component-unavailable` naming the component — a named failure, never a
degraded run. The plan's `content/protobuf >= 2` is written structurally rather than as that string
because a document that has to be *evaluated* to be understood cannot be reviewed, which is the same
reasoning that keeps an expression language out of exclusions (variant semantics §3.5).

## 8. Converting v1–v4 pacts

### 8.1 What conversion is for

A provider must be able to adopt Janus without waiting for its consumers. Conversion is what makes that
true: an existing pact becomes a contract, verifies through the same plan path as a native one, and the
consumer changes nothing. `upgrade/pact` is document-in, document-out and session-less (protocol spec
§8.4).

Conversion is **not** required to be lossless, and pretending otherwise would be the failure mode here.
It is required to be *honest*: every place the conversion lost something or chose between readings
produces a finding (§8.4), and `janus upgrade` shows them.

### 8.2 Matching rules become shapes

A v1/v2 pact keys its rules by path; a v3/v4 pact keys them by category and path. Both resolve to "this
slot, this position, this constraint", which is where a shape node goes.

| v1–v4 matcher | shape | notes |
|---|---|---|
| *(no rule at a position)* | `equality` on the example | what v1–v4 already mean by an example with no rule; the conversion writes it down rather than changing it |
| `equality` | `equality` | |
| `type` | `type` | |
| `regex` | `regex` | anchoring is unchanged — a pattern matches *anywhere* in the string, exactly as v1–v4 behave, because implicit full-match would be the better default in isolation and would silently change the verdict of every converted pact (shape spec §4.2) |
| `include` | `include` | |
| `number`, `integer`, `decimal` | `number`, `integer`, `decimal` | |
| `boolean`, `null` | `boolean`, `null` | |
| `date`, `time`, `timestamp` | `date`, `time`, `datetime` | format strings carry across character for character — the shape language's pattern vocabulary is the Java `DateTimeFormatter` subset v3/v4 already use (shape spec §4.2) |
| `semver` | `semver` | |
| `notEmpty` | `not-empty` | |
| `contentType` | `content-type` | |
| `min`, `max`, `minmax` on an array | `each-like` with `min`/`max` | |
| `values` | `each-entry` over values | |
| `eachKey`, `eachValue` | `each-entry` over keys / values | |
| `arrayContains` | `contains` | |
| `statusCode` | `http:status-class` | a component operator, not a core one — the kernel does not know what a status class is (shape spec §3.5) |

Two constructs need judgement rather than a row:

- **Several rules at one path combined with `AND`.** Where one rule subsumes the other, the narrower
  survives and the conversion reports a `note`. Where neither does, the shape language has no
  intersection operator: the converter keeps the first rule and reports `rule-unmapped` as `lossy`.
- **`combine: "OR"`.** A disjunction of constraints denotes the union of what each admits. The shape
  language expresses that only where the alternatives are value sets it can name; where it cannot, the
  converter keeps the first rule and reports `rule-combination-or` as `lossy`.

**Generators** map onto the shape's `generator` member where the generator kind has an equivalent, and
produce `generator-dropped` where it does not. A dropped generator is lossy but not dangerous — it
weakens what the contract can *produce*, never what it *accepts*.

### 8.3 The single example becomes the sole variant

A converted interaction has one variant. Its shape has no variant dimensions, so the space is `size: 1`
with `dimensions: 0`, the base variant is the only variant, and every strategy agrees on it. The
recorded variant is `id: "base"` with an empty `assignment`, and its `parts` are the pact's request and
response examples.

There is no special case anywhere for this. Variant semantics §9 makes the same point from the other
side: the degenerate contract falls out of the arithmetic rather than branching around it, which is the
test of whether the arithmetic was right.

**Where converted rules do contribute dimensions.** Not every conversion is that clean. A `min` on an
array becomes an `each-like` with a minimum, and that operator contributes a cardinality dimension
(shape spec §6), so the space is larger than one while the pact still demonstrates a single example.
The honesty rule (§2.2) settles what to do: exactly one variant is recorded, the selection is `base`
alone, `strategy` is `base-only`, and the report says `selected: 1` against a `space.size` above 1.

That refines variant semantics §9 rather than contradicting it. §9 describes the fully degenerate case
and says `size = 1`; the invariant that holds for *every* conversion is `selected = 1`. A converted
contract is honestly under-covered, its own report says by how much, and the way to fix it is to run
the consumer's suite under Janus — which is the incentive the migration path wants anyway.

### 8.4 Findings

Schema: [`schemas/v1/upgrade-findings.schema.json`](schemas/v1/upgrade-findings.schema.json). Each
finding carries a `code` from an open vocabulary, a `kind`, a JSON pointer into the **source pact** —
the file the author is looking at — prose addressed to that author, and optionally a `target` pointer
into the converted contract.

`kind` separates three things that would otherwise be read as one severity: `lossy` (the contract now
says less than the pact did), `judgement` (the converter chose between readings the pact did not
distinguish), and `note` (neither, and still worth reading).

| Code | Kind | Raised when |
|---|---|---|
| `example-frozen-as-equality` | `note` | a position had no rule, so its example became an `equality` constraint |
| `rule-unmapped` | `lossy` | no shape equivalent, including unresolvable `AND` combinations |
| `rule-combination-or` | `lossy` | `combine: "OR"` the shape language cannot name |
| `rule-narrowed` | `judgement` | a rule mapped onto a strictly narrower operator |
| `generator-dropped` | `lossy` | a generator with no shape equivalent |
| `content-type-inferred` | `judgement` | the pact declared no content type and one was inferred from the body |
| `body-not-parsed` | `judgement` | a body could not be parsed as its declared content type and was carried as bytes |
| `state-params-untyped` | `note` | v3 state parameters carried across as-is; the format types them, the pact did not |
| `duplicate-description` | `lossy` | two interactions collided under §4.2 and were disambiguated |
| `interaction-dropped` | `lossy` | an interaction could not be converted at all |

An empty `findings` list is a claim, and a strong one: the conversion was exact.

## 9. Publishing to a broker

**Compatible by default, enhanced when available.** A contract is a self-contained JSON object, so
every Pact Broker in the field stores it, deduplicates it, diffs it, fires `contract_content_changed`
and feeds `can-i-deploy` — and the broker UI, which cannot render it as interactions, degrades to
readable pretty-printed JSON rather than failing.

A host publishing to a broker declares `specification: "pact"` and `contentType: "application/json"`,
because those are the values the publish endpoint accepts. That is the broker's word for the row it
stores, not a claim about the format, and it is what makes Janus usable against brokers nobody has
upgraded yet. First-class support — a `janus` specification value the broker understands end to end —
is worth contributing upstream and is scoped in the format review §8.4; it is a change to several
sites, not an allowlist edit, and self-hosted brokers pin versions for years. So the default posture
above is the design, not a transitional measure.

Three affordances make the default work, and they are the only places broker compatibility touches this
specification:

| Affordance | What it buys |
|---|---|
| `consumer.name`, `provider.name` at those paths | the publish request validates; pacticipants wire up correctly |
| the interaction array named `interactions` | the broker hashes the material that affects verification and nothing else, so §3.2's placement rule gets the deduplication semantics it wants for free |
| no `metadata.pactSpecification` | a defensive read falls through cleanly, and the file tells no untruth |

One operational note that is not a format concern but bites before any format limit does: brokers
behind a reverse proxy commonly cap request bodies well below the size a large contract can reach. The
answer is compression on the publish path, not a different file format.

## 10. Versioning and stability

**One version line.** `$format`'s major is the document's compatibility line, and the open-world
authoring rules govern everything additive within it. A new optional member, a new value in an open
vocabulary, a new part or slot name: all additive, no bump. Removing a member, renaming one, or
changing what an existing one means: a major bump, and the checker in
[`tools/schema-compat`](../../../tools/schema-compat/README.md) enforces the difference in CI.

**Embedded vocabularies carry no version members of their own**, because each already has a mechanism
suited to its own failure mode and a second one could only disagree with the first:

- **shapes version by operator name.** An engine meeting an operator it does not implement fails the
  interaction and names it (shape spec §3.7); an operator's meaning is never redefined (§9). That is
  per-operator granularity with a precise error, which a document-level shape version could only
  coarsen.
- **variants version by policy name.** `janus-ipog-v1` denotes one algorithm forever; a different
  algorithm gets a different name (variant semantics §7). A version number wearing a better name.
- **plans never appear** (§2.1).

**Recorded artifacts outlive the code that wrote them.** That is the reason for every rule in §2.3 and
§2.4, and the reason the format is text: a contract written today must be readable by a toolchain
nobody has written yet, with no negotiation and no shared library.

## 11. Errors

Codes are the protocol's (protocol spec §10, `engine-error.schema.json`). These two used to be
`pact-invalid`/`pact-version-unsupported`, predating the naming decision in ADR 0011; they are
renamed here to `contract-invalid`/`contract-version-unsupported` while the prototype is still in its
design phase and no engine implementation exists to dispatch on the old names (protocol spec §10.2) —
the "renaming breaks every reader" cost this note used to weigh against is zero today and grows once
Phase 3 starts, which is exactly why the fix belongs now rather than later.

| Condition | Code |
|---|---|
| not a Janus contract, or unreadable as one | `contract-invalid` |
| `$format` names a major this engine does not implement | `contract-version-unsupported` |
| structurally invalid — missing required member, duplicate interaction identity (§4.2), a name in both `params` and `variant-params` (§6) | `contract-invalid`, with `problems[]` positions |
| a shape operator the engine does not implement | `interaction-invalid`, naming the operator (shape spec §3.7) |
| a required component missing or too old (§7) | `component-unavailable`, `details.component` naming the requirement |
| a variant id that is not in the contract | `variant-not-found` |

`problems[]` entries carry an RFC 6901 pointer into the contract. A reader that cannot say *where* a
contract is wrong has not diagnosed it — these files are read by people, and an error that names a
position is the difference between a fix and a bisect.
