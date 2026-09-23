# 0022 — A plan fragment replaces its slot's plan, declares a grammar the engine says it reads, and is checked before anything runs

- **Status**: accepted
- **Date**: 2026-09-23
- **Plan tasks**: 8.4
- **Evidence**: [plan-fragment stress test](../plan-fragment-stress-test.md);
  `engine/kernel/tests/fragments.rs`, `engine/kernel/src/plan/fragment.rs`'s tests,
  `engine/component-host/tests/csv_component.rs` (§ plan fragments), `third-party/janus-csv`;
  [ADR 0010](0010-plans-are-renderings-the-grammar-is-the-record.md), [ADR 0012](0012-one-interface-two-bindings.md)

## Context

ADR 0010 made plans renderings and the grammar the record, and named one exception: a
component-contributed fragment crosses a version boundary, because a component is shipped separately
from the engine that reads it. Component-interfaces spec §6.3 let `content/compile` return a fragment
"spliced at path", and §12.3 said an engine "either accepts it … or fails the load naming the skew".
Task 8.4 had the 8.1 CSV component contribute a fragment and a custom action, and asked what happens
when the grammar moves.

Building it showed the spec had left five things open, each of which the stress test needed answered:

- **splice how?** Whether a fragment joins the engine's generic plan for its slot, or replaces it.
- **what version string, and what order?** The grammar was `v0`, with no rule for comparing it to anything.
- **how does a component know what to write?** `component/hello` offered protocol versions, not grammars.
- **what may a fragment contain?** "Core actions and its own" — checked when, and against what list?
- **how does a fragment's action reach `matcher/apply`?** The plan grammar gives an action node children and
  no config, and `Apply` wants `values` and `config`.

It also found a hole nobody had asked about: the core action families look namespaced.

## Decision

**1. A fragment replaces its slot's generic plan.** The reason a content component contributes a
fragment is that its content type changes what an operator means — CSV's `integer` is text that spells
one — and a generic `match:integer` left beside the component's check would still fail. So the fragment
is the slot's whole plan, and the component takes on plan grammar §5.1's obligation for it: accept
exactly what the shape admits, under this content type. A component compiles the shapes it can express
and declines the rest; declining is always safe.

**2. Grammar versions are `v<major>[.<minor>]`, and a reader reads its own major at any minor no newer
than its own.** `v0` is `v0.0`. An additive grammar change is a new minor, anything else a new major. A
fragment **must** declare its version; one that does not cannot be checked for skew, and is refused.
The same rule governs every plan document the engine reads: `plan::from_json` refused nothing before
this and now refuses a grammar it does not read.

**3. The engine says which grammars it reads**, as `plan-grammar-versions` in `component/hello`. A
component writes a fragment in a listed grammar, falls back to an older one it also writes, or
contributes none — and an engine that sends no list gets no fragments.

**4. A fragment is checked when the interaction arrives, and before a verification starts**, beside the
interaction's requirements: its grammar is readable (`grammar-skew`); it is a node of that grammar; it
uses that grammar's core actions and the component's own *contributed* actions only; and every `resolve`
stays inside its slot (`fragment-invalid`). Each failure is `component-unavailable` naming the component,
the slot and the reason — never a mismatch at variant forty.

**5. The core action families are reserved component names.** `match`, `expect`, `check` and `convert`
look like namespaces. A component called `expect` could contribute `expect:unique`, indistinguishable from
core and colliding with the grammar version that adds one. It fails to load, as `component-invalid`.

**6. An action node becomes one `matcher/apply` application** whose first child's value is the value under
test, as a `MatchValue` with the path it came from, and whose further children's values travel as
`config.arguments`. This half is **provisional**: the grammar has no `config`, `Apply` does, and this
mapping is what joins them until one of them changes (Phase 9 finding 24).

## Alternatives considered

- **A fragment joins the generic plan.** Cheaper for authors — a component adds checks rather than
  reproducing a compiler — and useless for the case that motivates fragments: the generic check it sits
  beside still fails. Kept alive in a narrower form below.
- **Operator-level substitution instead of slot-level fragments.** The engine compiles the slot as usual
  and asks the content component, per *value* operator, whether it has its own action for this content
  type (`integer` under `text/csv` → `csv:integer`). The component's obligation shrinks from "compile the
  slot" to "say what one operator means", every structure stays the engine's, and variant pinning keeps
  working (finding 21 disappears). Not chosen here, because it is a new operation in a final spec and 8.4
  is a stress test, not a redesign. It is the likely answer, and it is this ADR's tripwire.
- **Semver strings (`0.1.0`) for grammar versions.** The grammar has no patch level — a change to a node's
  meaning is a major — and the plan schema already says `v0`.
- **Discover the engine's grammar by failing.** A component writes the newest grammar it knows and falls
  back when refused. There is no fallback: the refusal fails the interaction, and the component never
  hears about it.
- **Check fragments when first executed.** Skew would surface as a mismatch on some variant of some
  interaction, reading like a provider bug — the failure mode §2.3 exists to prevent.

## Consequences

Easier: a grammar change has a defined blast radius. A `v0` fragment keeps working on a `v0.x` engine,
a newer fragment on an older engine fails by name at the first interaction, and a component that reads
`plan-grammar-versions` never meets skew at all. CSV consumers can write `integer` and mean it.

Harder: a component that contributes a fragment is a shape compiler for every shape it accepts. The CSV
component's covers one structure and eight value operators, and every operator it did not write is a
shape it must decline. Two limits are recorded and not solved: `compile` is not told the variant, so a
fragment cannot pin dimensional operators, and a variant checked against it can pass where the generic
plan fails (finding 21, demonstrated). And a fragment sees only the decoded document, so it cannot show
decoding as a plan step (finding 22).

**Tripwire.** If a second content component needs a fragment, or finding 21 bites a real consumer, adopt
operator-level substitution and supersede decision 1 — keeping 2 through 5, which hold for any shape of
contribution.
