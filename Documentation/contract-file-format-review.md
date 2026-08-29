# Contract file format review

Feeds plan task **2.5**. Status: **review — the evidence behind
[ADR 0011](decisions/0011-contracts-as-self-identifying-json-documents.md), which is the decision**.

Task 2.5 is written in the project plan as "Pact file format v5". This review takes the position that
the name is wrong in a way that matters: what Janus needs is *its own* artifact format, so that the
Pact specification stays free to define a v5 or v6 of its own, and so that this project is free to
record what its design actually produces (shapes, variant assignments, selection reports) rather than
what a v4 pact's shape can be stretched to hold. The rest of this document is about how such an
artifact should be written down, and what each option costs.

It is deliberately a *format* review. What goes in the file — the schema — is the second half of 2.5
and is not decided here.

## 1. The question, split

The prompt for this review posed one dilemma: JSON is easy for tools and the broker but you have to
parse the whole file to learn what it is, whereas front matter tells you what it is immediately but
stops being JSON. That is a real tension, but it is a tension between two *different* decisions that
have been fused. There are four axes here, and they are close to independent:

| Axis | Question | Status |
|---|---|---|
| **A1 — document model** | what data the artifact carries | **fixed** (§2) |
| **A2 — serialisation** | which bytes encode that model | open |
| **A3 — identification** | how a reader learns what the bytes are, cheaply | open |
| **A4 — packaging** | one file or many; is the review unit the publish unit | open |

The "version field ends up at the end" problem is an A3 problem, and it does not require an A2 answer.
Solving it by changing the serialisation — front matter, a magic header, a binary container — pays for
identification with tooling, and the tooling is worth far more. §5 treats A3 on its own, and the
answer there is cheaper than it looks.

## 2. What is already fixed

Three accepted or proposed decisions constrain this before any option is on the table.

**The document model is JSON types plus bytes, and this is not reopenable here.**
[ADR 0002](decisions/0002-document-first-protocol-over-frozen-pipes.md) makes protocol frames
schema-governed JSON documents. The contract file crosses that boundary as a document, not as a blob:
`consumer-session/finalise` returns it as the `pact` member
([protocol spec §8.2](specs/engine-protocol/spec.md)), `verification/verify` takes contracts inline in
its `source`, and `upgrade/pact` is document-in/document-out (§8.4). Protocol §8.1 lists the file
among the documents the protocol carries but does not define, and binds it to the open-world authoring
rules and to §2.4–2.5's bytes forms. [ADR 0006](decisions/0006-bytes-in-the-document-model-and-negotiated-frame-encoding.md)
settles bytes: base64 (RFC 4648 §4, padded), statically declared or tagged by a sibling member.

So the *model* is decided. A serialisation that cannot express that model losslessly is out; a
serialisation that can is, at worst, a second encoding of a model the protocol already carries as
JSON — and every second encoding is a synchronisation obligation forever.

