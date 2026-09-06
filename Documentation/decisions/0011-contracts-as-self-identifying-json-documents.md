# 0011 — Record contracts as a single self-identifying JSON document, named and versioned independently of the Pact specification

- **Status**: accepted (decision 4's indentation is superseded by [ADR 0018](0018-canonical-contract-bytes-are-compact-not-pretty-printed.md); decisions 1–3 and 5–8 stand)
- **Date**: 2026-08-29
- **Plan tasks**: 2.5 (feeds 3.1, 4.4, 5.5; constrains 2.8)
- **Evidence**: [contract file format review](../contract-file-format-review.md) (candidate
  formats, measured size distribution, verified broker behaviour);
  [ADR 0002](0002-document-first-protocol-over-frozen-pipes.md) (schema-governed JSON documents);
  [ADR 0006](0006-bytes-in-the-document-model-and-negotiated-frame-encoding.md) (bytes and their
  base64 projection); `pact_broker@fd6e5b63` — `api/contracts/publish_contracts_contract_contract.rb`,
  `contracts/service.rb:133`, `pacts/content.rb`, `pacts/generate_sha.rb`,
  `config/runtime_configuration.rb:89`, `api/renderers/html_pact_renderer.rb:186-191`

## Context

Task 2.5 was written in the project plan as "Pact file format v5". Two things are wrong with that, and
they are separable.

The first is authority. The Pact specification is governed by the Pact community; the decision-log
README places naming and governance questions outside these ADRs on purpose. Janus cannot define
"pact v5" without claiming a mandate it does not have, and doing so would also bind the Pact
specification's next version to whatever this prototype happens to need. Both projects are better off
if Janus names its own artifact and leaves v5 and v6 free.

The second is fit. This project records things a v4 pact has no place for — shapes rather than a body
plus a parallel matching-rule map, a variant assignment per exercised example, a selection report that
makes coverage self-describing, exclusions with attributed reasons. Stretching a pact's shape to hold
them would produce a format that is neither a good pact nor a good Janus contract.

What is *not* open is the data model. [ADR 0002](0002-document-first-protocol-over-frozen-pipes.md)
makes protocol frames schema-governed JSON documents, and the contract crosses that boundary as a
document: the `pact` member of `consumer-session/finalise`, the inline `source` documents of
`verification/verify`, and both ends of `upgrade/pact` (protocol §8.2–8.4).
[ADR 0006](0006-bytes-in-the-document-model-and-negotiated-frame-encoding.md) settles bytes. The
kernel has no filesystem on the WASM path, so "a directory" is not something the engine can return.
The single-document form therefore has to exist whatever sits on disk.

That leaves three real questions — which bytes encode the document, how a reader learns what those
bytes are, and what the thing is called — and one temptation to resist: designing around the current
Pact Broker. The broker is open source and we can contribute to it, so it must not scope the design.
The review re-scored every candidate with the broker constraint deleted and found that exactly one
verdict moved (the exploded-directory form). The case for JSON stands on reviewability, deterministic
bytes, universal tooling and thirty-year durability — all internal requirements. The broker agrees by
coincidence, and coincidence is the right relationship to have with someone else's allowlist.

## Decision

**1. The artifact is a Janus contract, not a pact.** Format token `janus-contract/<major>`; media type
`application/vnd.pact.janus.contract.v1+json`; filename convention `*.janus.json`. Task 2.5's
conversion rules become "v1–v4 pact → Janus contract". `upgrade/pact` keeps its protocol name because
its *input* is a pact.

**2. The serialisation is a single JSON object**, one top-level object per contract, per ADR 0006's
document model. The reasons are, in order: people read and review these files constantly — the premise
ADR 0006 used to refuse base64-by-default, that variant semantics §2.2 used to make ids readable, and
that §3.9 used to forbid an expression language in exclusions; no SDK in any language needs a bespoke
reader, against an architecture rule that says SDKs are thin; deterministic bytes are achievable by
rule; and a stranger's toolchain must be able to parse it in 2036, which shape spec §9 requires.

**3. Identification is layered, cheapest check first, and never substitutes for validation.** A writer
MUST emit `$format` as the document's first member, so a conformant file begins with the byte prefix
`{"$format":`. A reader in **strict** mode compares that prefix; in **tolerant** mode it scans a
bounded window (8 KB) for `"$format"`, which survives third-party re-serialisation. Media type and
filename are hints layered above, never the decision. Identification selects *which* schema to
validate against; it never stands in for validating.

The naming is not arbitrary: `$` is U+0024, below every digit and letter, so a tool that re-serialises
with sorted keys — including the broker's own SHA path — moves `$format` to the *front*, not the back.
The failure mode that motivated this review is the one case a `$`-prefixed name is immune to. A
`$schema` URL MAY accompany it (`$format` still sorts first) and MUST NOT be fetched at read time.

**4. Writing is canonical.** UTF-8 without BOM, LF, two-space indent, one specified member emission
order, arrays in their specified order (variant order is already meaningful — variant semantics §4.3,
§5.1), trailing newline, no trailing whitespace. Determinism is not cosmetic: it is what keeps
content-addressed dedup and git diffs honest.

**5. Placement rule: if it changes what a verifier does, it lives inside an interaction; if it only
records how the file came to exist, it lives in top-level `metadata`.** Variant semantics §4.4 already
puts shapes, variants, assignments, exclusions and the selection report on the interaction, so this
mostly ratifies what the design already implies — but it is a good rule on its own terms, and it makes
the contract's content-addressable identity cover exactly the material a re-verification would care
about.

**6. Toward brokers: compatible by default, enhanced when available.** A Janus contract keeps
`consumer.name`, `provider.name` and `interactions` at those paths and names. That costs three member
names and buys correct storage, deduplication, diffing, `contract_content_changed` webhooks and
`can-i-deploy` edges on **every broker deployed today**, with the UI degrading to readable
pretty-printed JSON rather than failing. Hosts publish as `specification: "pact"`,
`contentType: "application/json"` because that is what the field accepts; that is the broker's word for
the row it stores, not a claim about the format. A contract MUST NOT emit
`metadata.pactSpecification.version` — it would be untrue, and absence is already safe (the broker
reads it via `nil.to_f`, takes the pre-v4 branch, and finds no messages).

First-class support is worth contributing upstream, and it is a PR *series*, not an allowlist edit:
widening `specification` alone would make publishing return 200 while `service.rb:133`'s
`select(&:pact?)` silently discards the contract — a green build that published nothing, strictly worse
than today. `specification: "janus"` becomes an opt-in a host sends once it knows the broker supports
it. Because self-hosted brokers pin versions and PactFlow is a separate codebase, the default posture
above is not a transitional measure; it is the design.

**7. One version line.** `$format`'s major is the document's compatibility line, and the open-world
authoring rules govern everything additive within it. Embedded vocabularies carry **no** version
members of their own, because each already has a mechanism suited to its own failure mode and a second
one could only disagree: shapes version by operator name and fail by name when unknown (shape spec §9,
§3.7); the sampler versions by named policy (`janus-ipog-v1`, variant semantics §7); plans never appear
in a contract at all (plan grammar §7, ADR 0010).

**8. Two options stay open, additively.** A lossless bidirectional **exploded-directory projection**
(one interaction per file, bodies in native form) is specified in 2.5 and implemented on evidence — it
is a projection of the single document, never a rival canonical form. A **container or binary
encoding** stays available on the same terms ADR 0006 uses for frames. Both are additive to a JSON
canonical form and neither is additive to a binary one; that asymmetry is most of the argument for
choosing JSON first.

Spec text: the contract file specification (task 2.5) — document identity and canonical writing,
the placement rule, the broker-publication appendix, and the projection rule.

## Alternatives considered

- **Call it "pact v5"** — rejected because Janus has no mandate over the Pact specification, and
  because it would bind that specification's next version to this prototype's needs.
- **Front matter over a JSON body** — rejected because the gain is already free: `{"$format":` is an
  eleven-byte magic prefix that is *also* valid JSON, so front matter buys a guarantee where a
  convention plus a bounded scan suffices, and pays for it with every JSON parser, `jq` invocation and
  schema validator in every language, forever.
- **YAML 1.2** — rejected on determinism: there is no canonical emitter, so "the same content" does not
  reliably mean "the same bytes", and implementation variance is survivable for a file written last
  week and fatal for one read in a decade. (Accepting YAML *input* for hand-authored interaction specs
  is a separate, still-open question belonging to task 3.2.)
- **NDJSON, or RFC 7464 JSON sequences** — rejected because an interaction per line is close to the
  worst possible diff, against the requirement that matters most. Their streaming advantage is
  measurably worth nothing: the largest real pact available locally (823 KB, 565 interactions) parses
  in 2.1 ms in CPython.
- **CBOR or MessagePack** — rejected because the artifact is read by people, and because ADR 0006 has
  already declined a binary encoding for want of evidence; making the same bet on a durable artifact
  rather than a millisecond-lived frame is the same bet at worse odds.
- **Zip or OCI container** — rejected because an opaque blob cannot be diffed, reviewed or merged.
  Held open behind a tripwire; note that its advantage (raw bodies) targets base64's 33% rather than
  the variant multiplier's 500%, so it aims at the smaller half of the size problem.
- **Exploded directory as the canonical form** — rejected because the engine returns a document, not a
  directory, so a single-document form exists regardless and the directory can only ever be a
  projection of it. Retained as decision 8.
- **A bespoke text grammar** — rejected because it would put a parser, printer, formatter and
  grammar-compatibility policy in every SDK language, against the thin-SDK rule, to make a
  machine-written file more pleasant to hand-write. Owning the meaning is the goal; owning a syntax is
  a tax.

## Consequences

**Easier.** Any tool in any language reads a Janus contract with its standard library. Schemas validate
the file directly. `jq`, editors, diff viewers and code review work with no adaptation. Contracts
publish to every broker in the field from day one, with dedup and webhooks behaving correctly. The
naming split lets the Pact specification evolve v5 and v6 without reference to this project, and lets
this project record what its design actually produces.

**Harder.** Binary bodies still pay base64's 33%. A very large contract is a single large file with an
unpleasant diff, and the projection that would fix it is deferred. Publishing declares
`specification: "pact"` until upstream support lands, which is mildly untidy and needs explaining in
the docs. Canonical writing rules are an obligation on every writer, including SDKs and any future
third-party implementation, and need conformance tests to stay real.

**Committed to.** One JSON object per contract. `$format` first, always. Three broker-shaped member
names. The placement rule, which constrains the 2.5 schema. A single version line, which forecloses
per-vocabulary version members without a superseding ADR.

**Tripwires.**

1. A realistic contract exceeds **5 MB**, or base64 bodies exceed **25%** of its bytes → revisit the
   container form and prioritise the projection. Current data: measured p95 across 190 real pact files
   is 9.3 KB, which a ×6 variant multiplier puts at roughly 56 KB.
2. A third-party tool is found rewriting contracts such that tolerant-mode identification fails →
   revisit whether identification needs a stronger guarantee than a convention.
3. Review of a real multi-interaction contract proves genuinely painful before tripwire 1 fires →
   build the exploded projection early.
4. The upstream broker PR series lands *and* the deployed fleet catches up → revisit decision 6's
   `specification: "pact"` default.

Not a tripwire, but a risk to verify before Phase 5: brokers behind a reverse proxy commonly cap
request bodies at 1 MB, which bites well before tripwire 1 and argues for gzip on the publish path
rather than for a different file format.
