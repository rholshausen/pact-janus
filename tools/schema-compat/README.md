# schema-compat

The CI checker required by [ADR 0002](../../Documentation/decisions/0002-document-first-protocol-over-frozen-pipes.md)
and specified by the Engine Protocol spec: governance does the job the type system no longer
does. It enforces two rule sets from
[`Documentation/specs/engine-protocol/spec.md`](../../Documentation/specs/engine-protocol/spec.md):

- **`schema-compat lint <dir>`** — the open-world authoring rules (spec §2.2) on every
  `*.schema.json` under `<dir>`: no `enum`, no `additionalProperties: false`,
  `x-known-values` only on open string vocabularies, short type-shaped titles, no remote
  `$ref`, `$id` required.
- **`schema-compat diff <base-dir> <head-dir>`** — the additive-evolution rules (spec §11.2)
  between the published schemas and a proposed change: members and alternatives are never
  removed, `required` sets and constraint keywords are frozen, `x-known-values` only grows.
  New members, new `$defs`, new files and appended combinator alternatives pass.

Exit code 1 with one violation per line (each citing the spec rule) when anything fails.
CI runs `lint` on every build and `diff` against the PR base branch (`.github/workflows/ci.yml`).

Why hand-built rather than adopted: spike 1.1 called this the "Smithy-diff-shaped gap" —
generic JSON Schema diff tools classify breaking changes for *closed-world* schemas, and
OpenAPI-oriented differs (oasdiff and kin) sit on the wrong document model. Neither knows the
project's own contract (`x-known-values` growth, the no-closing keywords, title discipline),
which is the entire point of the check. The rules are few enough that owning them (~300
lines, serde_json only) costs less than bending a general tool.

Known limits, deliberate for now: combinator alternatives are matched by position (append,
don't reorder), and `$ref`s are compared textually, not resolved. Both are fine for the
schema style this repo mandates; revisit if the style grows.
