# 0088 — Derive a provider shape from OpenAPI with `janus derive`, and report every mapping that loses something

- **Status**: accepted
- **Date**: 2026-11-13
- **Plan tasks**: S4.2 (amends subsumption-check spec §2.3; supersedes spike 7.3 §4.2's recommendation,
  which the plan had adopted)
- **Evidence**: [spike 7.3 findings](../../spikes/7.3-type-derived-shapes/FINDINGS.md) §§3–5,
  [ADR 0025](0025-a-provider-shape-entry-may-select-interactions-by-operation.md),
  [ADR 0079](0079-keep-each-likes-minimum-at-one-and-exercise-a-list-that-may-be-empty-with-an-element-first.md),
  [ADR 0086](0086-record-a-provider-shape-from-the-providers-own-tests-by-the-operation-each-test-names.md),
  [ADR 0087](0087-compare-the-built-in-components-operators-by-their-own-matchers-and-a-compare-each-declares.md)

## Context

A provider shape can now be recorded from a provider's own tests (S4.1), matched to a consumer's
interaction by operation (ADR 0025), and decided by the walk with the HTTP component's operators
(S4.7). The cheapest route to a shape, deriving it from the OpenAPI document a provider already
publishes, has no tool. Spike 7.3 built one, measured it and was deleted with the rest of the spike code
(S0.12); its findings are this item's specification.

Three things in the findings no longer fit what has been built since.

- **Selectors.** The spike predates ADR 0025, so it named every entry with a hand-written map to the
  consumer's descriptions, and §2 found that this is what makes a derived shape useless. ADR 0086 added
  `http:path` so that an importer could write an operation's template as OpenAPI writes it.
- **Outcomes.** The spike imported one 2xx response per operation. ADR 0086 decision 4 keeps one
  operation's outcomes apart by status, so a consumer interaction expecting a 404 is selected by the
  provider's 404 and nothing else. An importer that imports one success response leaves every
  consumer's error interaction `not-published`.
- **Cardinality.** Spike 7.3 §4.2 found that an array with no `minItems` gave a `wider-cardinality`
  finding against every consumer that used `eachLike`'s default, and recommended making no cardinality
  claim, which the plan adopted. The shape language cannot write "no claim": `each-like` always has a
  `min`, and the default is 1 (ADR 0079), which claims the list is never empty. The spike's own
  hand-authored document shows the cost: it has no `minItems` on `items`, and the sample provider really
  does answer an empty `items` list, which the recorded shape finds. Writing no claim would hide a true
  finding to avoid a noisy one.

Where the importer lives was decided by spike 7.3 §5 and adopted by the plan: outside the engine and the
component model, a program from one document to another. What is left is which program.

## Decision

1. **`janus derive <document>` writes a provider shape.** It is a subcommand of the `janus` CLI that
   never speaks the engine protocol, as `component push|pull` does not, because deriving is not an engine
   operation. It reads OpenAPI 3.0 and 3.1, as JSON or YAML, and refuses anything else as a usage error
   (exit 2), Swagger 2.0 included, naming what it found. It writes the shape to stdout, or to the file
   `--write` names, with provenance `derived`, and the provider's name from `--provider`, else
   `info.title`. It reports what it lost (decision 6) on stderr, or with the shape under `--output json`.
   A derivation that lost something still did what it was asked, so it exits 0. S4.8 reads an AsyncAPI
   document through the same command, by the document's own `asyncapi` member.
2. **One entry per operation and documented response status**, so a derived shape keeps outcomes apart
   as a recorded one does (ADR 0086 decision 4). Each entry's selector is the request's `method` as
   `equality`, upper-cased; its `path` as `http:path`, whose `template` is the base path (decision 3)
   followed by the path's key; and the response's `status`. A code is `equality`. A range (`4XX`) is
   `http:status-class` with `codes`, the range's statuses less every code the operation documents
   explicitly, since OpenAPI says the explicit code wins, and two entries selecting one interaction is a
   tie the checker does not break (ADR 0025 decision 3). `default` is not imported: written as every
   status no other response names, it would be hundreds of codes in a published document. It is
   reported. An entry's `description` is the operation as the recorder writes it, `GET /orders/{id}`,
   and its `source` is `{ "openapi": "<method> <path>", "operation-id": …, "response": "<key>" }`.
3. **The base path is the `servers` path.** OpenAPI's paths are relative to a server URL, so a consumer
   whose requests go to `https://api.example.com/v1` recorded `/v1/orders/66`. When every server that
   applies to an operation (its own, its path's, else the document's) has the same path, that is the
   prefix. When they differ, none is used, and that is reported. `--base-path` overrides it.