**People read these files, repeatedly, and that is load-bearing.** It is the stated reason ADR 0006
refused to normalise everything to base64 ("a JSON body stored as base64 makes a pact file unreadable
to the humans who read pact files constantly"); the reason variant ids are readable and diffable
rather than ordinal ([variant semantics §2.2](specs/variant-semantics/spec.md)); the reason exclusion
reasons are prose and not expressions ("a document that has to be *evaluated* to be understood cannot
be reviewed in a pact file", §3.9). A format that trades readability for anything else is arguing
against three accepted designs at once.

**The artifact outlives the engine that wrote it.** [Shape spec §9](specs/shape-language/spec.md)
turns on this: shapes recorded today are read years later by engines that did not exist when they were
written. Whatever is chosen has to be parseable by a stranger's toolchain in 2036 with no negotiation
and no shared library.

## 3. What the Pact Broker actually accepts

This was checked against the local `pact_broker` checkout (`fd6e5b63`, 2026-08-07) rather than
assumed, because it turns out to be the single most decisive constraint in the review.

**The OSS broker's publish endpoint hard-codes JSON.** `POST /contracts/publish` validates each
contract with an allowlist on both fields
(`lib/pact_broker/api/contracts/publish_contracts_contract_contract.rb:13-14`):

```ruby
required(:contentType).filled(included_in?: ["application/json"])
required(:specification).filled(included_in?: ["pact"])
```

and further requires that the base64 `content` decode to valid UTF-8, parse successfully, and parse
**to a Hash** — a JSON object, not an array or scalar (`:38-49`). Any non-JSON serialisation is
rejected at the door by today's broker. Not degraded: rejected. (PactFlow reportedly widens
`specification` to accept OAS documents; that is a commercial extension and was not verified here.)

**Everything else the broker does is tolerant.** Within "is a JSON object", it asks for very little:

- It validates `consumer.name` and `provider.name` *inside* the content against the outer names, but
  only when those members are present (`:52-68`). Keeping them at those paths, with those names, is
  what buys correct pacticipant wiring; it is a cheap affordance and worth taking.
- It stores the published bytes **verbatim** as `content` (`lib/pact_broker/pacts/repository.rb:403`),
  so on-disk formatting survives storage.
- Content-based deduplication sorts hash keys alphabetically and SHA1s the result
  (`generate_sha.rb`, `order_hash_keys.rb`), so key order on disk does not affect dedup either way.
- The HTML view degrades rather than failing: content that will not parse as a v1/v2 pact is rendered
  as pretty-printed JSON in a code block under the note *"this contract could not be parsed to a v1 or
  v2 Pact, showing raw content instead"* (`api/renderers/html_pact_renderer.rb:186-191`). A Janus
  contract in today's broker UI is readable, just not rendered as interactions.

**Two consequences worth carrying into the schema half of 2.5.**

1. Retrieval is not byte-identical to publication. `PactDecorator#to_hash` merges the parsed content
   with the resource's `_links` and timestamps (`api/decorators/pact_decorator.rb:12-19`), so a
   contract fetched from a broker has extra top-level members. A reader **must** ignore unknown
   members — the open-world rules already say so, but here it is not hypothetical.
2. Naming the interaction array `interactions` changes the broker's dedup semantics for us. With
   `base_equality_only_on_content_that_affects_verification_results` enabled, the SHA is computed over
   `interactions`/`messages` plus `pact_specification_version` and *nothing else*
   (`pacts/content.rb#content_that_affects_verification_results`). Two contracts differing only
   outside that array dedup to one pact version. That is an argument for putting everything that
   affects a verification run *inside* the interaction objects — including, notably, the per-
   interaction selection report that [variant semantics §4.4](specs/variant-semantics/spec.md) requires
   — rather than in a top-level sibling. Choosing a *different* array name avoids the special-casing
   but then hashes the whole document, so any volatile metadata (timestamps, engine version) produces
   a new pact version on every run. Neither is free; the first is better, and it constrains the schema.

## 4. The candidates

Criteria used below, derived from §2 and §3:

- **C1** expresses the JSON+bytes model losslessly
- **C2** readable and diffable in a pull request
- **C3** deterministic bytes for identical content
- **C4** tolerates unknown members (open-world)
- **C5** identifiable without a full parse
- **C6** publishable to today's broker
- **C7** works with ordinary JSON tooling (`jq`, editors, JSON Schema, every SDK's stdlib)
- **C8** scales to large contracts (size, partial reads)

### 4.1 Single JSON object, with a mandated leading member

One `.json` file, one top-level object, whose first written member identifies the format. The
precedent is broad and boring in the good way: JSON Schema's `$schema`, JSON-LD's `@context`,
CycloneDX's `bomFormat`/`specVersion`, SPDX's `spdxVersion`, SARIF's `$schema`/`version`. All are
long-lived, tool-consumed, self-identifying JSON artifacts — the same category as this one.

- **For**: C1 native. C4, C6 (verified §3), C7 best available — no bespoke reader anywhere, in any
  language, ever. C3 is achievable by *writing rules* rather than by luck: fixed member emission
  order, two-space indent, LF, UTF-8 without BOM, no trailing whitespace, arrays in specified order
  (variant order is already meaningful — variant semantics §4.3, §5.1). C2 is good with those rules,
  which is most of what makes today's pact files reviewable.
- **Against**: C5 is a convention the writer honours, not a property the format guarantees — a
  third-party rewrite (`jq -S`, a formatter, a script) can move the member. §5 shows the residual
  exposure is small and the fallback is cheap. C8 is the honest weakness: a multi-megabyte contract
  with base64 bodies must be parsed whole and diffs badly. No comments — but the artifact is machine-
  written, so the place comments are actually wanted is the hand-authored interaction spec, which is a
  different document and a different task.

### 4.2 Front matter plus a JSON body

`---\nformat: janus-contract/1\n---\n{ … }`, or a `#!`-style magic first line.

- **For**: C5 by construction. Room for comments and provenance above the payload.
- **Against**: C6 fails outright — the broker's content must parse to a Hash, and this does not. C7
  fails everywhere: `jq`, JSON Schema validators, `serde_json`, `JSON.parse` and every SDK's standard
  library need a splitter in front of them, in every language, forever, including languages nobody has
  written an SDK for yet. C1 needs a strip step before the model is reachable.
- **Verdict: reject.** The decisive point is that the gain is *already available for free*. A file
  beginning `{"$format":"janus-contract/1"` has an eleven-byte magic prefix that can be compared with
  `memcmp`, and is still ordinary JSON. Front matter buys a guarantee where a convention plus a
  bounded fallback will do, and pays for it with the entire JSON ecosystem.

### 4.3 YAML 1.2

- **For**: C2 is the best of any option — comments, no quoting noise, block scalars that let a JSON or
  XML body appear as itself rather than as an escaped string. C5 is available via `%YAML`/`---`
  directives. C1 holds, since YAML 1.2 is a JSON superset.
- **Against**: C3 is genuinely hard — there is no canonical YAML emitter, and quoting and style
  choices differ between libraries, so "the same content" does not reliably mean "the same bytes".
  C6 fails. C7 is worse than it looks for a *long-lived* artifact: YAML's implementation variance
  (the Norway problem, implicit typing, duplicate-key handling, resource-exhaustion constructs) is
  survivable when a human wrote the file last week and fatal when an unknown parser reads it in a
  decade. §2's durability requirement points directly away from it.
- **Verdict: not the canonical artifact.** There is a good idea adjacent to it, though: because YAML
  1.2 is a JSON superset, tooling can *accept* YAML for hand-authored inputs while the recorded
  artifact stays JSON. That belongs to the interaction-spec document (task 3.2), not here.

### 4.4 NDJSON / JSON Lines

A header line, then one interaction per line.

- **For**: C5 excellent — one short line, one read. C8 excellent: a verifier can filter interactions
  without parsing the ones it skips, `head` and `grep` work, and files stay appendable. C3 easy.
- **Against**: C2 poor, and it is the requirement that matters most. A whole interaction on one line
  is close to the worst possible diff and is unreadable without a tool. C6 fails (not a JSON object).
  C7 partial: `jq -s` copes, JSON Schema does not apply to the file as a whole.
- **Verdict: reject as canonical.** Its win is streaming, which is an unmeasured need; its loss is
  review quality and the broker, which are certain.

### 4.5 JSON text sequences (RFC 7464)

RS-delimited (`0x1E`) JSON values, with a registered media type.

- **For**: an actual magic byte, real streaming, standardised.
- **Against**: every NDJSON objection plus an invisible control character in a file people are
  expected to read, and near-zero tooling.
- **Verdict: reject.**

### 4.6 CBOR or MessagePack

- **For**: C3 excellent — RFC 8949 §4.2 defines a deterministic encoding, which is more than JSON
  offers natively. C1 native bytes with no base64 inflation. C5 good (CBOR's tag 55799 is a defined
  self-describing prefix, `d9d9f7`). Compact.
- **Against**: C2 fails outright, against the one requirement §2 shows to be load-bearing in three
  separate accepted designs. C6 fails. C7 fails for the casual reader. And ADR 0006 has *already*
  declined a binary encoding on the wire for want of evidence; adopting one for the durable artifact
  would be the same bet with worse odds, since a frame is discarded in milliseconds and an artifact is
  read for years.
- **Verdict: reject as canonical.** Worth holding open exactly as ADR 0006 holds it open for frames —
  as a cache or transport encoding of a model whose canonical form stays textual.

### 4.7 Container archive (zip, or an OCI artifact)

A manifest plus per-interaction and per-body entries.

- **For**: C5 by construction (`PK\x03\x04`, or an OCI media type). C8 excellent — bodies stored raw
  with no base64 tax, random access to a single interaction. Aligns with the OCI component
  distribution model already in the plan (2.6, 8.2).
- **Against**: C2 fails for version control — an opaque blob, no diff, no review, and merge conflicts
  that cannot be resolved. C3 needs an explicit reproducible-archive discipline (zip entries carry
  timestamps and ordering). C6 and C7 fail.
- **Verdict: reject as canonical.** It is the right answer *if* the size tripwire in §7 ever fires,
  and it is worth noting that the project already has an OCI story for components, so the machinery
  would not be new.

### 4.8 Exploded directory bundle

A directory: a manifest at a known path, `interactions/*.json`, bodies as sidecar files in their
native form.

- **For**: C2 is the best of any option for review — one interaction per file means a diff shows
  exactly which interaction changed, a JSON body is readable JSON rather than an escaped string, and a
  binary body is an actual binary file. C8 good. C5 good (a known manifest filename).
- **Against**: C6 fails — it must be packed to publish, so there are two forms, a lossless
  deterministic packing rule, and a permanent obligation to keep them agreeing. It is also not a
  *file* format, which costs CLI ergonomics ("which path do I pass to `pact verify`?") and makes every
  "attach the contract" workflow harder.
- **Verdict: not the canonical form, but the strongest complement to §4.1.** Whether it is in scope
  for 2.5 or deferred is a real decision, listed in §7.

### 4.9 TOML

Deeply nested heterogeneous documents become arrays-of-tables soup, and there is no natural home for
an arbitrary JSON body. **Reject** — one line is all it deserves.

### 4.10 A bespoke text grammar

An HCL- or proto-like `.janus` syntax designed for the domain. This is the option that "it should be
its own file format" most naturally suggests, so it deserves a straight answer rather than silence.

- **For**: C2 potentially the best possible, since the syntax is designed for exactly these
  constructs. C5 by construction. Comments, and a chance to make shapes read well.
- **Against**: we would own a grammar, a parser, a printer, a formatter, an editor mode, and a
  grammar-compatibility policy — **in every SDK language**, against an architecture rule that says
  SDKs are thin and carry no logic. C6 fails. And the readability win of a hand-tuned syntax is
  largest for hand-authored documents; this artifact is machine-written. The document that *is*
  hand-authored is the interaction spec, and it is not this task.
- **Verdict: reject.** "Its own format" should mean its own schema, name, media type and versioning
  line — not its own syntax. Owning the meaning is the goal; owning a parser is a tax.

### 4.11 Summary

| | C1 model | C2 review | C3 determinism | C4 open-world | C5 identify | C6 broker | C7 tooling | C8 scale |
|---|---|---|---|---|---|---|---|---|
| JSON + leading member | ✓ | ✓ | ✓ by rule | ✓ | ~ by convention | ✓ | ✓✓ | ✗ |
| Front matter + JSON | ~ | ✓ | ✓ | ✓ | ✓✓ | ✗ | ✗ | ✗ |
| YAML 1.2 | ✓ | ✓✓ | ✗ | ✓ | ✓ | ✗ | ~ | ✗ |
| NDJSON | ✓ | ✗ | ✓ | ✓ | ✓✓ | ✗ | ~ | ✓✓ |
| JSON-seq | ✓ | ✗ | ✓ | ✓ | ✓✓ | ✗ | ✗ | ✓✓ |
| CBOR | ✓✓ | ✗✗ | ✓✓ | ✓ | ✓ | ✗ | ✗ | ✓ |
| Zip / OCI | ✓✓ | ✗✗ | ~ | ✓ | ✓✓ | ✗ | ✗ | ✓✓ |
| Exploded directory | ✓ | ✓✓ | ✓ | ✓ | ✓ | ✗ | ✓ | ✓ |
| Bespoke grammar | ✓ | ✓✓ | ✓ | ~ | ✓✓ | ✗ | ✗✗ | ~ |

The C6 column looks like it is doing most of the work. §8.0 re-scores the matrix with C6 deleted and
finds that it is not: only §4.8's verdict depends on it.

## 5. Identification, on its own

The concern that motivated this review deserves a direct answer, because it is more tractable than it
first appears.

**The premise, stated precisely.** JSON is not self-delimiting at the front: nothing in the grammar
forces a discriminator to appear early, and JSON objects are semantically unordered, so a
re-serialiser is free to move it. The failure mode is real — pact's own version detection reads
`metadata.pactSpecification.version` and falls back to structural guessing, and that guessing is a
known source of pain in `pact_models`.

**Three things weaken it considerably.**

1. **Serialisation order is ours to fix.** JSON *text* is ordered even though JSON *objects* are not,
   and every mainstream serialiser can be told to emit in a given order. A conformance rule — "a
   writer MUST emit `$format` as the first member" — makes the file begin with a fixed byte prefix.
   `{"$format":"janus-contract/1"` is a magic number that happens to also be valid JSON.
2. **`$` survives alphabetical re-sorting.** `$` is U+0024, below every digit (U+0030+), uppercase
   (U+0041+) and lowercase (U+0061+) letter. A tool that re-serialises with sorted keys — including
   the broker's own SHA path, which does exactly this — puts `$format` *first*, not last. The
   pathological case in the premise is the one case a `$`-prefixed name is immune to.
3. **The fallback is bounded, not a full parse.** A reader that does not find the prefix scans a fixed
   window (say 8 KB) for `"$format"` before giving up. That is a `memchr` over a few kilobytes, not a
   parse of a multi-megabyte document, and it degrades to a clear "this is not a Janus contract"
   rather than to a misparse.

**Recommended layering**, cheapest check first:

| Mechanism | Cost to check | Guarantee |
|---|---|---|
| media type `application/vnd.pact.janus.contract.v1+json` | free when transported over HTTP | strong, but only over the wire |
| filename convention (`*.janus.json`, e.g. `orders-api.janus.json`) | free | weak — a hint, never a decision |
| byte prefix `{"$format":` | `memcmp`, 11 bytes | strong when the file came from a conformant writer |
| windowed scan for `"$format"` | `memchr` over ~8 KB | survives third-party re-serialisation |
| full parse + schema validation | proportional to size | authoritative; required before *use* regardless |

Two reader modes make the contract explicit: **strict** (prefix must match — for pipelines that
control their writers, and for fast rejection) and **tolerant** (windowed scan — for files that have
round-tripped through a broker, a formatter or a script). Identification never substitutes for schema
validation; it only decides *which* schema to validate against, which is precisely the job pact's
current structural guessing does badly.

**A note on `$schema`.** Carrying a `$schema` URL alongside `$format` is worth it for editor
completion and offline validation, and costs nothing — `$format` still sorts first (`f` < `s`). It
should never be *fetched*: it is an identifier, not a dependency.

## 6. Recommendation

**Adopt §4.1 with §5's identification layering.** Concretely:

```json
{
  "$format": "janus-contract/1",
  "$schema": "https://pact.io/janus/schemas/contract/v1/contract.schema.json",
  "consumer": { "name": "orders-ui" },
  "provider": { "name": "orders-api" },
  "interactions": [ … ],
  "metadata": { … }
}
```

with these rules, all of which are format decisions rather than schema decisions and so belong in this
half of 2.5:

- **`$format` is the first member a writer emits.** Value is `janus-contract/<major>`; the major is the
  compatibility line, and the open-world rules govern everything additive within it.
- **`consumer.name` and `provider.name` keep those exact paths**, because the broker validates them
  there (§3) and because every existing tool expects them.
- **The interaction array is named `interactions`**, with the §3 consequence accepted: anything that
  affects a verification run lives inside an interaction object, not beside the array.
- **Canonical writing rules**: UTF-8, no BOM, LF, two-space indent, one specified member order, arrays
  in their specified order, trailing newline, no trailing whitespace. Determinism is not cosmetic —
  it is what keeps broker dedup and git diffs honest.
- **No `metadata.pactSpecification.version`** — it would be untrue and buys nothing (§8.4).
- **Media type** `application/vnd.pact.janus.contract.v1+json`; **filename** `*.janus.json`. Both are
  hints layered over the byte prefix, never the decision.
- **Readers ignore unknown members** at every level, and MUST cope with the `_links` a broker adds.

**Publishing to today's broker** works by declaring `specification: "pact"` and
`contentType: "application/json"` because the allowlist requires it (§3). That is not a claim about
what the format *is* — it is the broker's word for the row it stores, and the alternative is no broker
support at all until the broker changes. The right long-term move is a PR series against the broker
(**not** a two-line allowlist change — see §8.4, which corrects this); the right short-term move is to
fit through the hole that exists, which costs nothing.

**On the name.** The recommendation is to stop calling this a pact file. It is a **Janus contract**:
its own format token, its own media type, its own schema directory, its own version line, and no
implied relationship to the Pact specification's v1–v4 lineage or to whatever v5 the Pact
specification eventually defines. Task 2.5's conversion rules become "v1–v4 pact → Janus contract",
and `upgrade/pact` in the protocol keeps its name because its *input* is a pact. This also removes an
awkwardness that would otherwise sit in the plan forever: Janus cannot define "pact v5" without
claiming authority over a specification governed by the Pact community, which the decision-log README
explicitly places outside these ADRs.

**Defer, with the door open**: the exploded directory form (§4.8) and a binary or container encoding
(§4.6, §4.7). Both are additive to a JSON canonical form; neither is additive to a binary one. That
asymmetry is most of the argument.

## 7. Open questions and tripwires

> All six are resolved in §8. Kept here as the record of what was open and why.

1. **Is the exploded directory form in scope for 2.5?** It is the largest available review-quality
   win, and it is also a second form to keep in sync. A defensible middle: specify the packing rule
   now (so nothing forecloses it), implement it when a real contract is unpleasant to review.
2. **Size tripwire.** If a realistic contract exceeds a few megabytes, or if base64 body inflation
   becomes visible in the 1.7 benchmarks, §4.7 becomes the serious option and this review should be
   revisited. Nothing in the plan has yet produced a contract large enough to know.
3. **Streaming tripwire.** If a verifier ever needs to filter interactions without parsing the whole
   file, §4.4's advantage becomes real. Today it is speculative.
4. **Broker allowlist.** Worth confirming what PactFlow accepts for `specification`, and worth
   raising a widening PR against `pact_broker` early — it is a two-line change to an allowlist, and
   having it landed before Phase 5 would remove the "declare yourself a pact" awkwardness entirely.
5. **Does `metadata` belong at the top level at all?** §3's dedup behaviour argues for keeping it
   small and stable. If it carries anything volatile, dedup either churns or hides real changes,
   depending on the array name chosen.
6. **Schema versioning line.** `janus-contract/1` assumes a single integer major. Whether the shape
   language and variant vocabularies embedded in a contract carry their own version members, or ride
   the contract's, is a schema question with a format-shaped edge; shape spec §9 and ADR 0010 both
   bear on it.

---

## 8. Resolutions

Added after review. The prompt for this round was that the Pact Broker is an OSS project we can
contribute to, so the design should not be scoped by its current constraints unless doing so makes
sense on its own terms. That is right, and it changes less than expected — but it changes the *reason*
for the recommendation, which matters more than changing the recommendation would have.

### 8.0 What relaxing the broker constraint actually changes

Delete column **C6** from the §4.11 matrix entirely and re-read it. Exactly one verdict moves:

| Option | Verdict without C6 | Why |
|---|---|---|
| Front matter + JSON | unchanged — reject | C7 still fails everywhere, in every language, forever |
| YAML 1.2 | unchanged — reject | C3 (no canonical emitter) and durability still fail |
| NDJSON / JSON-seq | unchanged — reject | C2 still fails; one interaction per line is an unreviewable diff |
| CBOR / MessagePack | unchanged — reject | C2 fails against three accepted designs (§2) |
| Zip / OCI | unchanged — reject | C2 fails for version control |
| Bespoke grammar | unchanged — reject | C7 and the thin-SDK rule still fail |
| **Exploded directory** | **becomes competitive** | C6 was its only hard failure |

So the broker was never load-bearing for the *format* choice. §4.11 said C6 "is doing most of the
work"; with the data in hand that is now demonstrably overstated. The case for a single JSON document
rests on **C2** (people review these files — ADR 0006, variant semantics §2.2 and §3.9), **C7** (no
bespoke reader in any SDK language, ever, against a thin-SDK rule), **C3** (deterministic bytes) and
**§2's durability requirement**. Every one of those is internal to this project. The broker agrees with
them by coincidence, not by authority — which is the right relationship to have with someone else's
allowlist.

The distinction to hold onto for the rest of this section is between a **constraint that shapes the
design** (reject those) and a **free affordance that buys compatibility with brokers already deployed**
(keep those). Naming a member `consumer` rather than `consumerDetails` costs nothing and buys
correctness on every broker in the field today. That is not being scoped by the broker.

### 8.1 Q1 — Is the exploded directory form in scope?

**Resolved: it can never be the canonical form, and its projection rule is specified in 2.5 while its
implementation is deferred.**

The reason it cannot be canonical is not the broker. It is that the contract crosses the engine
boundary as a **single JSON document** — the `pact` member of `consumer-session/finalise`, the inline
`source` documents of `verification/verify`, and both ends of `upgrade/pact` (protocol §8.2–8.4, under
ADR 0002). The engine has no filesystem on the WASM path and deliberately keeps I/O out of the kernel;
"a directory" is not something it can return. So the single-document form must exist regardless of what
sits on disk, and an exploded directory is therefore necessarily a **projection** of it, not a rival to
it. Q1 was mis-framed as a format choice; it is really "do we also ship a lossless bidirectional
projection, and when".

Given the size data in §8.2, the answer to *when* is: not yet. At the p95 of contracts that exist
today the single file is a few tens of kilobytes and reviews fine. Specify the projection rule now —
deterministic, lossless, round-trip-tested in both directions — so nothing forecloses it, and build it
when §8.2's tripwire fires or when a real contract is genuinely unpleasant to review, whichever comes
first.

### 8.2 Q2 — The size tripwire, with numbers

Measured against real pact files in the sibling checkouts (190 files under `pacts/` directories,
excluding `node_modules`):

| | today's pacts | ×6 variant multiplier (Janus estimate) |
|---|---|---|
| p50 | 1.2 KB | ~7 KB |
| p90 | 4.8 KB | ~29 KB |
| p95 | 9.3 KB | ~56 KB |
| worst local case | 823 KB (565 interactions, `RustPactVerifier-PactBroker.json`) | ~5 MB |

The multiplier is the estimate, not the measurement: a Janus contract records the shape once but a
concrete request/response example **per exercised variant**, and pairwise sampling over a moderate
space typically yields base + two boundaries + roughly six covering variants. Base64 inflation is a
distant second-order effect by comparison — it costs 33% on binary bodies only, where the variant
multiplier costs 500% on everything. **The variant count is the size story; base64 is not.** That is
worth stating because it points optimisation effort at the right place, and because it means a
container format (§4.7), whose whole advantage is storing bodies raw, is aimed at the smaller problem.

**Tripwires, concretely.** Revisit §4.7 and §4.8 if either holds:

1. a realistic contract exceeds **5 MB**, or
2. base64-encoded bodies exceed **25%** of a contract's bytes (at which point the container format's
   advantage is finally the dominant term rather than a rounding error).

One practical risk to verify separately, since it bites well before either tripwire: brokers deployed
behind a reverse proxy commonly cap request bodies at 1 MB by default. A multi-megabyte publish may
fail with a 413 that has nothing to do with the broker application. This is worth checking against a
real deployment before Phase 5, and it is an argument for gzip on the publish path rather than for a
different file format.

### 8.3 Q3 — The streaming tripwire

**Resolved: no. Closed rather than left open.**

The 823 KB worst-case file parses in **2.1 ms** with CPython's `json` module; `serde_json` is several
times faster again. Extrapolating to the 5 MB tripwire case gives roughly 13 ms in Python and low
single-digit milliseconds in Rust — against verification runs that make real network calls per
variant. There is no plausible contract size at which parse time is visible next to the work the
parse enables, so §4.4's streaming advantage is buying nothing. If §8.2's tripwire ever fires, the
answer is §4.8's projection (which gives per-interaction addressing *and* better diffs), not NDJSON
(which gives addressing and worse diffs).

### 8.4 Q4 — The broker: correcting §6

**§6 was wrong on a point of fact and it needs saying plainly: widening the allowlist is not a
two-line change, and shipping only those two lines would be worse than shipping nothing.**

`ContractToPublish#pact?` is `specification == "pact"`
(`lib/pact_broker/contracts/contract_to_publish.rb:10`), and its single caller is
`lib/pact_broker/contracts/service.rb:133`:

```ruby
pacts = parsed_contracts.contracts.select(&:pact?).collect do | contract_to_publish |
```

A contract whose `specification` is anything else is **silently filtered out**. Adding `"janus"` to the
allowlist at `publish_contracts_contract_contract.rb:14` would make the publish request return 200,
create the pacticipant version, create the tags — and store no contract, create no pact version, fire
no webhook, and add no edge to the matrix. A green CI step that published nothing. That failure mode is
strictly worse than today's, where declaring `specification: "pact"` works correctly.

The actual change set for first-class Janus support in the broker:

| Site | Change |
|---|---|
| `api/contracts/publish_contracts_contract_contract.rb:13-14` | widen both allowlists |
| `contracts/contract_to_publish.rb:10` | replace identity-on-`"pact"` with "is stored as a pact version" |
| `pacts/content.rb` | `interactions`, `provider_states`, `pact_specification_version`, `with_test_results`, `with_ids` all assume pact shape; each needs a Janus-aware path |
| `pacts/generate_sha.rb`, `pacts/sort_content.rb` | a Janus-aware "content that affects verification results" extraction |
| `api/renderers/html_pact_renderer.rb` | a renderer, or accept the verified raw-JSON fallback |

That is a coherent PR series and worth doing — but it is Phase 8/9 work with a review cycle attached,
not a prerequisite.

**The strategic point is version skew, and it does not go away when the PR merges.** Self-hosted
brokers pin versions and upgrade on their own schedule; PactFlow is a separate codebase (grepping the
OSS tree for OAS handling finds nothing — the `specification: "oas"` support is commercial and was not
verifiable here). A Janus contract that only works against a broker version we have not yet shipped
works nowhere for years. So the posture is **compatible by default, enhanced when available**:

- publish as `specification: "pact"`, `contentType: "application/json"` — every broker in the field
  stores it, dedups it, diffs it, fires `contract_content_changed`, and feeds `can-i-deploy`, and the
  UI degrades to readable pretty-printed JSON rather than failing (all verified in §3);
- keep the four free affordances (`consumer.name`, `provider.name`, `interactions`, and the shape of
  `metadata`), because they cost four member names and buy that;
- land the PR series so that newer brokers render and version Janus contracts properly;
- treat `specification: "janus"` as an opt-in a host may send once it knows the broker supports it —
  which the broker's own capability advertisement can answer, rather than the contract file.

None of this is the format being scoped by the broker. The format was already going to be a single
JSON object for the reasons in §8.0; these are member-name choices inside it.

**One affordance to drop, though.** Do **not** emit `metadata.pactSpecification.version`. It would be a
straightforward untruth — a Janus contract does not conform to any pact specification version — and it
buys nothing: `Pacts::Interactions::Types#spec_version` reads it via
`content.pact_specification_version.to_f`, and its absence yields `0.0` (Ruby's `nil.to_f`), taking the
pre-v4 branch, which then finds no `messages` key and returns `has_messages? == false`. Correct, and no
error. Verified by reading `pacts/content.rb:144-148` and `pacts/interactions/types.rb`. Honesty is
free here.

### 8.5 Q5 — Does `metadata` belong at the top level?

**Resolved: yes, and its exclusion from the dedup boundary is a feature.**

The concern was that with `base_equality_only_on_content_that_affects_verification_results` — confirmed
**default `true`** at `lib/pact_broker/config/runtime_configuration.rb:89` — the broker's SHA covers
`interactions`/`messages` and `pact_specification_version` and nothing else, so top-level metadata
changes neither create a pact version nor fire `contract_content_changed` (the broker's own webhook
documentation states this explicitly).

That turns out to be exactly the desired behaviour, because of where the interesting data already
lives. Variant semantics §4.4 puts the selection report **on the interaction**, not on the contract:
"For the interaction as a whole it records the shape once, the selection report (§3.9), and any
exclusions with their reasons." So every part of a contract that affects what a verification run does —
shapes, variants, assignments, exclusions, resolved provider-state parameters, the selection report —
is inside an interaction object by construction, and inside the SHA.

Top-level `metadata` is therefore left holding precisely the volatile material: engine version, writer
version, timestamps, build URL. Excluding that from the dedup boundary means **re-running an unchanged
consumer build produces no new pact version and triggers no provider verification**. Including it would
mean every CI run churned a version and fired a webhook. The coupling introduced by naming the array
`interactions` is not a cost being paid; it is the semantics we would have had to build ourselves.

Rule for the schema half of 2.5, stated positively: **if it changes what a verifier does, it goes
inside an interaction; if it only records how the file came to exist, it goes in top-level
`metadata`.** That rule is worth writing down independently of the broker, because it is a good rule.

### 8.6 Q6 — The version line

**Resolved: one version line on the contract document, and no per-vocabulary version members.**

`$format: "janus-contract/<major>"` is the single compatibility line; the open-world authoring rules
govern everything additive within a major. The temptation is to add version members for the embedded
vocabularies — shapes, variants — and it should be resisted, because each already has a versioning
mechanism appropriate to its own failure mode, and a second one could only disagree with the first:

- **Shapes version by operator name.** Shape spec §9 already settles this: the vocabulary grows, an
  engine that meets an operator it does not know fails the interaction *by name* (§3.7), and an
  operator's meaning is never redefined. That gives per-operator granularity and a precise error. A
  document-level shape version would be coarser and would add a way for the two to contradict each
  other.
- **Variants version by policy name.** The sampling algorithm is already recorded as a named policy
  (`janus-ipog-v1`) in the selection report, and variant semantics §7 requires the name to change
  rather than the algorithm behind it. That is a version number wearing a better name.
- **Plans never appear.** Plan grammar §7 is explicit: "No pact file contains a plan", and ADR 0010
  makes plans renderings rather than records. Nothing to version.

So the contract's own major covers the document's structure, and the vocabularies inside it carry their
own identity in the form each already uses. This also answers the question the RFC raises about
versioning granularity in the affirmative direction: fewer version numbers, each attached to the thing
that actually changes.

### 8.7 What this changes in §6

- The `metadata.pactSpecification.version` affordance is **withdrawn** (§8.4).
- "The right long-term move is a PR widening that allowlist — it's two lines" is **withdrawn and
  replaced** by the change set and skew strategy in §8.4.
- The stated *rationale* for a single JSON document changes from "the broker requires it" to "C2, C3,
  C7 and durability require it, and the broker happens to agree" (§8.0). The recommendation itself
  stands unchanged.
- §7's open questions 1–6 are all resolved above; questions 2 and 3 leave behind numbered tripwires
  rather than open questions, and the reverse-proxy body-size risk in §8.2 is added as a thing to
  verify against a real deployment before Phase 5.

These resolutions are carried into
[ADR 0011](decisions/0011-contracts-as-self-identifying-json-documents.md), which is the decision of
record; this document is its evidence and its alternatives-considered in full.
