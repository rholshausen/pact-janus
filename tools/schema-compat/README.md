# schema-compat

The CI checker required by [ADR 0002](../../Documentation/decisions/0002-document-first-protocol-over-frozen-pipes.md)
and specified by the Engine Protocol spec: governance does the job the type system no longer
does. It enforces two rule sets from
[`Documentation/specs/engine-protocol/spec.md`](../../Documentation/specs/engine-protocol/spec.md)
over **every schema under `Documentation/specs/`** — the protocol's own, the shape language's
(task 2.2), and whatever later designs ship. They all cross the same boundary and face the same
version skew, so they all follow the same rules:

- **`schema-compat lint <dir>`** — the open-world authoring rules (spec §2.2) on every
  `*.schema.json` under `<dir>`: no `enum`, no `additionalProperties: false`,
  `x-known-values` only on open string vocabularies, short type-shaped titles, no remote
  `$ref`, `$id` required. Plus the bytes rules (spec §2.4–2.5): a static bytes member is a
  `string` with `contentEncoding: "base64"`, a tagged member's `x-tagged-by` names a sibling
  that is an open string vocabulary offering `base64`, and no member is both.
- **`schema-compat diff <base-dir> <head-dir>`** — the additive-evolution rules (spec §11.2)
  between the published schemas and a proposed change: members and alternatives are never
  removed, `required` sets and constraint keywords are frozen, `x-known-values` only grows,
  and no member flips between text and bytes (`contentEncoding` and `x-tagged-by` are frozen
  like any other constraint). New members, new `$defs`, new files and appended combinator
  alternatives pass.

The bytes rules matter here specifically because nothing else can enforce them:
`contentEncoding` is annotation-only in JSON Schema draft 2020-12, and `x-tagged-by` is this
project's own annotation, so a validator would accept either change silently.

Exit code 1 with one violation per line (each citing the spec rule) when anything fails.
CI runs `lint` on every build and `diff` against the PR base branch (`.github/workflows/ci.yml`).

The crate's integration tests do a second job the binary does not: they validate the specs' worked
examples against the shipped schemas — protocol frame transcripts (`tests/examples.rs`) and shape
documents (`tests/shape_examples.rs`) — so a spec's examples cannot drift from its schemas. Every
```json block in the shape-language spec and its examples must carry a marker (`shape`, `variants`,
`value`, `sketch`); an unmarked block fails the test rather than silently skipping the check.

Why hand-built rather than adopted: spike 1.1 called this the "Smithy-diff-shaped gap" —
generic JSON Schema diff tools classify breaking changes for *closed-world* schemas, and
OpenAPI-oriented differs (oasdiff and kin) sit on the wrong document model. Neither knows the
project's own contract (`x-known-values` growth, the no-closing keywords, title discipline),
which is the entire point of the check. The rules are few enough that owning them (~300
lines, serde_json only) costs less than bending a general tool.

Known limits, deliberate for now: combinator alternatives are matched by position (append,
don't reorder), and `$ref`s are compared textually, not resolved. Both are fine for the
schema style this repo mandates; revisit if the style grows.