4. **A response's parts are its `status`, its `headers` and its `body`.** `status` is the selector's
   own shape. A documented response header becomes a member of `headers` as the transport presents one,
   a list of field lines (component-interfaces spec §5.9): `each-like` with `min: 1`, `max: 1` and the
   header schema's shape as its item, inside `optional` unless the header is `required`. A response with
   exactly one media type also claims `content-type`, as `http:media-type` of that type, which the walk
   decides (ADR 0087). `body` is the schema of the JSON media type (`application/json`, or one with a
   `+json` suffix). A response whose content is not JSON claims no body, and is reported, until a
   content component's documents can be derived; one with no content claims no body either, since the
   check compares only slots both documents have (subsumption-check spec §2.1).
5. **The schema mapping is spike 7.3 §3's table**, with these changes and additions:
   - **An array with no `minItems` is `min: 0`**, what JSON Schema means by it, and each one is reported,
     so a reader can tell that a `wider-cardinality` finding came from the document's silence.
     `--assume-non-empty` writes such an array as `min: 1` instead, which the report also says.
   - `const` (3.1) is `equality`. An `enum` with `null` among its values is `nullable` around the rest.
   - `additionalProperties` with a schema and no `properties` is `each-entry` of that schema's shape,
     with `min: 0`. Beside `properties`, the members are kept and the schema for the rest is dropped, and
     reported.
   - A `writeOnly` property is left out of a response, since the provider never sends it.
   - A discriminator's literal is the `discriminator.mapping` key whose value is the alternative's
     reference, else the referenced schema's name.
   - A `$ref` is followed within the document. One to another document, or one that recurs, becomes
     `any` where it is met, and is reported.
   - A `pattern` the shape language's regex dialect cannot compile is `string`, and is reported. A
     `format` other than `date-time` and `date` is dropped, and reported once per format and path.
   - An example is the schema's own (`example`, or the first of `examples`) where it has one, and is made
     up for its kind otherwise. Examples are what an `equality` or `any-of` compares; elsewhere they are
     illustration, and the walk does not read them.
6. **Every mapping that loses something is reported**, by `code`, the document's JSON pointer to the
   schema it lost, the entry it is in, and a sentence. A loss is anything that makes the shape admit more
   than the document says, or claim what the document did not: a widening to `any`, a constraint
   dropped (`additionalProperties: false`, numeric and length bounds, `uniqueItems`, formats), a body or
   response not imported, an unstated `minItems`, and an ambiguous base path. The codes are the CLI's
   surface, listed in its help and pinned by its tests.
7. **Provenance is `derived`, and the walk does not care.** Subsumption-check spec §2.3 changes its
   description of `derived` from "imported from types via a content component" to "imported from a type
   document by a program outside the engine, such as `janus derive`".
8. **The protobuf half is S5.2's.** A protobuf shape is useful only to a contract whose body a protobuf
   content component decodes, which S5.2 brings, so it derives one from a `.proto` through the same
   command.

## Alternatives considered

- **No cardinality claim for an absent `minItems`**, as spike 7.3 §4.2 recommended. The only spelling
  is `min: 1`, which claims the list is never empty, so a provider that answers an empty list, as the
  sample provider does, is never reported. Noise the report explains is the lesser cost, and S4.4's
  `provenance` selector is where a team that does not want derived findings to block says so.
- **A separate binary** (`janus-derive`). Its own cadence, and a new release asset with its own build,
  attestation and release check, whose name is an ADR-level surface (ADR 0030). `janus` already ships on
  every target, and `upgrade` is the precedent for a command from one document to another.
- **An engine operation, or a content component's** (spike 7.3 §5 options a and b). OpenAPI is not a
  content type, and deriving touches no engine state.
- **Import only the success response**, as the spike did. Every consumer interaction that expects an
  error is then `not-published`.
- **Write `default` as the statuses no other response names.** Exact, and hundreds of codes in every
  entry a report prints.

## Consequences

Easier: a provider with an OpenAPI document has a shape with one command, matched to every consumer by
operation, with each place it says more or less than the document listed beside it.

Harder: a derived shape now reports `wider-cardinality` on every array whose document is silent,
against a consumer that used `eachLike`'s default, which is O(arrays) noise of the kind spike 7.3
measured. The report names each one, and the flag removes them for a provider that knows its lists are
never empty. A shape is only as good as its `servers` and its templates, and a wrong base path makes
entries nothing selects, which the check reports as `not-published`.

Committed to: the `janus derive` command line and its loss codes are CLI surface, renamed only with a
change file. The importer writes the HTTP component's operators, so a change to `http:path`,
`http:status-class` or `http:media-type` is a change to what it writes.

**Tripwire.** Revisit decision 5's cardinality rule if pilot providers' derived shapes are dominated by
unstated-`minItems` findings that they exempt rather than fix; and decision 2's treatment of `default`
if consumers' error interactions are routinely `not-published` against derived shapes because their
providers document errors only as `default`.
