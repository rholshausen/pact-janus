# 0026 — Namespace every plan action, and reserve the core families as namespaces

- **Status**: accepted — decided in task 9.4; implemented as plan grammar v1 in the real build (staged
  plan §5), the prototype keeps v0
- **Date**: 2026-09-24
- **Plan tasks**: 9.4 (amends design 2.4 §4.1, §4.2, §4.4; feeds the staged plan)
- **Evidence**: plan-grammar spec §4.1 and §4.6, [ADR 0010](0010-plans-are-renderings-the-grammar-is-the-record.md),
  [ADR 0022](0022-a-fragment-replaces-its-slots-plan-and-declares-a-grammar-the-engine-says-it-reads.md),
  the [plan-fragment stress test](../plan-fragment-stress-test.md) (task 8.4), `third-party/janus-csv`

## Context

Plan grammar v0 names actions three ways:
- component actions carry the component's namespace: `csv:integer`, `json:parse`;
- most core actions carry a `:`-separated *family*: `match:regex`, `expect:count`, `check:exists`;
- the rest carry nothing at all: `if`, `for-each`, `upper-case`.

Spec §4.1 says the first two look alike and mean different things: "the families a `:` prefix names
are conventions, not scopes … the difference is whether the first segment names a component, and §4.6
is explicit about which is which". That causes three problems:

- **The name does not tell a reader what they are looking at.** In `explain` output, the RFC's main
  inspectability claim, `%match:integer` and `%csv:integer` look like siblings. Only a list in the
  specification says one is core and the other is a third party's.
- **Nothing reserves the family names.** The namespace rule reserves *unnamespaced* names for the
  specification. It says nothing to stop a component calling itself `match`, `expect` or `check`, whose
  actions would then land in a core family.
- **The rule reads backwards.** The core actions with the least governed-looking names (`if`, `length`)
  are the ones the rule protects hardest. The ones that look namespaced (`match:*`) are not namespaced at
  all.

Task 8.4 made this matter. Once components contribute fragments and actions, a plan mixes core and
contributed actions in one tree, and a reader has to tell them apart.

The plan grammar is the RFC's "new specified surface, getting its stability guarantees wrong would be
costly". A rename is a major version (spec §7.1). Today the only thing that depends on v0 is the
prototype's own CSV component, so it will never be cheaper.

## Decision

1. **Every action name is `namespace:name`, with exactly one `:`.** There are no unnamespaced actions.
2. **Six namespaces are reserved for the specification:**
   - `flow`: control, which today has no prefix: `flow:if`, `flow:and`, `flow:or`, `flow:error`,
     `flow:apply`, `flow:for-each`, `flow:tee`;
   - `value`: value transforms, which today have no prefix: `value:join`, `value:join-with`,
     `value:length`, `value:lower-case`, `value:upper-case`, `value:to-string`;
   - `expect`: structural assertions, as today;
   - `check`: boolean checks, as today;
   - `match`: one per value operator of the shape language, as today;
   - `legacy`: v1–v4 semantics that no shape compiles to.

   A component MUST NOT declare a reserved namespace. An engine MUST refuse one that does, at load, by
   name. Further reserved namespaces are added only by a new major version of the grammar.
3. **Legacy-only actions move to `legacy`**, so which compiler may emit an action (spec §4.5) can be
   read off its name:

   | v0 | v1 |
   |---|---|
   | `expect:only-entries` | `legacy:only-entries` |
   | `match:array-contains` | `legacy:array-contains` |
   | `match:min-type` | `legacy:min-type` |
   | `match:max-type` | `legacy:max-type` |
   | `match:min-max-type` | `legacy:min-max-type` |
   | `match:header-value` | `legacy:header-value` |

   `match:*`, `expect:*` and `check:*` are otherwise unchanged. The meaning of every action is
   unchanged: this is a renaming, and nothing else.
4. **This is plan grammar v1.** An engine that reads v1 SHOULD also read v0 fragments, mapping each v0
   name to its v1 name one to one. That is ADR 0022's designed-for case, an older fragment read by a
   newer engine by rule, and it keeps the CSV component working without a rebuild.
5. **Decided now, built later.** The prototype keeps v0: its kernel, corpora and CSV component are
   unchanged. The rename is a stage 1 item in the staged implementation plan. It has to land before the
   real build publishes the plan grammar for third parties, and before `explain` output reaches users.

## Alternatives considered

- **One core namespace** (`core:match-regex`, `core:if`). Uniform and collision-proof, but every node in
  `explain` output then carries the same five letters, and the family, which tells a reader what kind
  of node it is, disappears into the name.
- **A different separator for components** (`csv/integer`), keeping core families on `:`. The smallest
  change, but it leaves two naming schemes and the unprefixed control actions, and a reader still has
  to know the difference between `:` and `/`.
- **Keep v0 and only reserve the family names.** Fixes the collision and nothing else: `%if` and
  `%csv:integer` still look like different kinds of thing, and `%match:integer` and `%csv:integer` still
  look like the same kind.

## Consequences

Easier: a reader of `explain` output knows from any node's name whether the specification or a
component defined it, and whether it carries v1–v4 semantics. Component resolution gains a check that
could not exist before.

Harder: every corpus snapshot, the interpreter's dispatch table, both compilers and every `explain`
example in the documentation change once. That is a mechanical diff, and the corpora make it checkable:
no verdict may change, only snapshots.

Committed to: the six reserved namespaces, and the one-to-one v0 mapping for as long as any engine reads
v0 fragments.

**Tripwire.** Revisit if a seventh core family becomes necessary within v1. That would mean the families
were drawn wrong, better learned before v2 than after. Revisit too if components routinely want a name
the reserved set has taken, such as a plugin wanting `value:`.
