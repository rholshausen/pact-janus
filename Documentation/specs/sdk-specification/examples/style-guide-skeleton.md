# `<language>` SDK style guide

Copy this file to `sdks/<language>/STYLE.md` and fill in every section once, per spec.md §5. This file
carries exactly what the behavioural specification (`spec.md` §3) deliberately does not: the decisions
that make this SDK feel native to its language. Nothing here overrides the behavioural specification —
where the two disagree, the behavioural specification is the primitive's actual semantics, and this file
records how that semantics is *expressed*, never a different one.

## Naming and module layout

- Case convention for primitive names (e.g. `each-like` -> `eachLike` in TypeScript, `eachLike` in Java).
- Package/module structure: where the idiomatic layer lives relative to the generated-bindings layer
  (spec.md §2), and the rule for never importing generated types outside it without going through the
  idiomatic layer's own surface.

## The async model

- Which native construct represents "a session operation that talks to the engine": `Promise`,
  `CompletableFuture`, a coroutine, a blocking call on a dedicated executor. Name the one this language's
  ecosystem expects, not a compromise that fits every language equally badly.
- How `execute`'s per-variant closure (spec.md §4) composes with that construct — in particular, whether
  the closure itself may be async, and how a rejected/failed closure propagates to fail the variant and
  ultimately the test, without being swallowed.

## The error surface

- The mapping from each engine-protocol spec §10 error category to a language-native failure type: does
  `document`-category errors become one exception type with a `problems` accessor, or several
  language-specific subtypes? Whichever is chosen, every primitive's `errors` list (spec.md §3.2) is
  surfaced through it — no primitive invents its own ad hoc failure shape.
- Whether structured `details` (component identifiers, budget numbers, positions) are exposed as
  strongly-typed accessors or as a generic map, and why.

## Builder ergonomics

- Fluent chain (`.given(...).request(...).response(...)`) vs. a data class plus a free function vs.
  whatever this language's test-DSL conventions already favour (e.g. a JVM DSL commonly reads as nested
  builder blocks rather than a flat chain).
- How a bare literal value in `request`/`response` becomes an `equality` shape (spec.md §3, the `request`
  and `response` primitives) — this MUST be the same rule the behavioural specification states; this
  section only records how it looks in this language's syntax.

## Test-framework integration

- How the per-variant closure becomes multiple test-runner invocations with readable names (Jest's
  `test.each`, JUnit 5's `@TestFactory`/dynamic tests, or an equivalent) — each selected variant SHOULD
  appear as its own named test result, not as one aggregate pass/fail, so a single failing variant is
  addressable without re-running the whole suite.
- Fixture/lifecycle hooks this language's framework expects (`beforeAll`/`afterAll` or equivalents) and
  how they align with session creation and `finalise` (spec.md §3, the `execute` primitive).

## Packaging and distribution

- Package registry, versioning scheme, and how the pinned engine version (task 6.1's binding pipeline
  output) is embedded or declared as a dependency.

## Deviations from the behavioural specification

Required whenever this language's ergonomics force a departure from a primitive's `signature` roles or
`produces` mapping as stated. One entry per deviation:

| Primitive | Deviation | Reason |
|---|---|---|
| _(example)_ `datetime` | format is inferred from the example's own lexical shape rather than requiring an explicit `format` argument | this language's date library cannot cheaply accept a format string separate from a parse attempt; inference rule: `<name the exact rule>` |

An empty table here is a claim — "this SDK deviates from the behavioural specification nowhere" — and
SHOULD be reviewed with the same scrutiny as a populated one: an empty table that turns out to hide an
undocumented deviation is exactly the silent divergence spec.md §5 exists to prevent.
