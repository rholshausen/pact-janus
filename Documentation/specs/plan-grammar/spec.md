# Plan grammar and core action set specification (v1)

Plan task: **2.4**; grammar v1 by staged plan item **S0.1** ([ADR
0026](../../decisions/0026-every-plan-action-is-namespaced-and-core-namespaces-are-reserved.md)). Status: **final**.

A shape says what a value must be; a **plan** is what the engine actually executes to find out, and
what `explain` prints when someone asks why a match succeeded or failed. This document specifies the
node grammar, the value model, the core action set, the two text forms, how a shape compiles to a
plan, the golden-corpus format, and the versioning policy the RFC lists as an implementation unknown.

It is written against a working prototype rather than from nothing: [ADR
0004](../../decisions/0004-fork-v2-engine-as-kernel.md) forked the v2 matching engine from
pact-reference as the kernel's starting point, and §9 records exactly what this specification takes
from it and what it does not. The versioning decision is [ADR
0010](../../decisions/0010-plans-are-renderings-the-grammar-is-the-record.md).

The schemas under [`schemas/v1/`](schemas/v1/) are the **specified surface**, on the same terms as the
other Phase 2 designs': this prose defines their semantics, the schemas define their shapes, and they
follow the Engine Protocol's open-world authoring rules
([protocol spec §2.2](../engine-protocol/spec.md#22-open-world-authoring-rules)).

**Why this is v1, and what v0 was.** The prototype's grammar was `v0`: a real compatibility surface —
plugins author fragments against it — that had had no external consumer, so calling it v1 would have
claimed a stability nobody had tested. Plan task 8.4 was the test. A third-party component contributed
fragments and actions, the versioning policy held, and one thing did not: v0 named its actions three
different ways, and a reader of `explain` could not tell a core action from a component's (ADR 0026).
v1 is v0 with that fixed and **nothing else changed** — every node kind, every result, every action's
meaning is the same, and only names differ. §4.7 is the whole difference, as a table an engine reads v0
documents by. [`schemas/v0/`](schemas/v0/) stays, as the record of what a v0 document is.

## Contents

1. [Scope and conformance](#1-scope-and-conformance)
2. [Plans](#2-plans)
3. [The text forms](#3-the-text-forms)
4. [The action set](#4-the-action-set)
5. [Compiling shapes to plans](#5-compiling-shapes-to-plans)
6. [Golden corpora](#6-golden-corpora)
7. [Versioning and stability](#7-versioning-and-stability)
8. [Errors](#8-errors)
9. [What this takes from the v2 engine](#9-what-this-takes-from-the-v2-engine)

Worked examples — the RFC's order payload compiled and executed, and a corpus case end to end — live
under [`examples/`](examples/), validated by `cargo test -p pact_janus_schema_compat`.

---

## 1. Scope and conformance

**MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT** and **MAY** are to be interpreted as described in
RFC 2119.

Conformance roles:

- A **compiler** turns an interaction specification (shapes, per design 2.2) or a v1–v4 pact
  (matching rules) into a plan.
- An **interpreter** executes a plan against resolved values and annotates it with results.
- A **renderer** prints a plan, executed or not, in the text forms of §3.
- A **component** contributes actions and plan fragments under its own namespace (design [2.6](../component-interfaces/spec.md) owns the
  interface; this document owns what an action *is*).

In scope: the node grammar, the value model, the core action set, the text forms, the shape→plan
compilation obligation, the corpus format, and versioning.

Out of scope, with owners:

| Question | Owner |
|---|---|
| the shapes a plan is compiled from, and `admits` | design [2.2](../shape-language/spec.md) |
| which variant a plan is compiled under, and how variants are selected | design [2.3](../variant-semantics/spec.md) |
| how a plan document crosses the engine boundary | design [2.1](../engine-protocol/spec.md) |
| how a component ships an action or a fragment | design [2.6](../component-interfaces/spec.md) |
| the v1–v4 cascading and precedence semantics the legacy compiler implements | design 3.5 |
| generators — producing a value rather than matching one | designs [2.6](../component-interfaces/spec.md), 4.3 |
| what the CLI prints and which flags select it | design 5.5 |

## 2. Plans

### 2.1 Nodes

A plan is a tree. Every node has a **kind**, zero or more **children**, and — once executed — a
**result** (§2.3). The grammar defines eight kinds:

| Kind | Carries | Executes by |
|---|---|---|
| `container` | a label | executing its children; its result is their conjunction |
| `action` | a name (§4) | applying the named action to its children |
| `value` | a literal (§2.2) | yielding the literal |
| `resolve` | a path | resolving the path against the interaction context |
| `resolve-current` | a path | resolving the path against the current item on the iteration stack |
| `pipeline` | — | applying each child to the next, yielding the last; each child's value becomes the current item for the next |
| `splat` | — | executing its children and replacing itself with their results |
| `annotation` | text | nothing — it is not executable |

Two of those exist for reasons worth stating, because a reader will otherwise take them for
incidental. **`container` is what makes a plan inspectable**: it has no semantics beyond conjunction
and grouping, and it exists so that `explain` can print `:"query parameters"` above the nodes that
check them. Deleting containers would leave a plan that executes identically and explains nothing.
**`annotation` is not executable at all**, for the same reason: a plan is a document people read.

`splat` is the variadic escape: an action whose argument count depends on the value being matched —
every entry of an object, every element of an array — takes a `splat` child that expands at execution
time. `resolve-current` is its companion, addressing whatever the enclosing construct has made
current: the element a `flow:for-each` is on, or the running value of a `pipeline`. Those are the only two
constructs that make anything current, and a `resolve-current` outside both is a compiler bug (§8).

There is no empty or null node kind. The forked engine has one as a Rust `Default`; it is an artifact
of the implementation language, not a concept in the grammar, and a node that means nothing has no
place in a document that gets printed.

### 2.2 Values

A `value` node carries a value from the **Engine Protocol's document model** (protocol spec §2.4) —
`null`, boolean, number, string, array, object, bytes — and nothing else, plus exactly one plan-only
kind:

- **`entry`**: a key paired with a value, which iteration over an object's members needs and which the
  document model has no term for.

That is the whole value model: seven document kinds plus `entry`. The forked engine has fourteen, and
the difference is the point of §9 — `json`, `xml`, multi-valued string maps and string lists are all
content- or transport-specific, and a kernel value model that names them has already lost the
argument B3 makes. A decoded JSON body *is* a document; a set of HTTP headers *is* an object whose
members are arrays of strings. Neither needs a kind of its own.

Bytes in a plan document use the protocol's bytes forms (protocol spec §2.4–2.5), so a plan carrying
a malformed payload round-trips through a frame like anything else.

### 2.3 Results

Executing a node produces one of three results:

| Result | Meaning |
|---|---|
| `ok` | the node succeeded and produced no value |
| `value` | the node succeeded and produced a value |
| `error` | the node failed, with a message and the path that located it |

A `container`'s result is the conjunction of its children's: `error` if any child errored, otherwise
`ok`. This is what makes a plan's root result the interaction's verdict, and what makes any subtree's
result a meaningful sub-verdict — which is what `explain --executed` shows.

An `error` is a value, not an exception. Execution does not stop at the first one: an interpreter MUST
execute every node whose inputs are available, so that one run reports every mismatch rather than the
first. This is the plan-level statement of the protocol's errors-are-values rule (protocol spec §10.1)
and it is what makes a failure report worth reading.

### 2.4 Execution

An interpreter walks the tree depth-first, left to right. Children are executed before the node that
owns them, except where an action is **lazy** — `flow:if` evaluates its condition before its branches,
`flow:or` stops at the first success — and every lazy action MUST declare it (§4.2), because a reader of an
executed plan needs to know whether an unannotated node was skipped or never reached.

Values reach the plan through **resolvers**. A `resolve` node holds a path — `$.query`,
`$.body.items`, `$.headers.content-type` — resolved against the interaction context by the transport
and content components that own those parts. The kernel does not know what a header is; it knows how
to ask.

### 2.5 Determinism

**Compiling the same inputs with the same engine MUST produce the same plan**, node for node, in the
same order: no map iteration order, no hashing, no clock, no randomness. Ties in any ordering decision
are broken by declaration order in the source shape or matching rules.

This is the property golden corpora depend on (§6) — a compiler that emits structurally equivalent
plans in varying order makes every corpus case flaky, and a flaky corpus is worse than none.

It is deliberately *only* the within-a-version property. Across versions a plan may legitimately
differ, and §7 is where that is stated and bounded.

## 3. The text forms

### 3.1 The pretty form

Nodes are written with a sigil, children in parentheses, one per line, indented two spaces:

| Kind | Sigil | Example |
|---|---|---|
| `container` | `:` | `:query-test`, `:"query parameters"` (quoted when it contains whitespace) |
| `action` | `%` | `%expect:empty` |
| `value` | none | `'a string'`, `42`, `true`, `NULL` |
| `resolve` | `$` | `$.query` |
| `resolve-current` | `~>` | `~>.name` |
| `pipeline` | `->` | `->` |
| `splat` | `**` | `**` |
| `annotation` | `#{ }` | `#{'the shipped-order case'}` |

```text
(
  :query-test (
    :"query parameters" (
      %expect:empty (
        $.query,
        %value:join (
          'Expected no query parameters but got ',
          $.query
        )
      )
    )
  )
)
```

This is a **specified surface**, not a debugging convenience. It is what `explain` prints, what a
golden corpus records (§6), and what a user reads when a verification fails, so it is governed like a
schema: the sigils and layout above are fixed, and a renderer MUST produce exactly this form.

### 3.2 The executed form

The executed form is the same tree with ` => <result>` appended to every node that produced one.
Nothing else differs — one renderer, two modes, so a user comparing a plan with its execution is
comparing like with like.

```text
(
  :query-test (
    :"query parameters" (
      %expect:empty (
        $.query => {'a': 'b'},
        %value:join (
          'Expected no query parameters but got ' => 'Expected no query parameters but got ',
          $.query => {'a': 'b'}
        ) => 'Expected no query parameters but got {\'a\': \'b\'}'
      ) => ERROR(Expected no query parameters but got {'a': 'b'})
    ) => BOOL(false)
  ) => BOOL(false)
)
```

A node with no ` => ` was **not executed**. That is a fact worth reading — it means a lazy branch was
skipped or an earlier failure left the value unavailable — and it is why §2.4 requires laziness to be
declared rather than incidental.

### 3.3 Styling on a terminal

A host printing either form to a terminal MAY style it, colouring each piece by what it is: a sigil,
a value, a result. Styling only wraps the text: with it removed, what remains MUST be exactly the
form above. The form is never styled where it is recorded or read by a program — the
`verification/explain` result, the `verification/executed-plan` event, a corpus snapshot, a file, a
pipe — and a host honours its platform's conventions for turning styling off and on (`NO_COLOR`,
`CLICOLOR_FORCE`). The engine never styles; it does not know where its text is going.

## 4. The action set

### 4.1 Naming and namespacing

**An action name is `namespace:name`, with exactly one `:`.** Both halves are kebab-case. There are no
unnamespaced actions. The namespace says who defined the action, and that is the first thing a reader
of `explain` output needs to know about a node:

- **Six namespaces are reserved for this specification:** `flow`, `value`, `expect`, `check`, `match`
  and `legacy` (§4.2). An action in one of them is a core action, defined here, or it is nothing: an
  engine MUST NOT resolve a name in a reserved namespace to a component.
- **Every other namespace is a component's.** A component contributes actions under its own identifier:
  `protobuf:decode`, `http:status-class`, `csv:integer`. The requirement that pulls the component in
  travels in the interaction spec, not in the plan node.
- **A component MUST NOT declare a reserved namespace**, and an engine MUST refuse one that does, at
  load, naming it (component-interfaces spec §2). Without that, a component called `expect` could
  contribute `expect:unique`, indistinguishable from a core action and colliding with the grammar
  version that adds one.
- **An unknown action is a named failure**, never an ignored node (shape spec §3.7's rule, applied
  here): silently skipping a node drops a constraint while still reporting success.

The rules mirror the shape language's (shape spec §3.5), for the same reasons, with one difference:
the shape language reserves the *unnamespaced* names and a plan has none. A reserved set that is a
short list of namespaces can be checked when a component loads; a rule about the absence of a prefix
could not stop a component naming itself after a core family, which is the hole v0 had.

Further reserved namespaces are added only by a new major version of the grammar (§7.1). Within v1 the
set is these six, so a namespace outside it is a component's to take and stays that way.

### 4.2 Core actions

| Namespace | What it holds | Actions | Lazy |
|---|---|---|---|
| `flow` | control | `flow:and`, `flow:or`, `flow:if`, `flow:error`, `flow:apply`, `flow:for-each`, `flow:tee` | `flow:or`, `flow:if` |
| `value` | value transforms | `value:join`, `value:join-with`, `value:length`, `value:lower-case`, `value:upper-case`, `value:to-string` | |
| `expect` | structural assertions | `expect:object`, `expect:array`, `expect:empty`, `expect:not-empty`, `expect:count`, `expect:size`, `expect:entries`, `expect:absent` | |
| `check` | boolean checks | `check:exists`, `check:equals`, `check:null` | |
| `match` | one per value operator of the shape language (§4.3) | `match:*` | |
| `legacy` | v1–v4 semantics no shape compiles to (§4.4) | `legacy:only-entries`, `legacy:array-contains`, `legacy:min-type`, `legacy:max-type`, `legacy:min-max-type`, `legacy:header-value` | |

**Every core action takes at least a fixed number of children**, and a node with fewer is malformed:

| Least children | Actions |
|---|---|
| 0 | `flow:and`, `flow:or`, `flow:apply`, `flow:tee`, `value:join` |
| 1 | `flow:error`; `value:join-with`, `value:length`, `value:lower-case`, `value:upper-case`, `value:to-string`; `check:exists`, `check:null`; `expect:object`, `expect:array`, `expect:empty`, `expect:not-empty`, `expect:absent`; `match:any`, `match:string`, `match:number`, `match:integer`, `match:decimal`, `match:boolean`, `match:null`, `match:datetime`, `match:date`, `match:time`, `match:semver`, `match:any-of`, `match:contains`; `legacy:only-entries` |
| 2 | `flow:if`, `flow:for-each`; `check:equals`; `expect:count`, `expect:entries`; `match:equality`, `match:type`, `match:regex`, `match:include`, `match:content-type`; `legacy:array-contains`, `legacy:header-value` |
| 3 | `expect:size`; `legacy:min-type`, `legacy:max-type` |
| 4 | `legacy:min-max-type` |

The first child is the value under test wherever there is one. The engine's own compilers never emit a
short node, so this is a rule about documents somebody else wrote: an engine MUST refuse a fragment
that contains one (component-interfaces spec §6.3), and an interpreter that meets one anyway MUST give
the node an `error` result naming the action and the count (§8). It MUST NOT fail in any other way —
the protocol's errors-are-values rule (protocol spec §10.1) holds for a malformed plan as it does for a
mismatch.

The `check:` family is what the `expect:` and `match:` families are not: a check **yields a boolean**,
an assertion **fails**. Both are needed and they are not each other's negation in any useful sense — a
presence *branch* asks a question and carries on either way, while a `forbidden` member makes a claim
that can be wrong. Every conditional in §5.2 takes a `check:` action as its condition, and no `match:`
or `expect:` action ever appears in one; an assertion in a condition position would fail the
interaction while deciding a branch, which is not a thing a reader could be expected to predict.

`expect:count` asserts an exact size and `expect:size` a range, with `NULL` for an unbounded end.

`expect:object` and `expect:array` assert the *kind* a structural operator admits (§5.2). They are
assertions rather than matches because that is what they are — a structural property of the value,
like its size — and they are not per-operator actions of the sort §4.3 rules out: `object` and
`each-entry` both emit `expect:object`, `array`, `each-like` and `contains` all emit `expect:array`,
and neither action tells a reader which operator produced it. Without them a structural operator's
plan asserts nothing about a value of the wrong kind, which §5.1's obligation does not allow: an
`object` all of whose members are `optional` would otherwise accept the number `7`, and an
`each-like` with `min: 0` would accept a string, because a string has a length and splats to
nothing.

### 4.3 The matching family

There is one `match:` action per **value** operator of the shape language (shape spec §4.1):

| Shape operator | Action |
|---|---|
| `any` | `match:any` |
| `equality` | `match:equality` |
| `type` | `match:type` |
| `string` `number` `integer` `decimal` `boolean` `null` | `match:string`, `match:number`, … |
| `not-empty` | `expect:not-empty` (a structural assertion, not a match) |
| `regex` | `match:regex` |
| `datetime` `date` `time` | `match:datetime`, `match:date`, `match:time` |
| `include` | `match:include` |
| `content-type` | `match:content-type` |
| `semver` | `match:semver` |
| `any-of` | `match:any-of` |
| `contains` | `match:contains` |

The **structural and dimensional** operators — `object`, `array`, `each-like`, `each-entry`,
`optional`, `forbidden`, `nullable`, `one-of` — have **no action of their own**. They compile to plan
*structure*: containers, control actions and assertions (§5).

That split is the design, not an accident of the table. An `optional` compiled to a `match:optional`
action would hide the presence decision inside an opaque node, and the whole claim of the plan model
is that a user can see what the engine will do. Compiled to structure, the presence check is a node
with a name, a result and a line in the output.

### 4.4 The legacy family

The v1–v4 compiler (design 3.5) emits from the same set. Most of it overlaps: `match:regex` serves
both compilers with one semantics, matching anywhere in the value (shape spec §4.2), and the v1–v4
compiler emits a pact's pattern anchored, `^(?:…)$`, because today's verifiers match it against the
whole value (staged plan S1.18). The pattern a plan prints is the one it matches. Where the two
compilers differ is small and worth naming:

The actions only the v1–v4 compiler may emit are the `legacy` namespace, and nothing else is in it, so
"which compiler may emit this" (§4.5) is read off a node's name. Two actions go the other way:

| Action | Emitted by | Note |
|---|---|---|
| `legacy:only-entries` | legacy only | v1–v4's request bodies are closed by default; response bodies (like every shape) are must-ignore — design 3.5 verified this asymmetry against the 803 specification test cases rather than assuming a single default, and only the request side ever emits this action. No shape compiles to it either way — ADR 0007 commitment 4 refuses closed objects, and §5.3 makes that a checkable rule rather than a convention. |
| `legacy:array-contains` | legacy only | the old `arrayContains`; shapes reach it through the opaque `contains` operator |
| `legacy:min-type`, `legacy:max-type`, `legacy:min-max-type` | legacy only | v1–v4's `MinType`/`MaxType`/`MinMaxType`: a type check plus a size bound, enforced only when the resolved value is an array — a string's characters and an object's members are not items — and never emitted bounded for a cascaded rule, which the legacy compiler reduces to `match:type` (design 3.5's write-up of exactly why). The shape language has no single operator combining the two — `type` and `each-like`'s cardinality are separate operators there. |
| `legacy:header-value` | legacy only | v1–v4's default (no matching rule) header comparison — not plain string equality: a MIME-shaped value compares type and parameters as a set (order- and case-insensitive on the parameter values, extra actual parameters allowed), a comma-separated one tolerates whitespace around the commas, anything else is exact and case-sensitive. Shapes match a header's value with the ordinary value operators (§4.3) instead. |
| `match:any-of`, `expect:absent` | shapes only | v1–v4 has no enumeration or absence assertion |

This is why the forked action set is worth keeping rather than replacing: it is the legacy half of the
answer, already validated against pact-reference's 803 specification test cases. What this grammar adds
is the shape half.

**Both paths must agree.** A v1–v4 pact can reach a plan two ways: compiled directly (design 3.5), or
upgraded to shapes (design 2.5) and compiled as shapes. Where both paths exist for the same input they
MUST produce the same verdicts over the same values. They need not produce the same plan — and §6
requires corpus cases that check exactly this, because two compilers agreeing is a claim, and an
unchecked claim about migration is how migrations break.

### 4.5 Which compiler may emit what

An action carries a set of compilers permitted to emit it, and the name says which: the shape compiler
MUST NOT emit an action in the `legacy` namespace, and the v1–v4 compiler MUST NOT emit `match:any-of`
or `expect:absent`. An engine MUST reject a plan in which the shape compiler emitted a `legacy:` action,
as an internal error — this is the mechanism that keeps 2.2's must-ignore guarantee true rather than
merely intended. A guarantee that depends on nobody writing the wrong line is not a guarantee.

### 4.6 Component actions

Everything content- or transport-specific is in a component's namespace and contributed: `json:parse`,
`xml:parse`, `header:parse`, `form:parse`, `multipart:parse`. None of them is core, and the kernel
resolves none of them itself.

This is the largest single change from the forked engine (§9), and it is not a stylistic preference:
it is the architecture rule that the kernel knows nothing about HTTP or JSON, applied to the one place
where the knowledge would otherwise be invisible — inside the interpreter's dispatch table rather than
in an obviously content-shaped module. Whether the JSON component ships in-tree is design 2.6's
question and does not change this: in-tree components implement exactly the same interface.

### 4.7 Reading grammar v0

Grammar v0 named actions three ways: a component's carried its namespace, most core ones carried a
*family* prefix that was a convention and not a scope, and control and value actions carried nothing.
v1 renames, and does nothing else. **An engine that reads v1 SHOULD also read a v0 document, by this
mapping**, one name to one name:

| v0 | v1 |
|---|---|
| `and`, `or`, `if`, `error`, `apply`, `for-each`, `tee` | `flow:and`, `flow:or`, `flow:if`, `flow:error`, `flow:apply`, `flow:for-each`, `flow:tee` |
| `join`, `join-with`, `length`, `lower-case`, `upper-case`, `to-string` | `value:join`, `value:join-with`, `value:length`, `value:lower-case`, `value:upper-case`, `value:to-string` |
| `expect:only-entries` | `legacy:only-entries` |
| `match:array-contains` | `legacy:array-contains` |
| `match:min-type`, `match:max-type`, `match:min-max-type` | `legacy:min-type`, `legacy:max-type`, `legacy:min-max-type` |
| `match:header-value` | `legacy:header-value` |
| every other `expect:*`, `check:*` and `match:*` action | the same name |
| `<component>:<name>` | the same name |

A document is read under the grammar it declares and no other (§7.1): a v0 fragment that says
`flow:if` is naming a component called `flow`, and a v1 fragment that says `if` is naming nothing. What
an engine *shows* is always v1 — a v0 fragment's nodes are printed by `explain` under their v1 names, so
a plan has one vocabulary whoever wrote its parts. v0 also reserved a `convert` family that never had an
action; v1 does not reserve it.

This is what [ADR 0022](../../decisions/0022-a-fragment-replaces-its-slots-plan-and-declares-a-grammar-the-engine-says-it-reads.md)
designed for, an older fragment read by a newer engine by rule, and it is why a component built against
v0 keeps working without a rebuild.

## 5. Compiling shapes to plans

### 5.1 The obligation

Shape spec §7.4 states the rule and this specification inherits it unchanged: **the plan compiled for
a shape `S` MUST accept exactly `admits(S)`**. Nothing here constrains which actions a compiler uses
to achieve it; §6 is where the correspondence stops being a claim.

Under a variant assignment the shape is narrower (shape spec §7.1, variant spec §5.2), and so is the
plan: a compiler given an assignment pins each dimensional operator to its selected point and compiles
the narrowed shape. A plan is therefore compiled *per variant*, and two variants of one interaction
are two plans.

### 5.2 Operator by operator

| Operator | Compiles to |
|---|---|
| value operators | the matching action of §4.3, applied to the resolved value |
| `object` | `expect:object`, then a `container` per named member, each holding the member's compiled shape; no assertion about unnamed members |
| `array` | `expect:array`, `expect:count`, then a `container` per index |
| `each-like` | `expect:array`, `expect:size` for the cardinality, plus `flow:for-each` over a `splat` of the elements, with the item shape compiled once and applied to `resolve-current` |
| `each-entry` | `expect:object`, then the same over entries, with `entry` values (§2.2) feeding key and value shapes |
| `optional` | `flow:if` on `check:exists`: present branch compiles `of`, absent branch is `ok` |
| `forbidden` | `expect:absent` |
| `nullable` | `flow:if` on `check:null`: null branch is `ok`, otherwise compile `of` |
| `one-of` | nested `flow:if` on `check:equals` of the discriminator, one branch per alternative, `flow:error` on no match naming the value read |
| `any-of` | `match:any-of` with the options as `value` children |
| `contains` | `expect:array`, then `match:contains` — opaque, per shape spec §4.1 |

Every structural operator leads with the kind it admits (shape spec §4.3: "`object` admits objects",
"`each-like` admits arrays whose length is in `[min, max]`"). Emitting it is not belt-and-braces: the
operators below it address *members* and *elements*, and a value of the wrong kind has neither, so
without the assertion they have nothing to fail on. §5.1's obligation is the reason, and corpus case
`shapes/structural-kind-guard` is where it stops being a claim.

The must-ignore default (shape spec §4.3) is visible here as an *absence*: `object` compiles no
assertion about members nobody named. Extra fields are admitted because nothing in the plan looks at
them, which is the most inspectable form the guarantee could take. The kind assertion does not touch
that — it says what the value *is*, never what members it may carry.

### 5.3 What the shape compiler may not emit

The shape compiler MUST NOT emit `legacy:only-entries` or any other action that fails on an unnamed
member, and no action in the `legacy` namespace at all. §4.5 makes this checkable; ADR 0007 commitment 4
is why it is a rule.

Nor may a shape *ask* for a core action by name. A component operator compiles to the action of the
same name (shape spec §3.5), so an operator spelled in a reserved namespace would be a core action
reached from outside the compiler's own table, with whatever arguments the shape happened to carry.
The shape language refuses such an operator before anything is compiled (shape spec §3.7), which is
what makes "the shape compiler emits a core action only where §5.2 says so" true of every shape and not
only of well-meant ones. Corpus case `shapes/reserved-namespace-operator` is the evidence.

## 6. Golden corpora

### 6.1 A case

Schema: [`schemas/v1/corpus-case.schema.json`](schemas/v1/corpus-case.schema.json). A case is a
directory holding four files:

| File | Holds | On a diff |
|---|---|---|
| `case.json` | the input, the captured values, and the expected verdict with every expected mismatch | its `result` is an **assertion** — a diff is a bug |
| `plan.txt` | the compiled plan, pretty form (§3.1) | **snapshot** — regenerate and review |
| `executed.txt` | the executed plan, executed form (§3.2) | **snapshot** — regenerate and review |
| `README.md` | what the case is evidence for, when the one-line `description` is not enough | — |

The split is deliberate: the two halves that are text snapshots are *files a human diffs*, not strings
inside a JSON document where every newline is an escape. A corpus nobody can read in a pull request is
a corpus nobody reviews.

**A case may say that its input is refused.** Some rules are about what never becomes a plan — an
operator nobody can contribute, a shape that does not parse. Such a case has `status: invalid` and a
`problems` list in place of `mismatches`: each problem has the `pointer` the engine reports (an RFC 6901
pointer into the input) and words its message must contain. The engine must refuse the input with
exactly those problems, in order. There is no plan, so the case has no `plan.txt` or `executed.txt`,
and its `values` are empty.

### 6.2 Snapshots and assertions are different failures

Both halves fail a corpus run, and a reader must be able to tell which kind of red they are looking
at, because the remedies are opposite:

- **A `result` diff is a behaviour change.** The engine now accepts or rejects something it did not.
  Either the change is a bug, or it is a deliberate change to matching behaviour — in which case
  `corpora/` changes in the same commit, which is the rule `AGENTS.md` already states.
- **A `plan.txt` diff with `result` unchanged is a plan change.** A newer compiler may legitimately
  emit a different plan — a better optimisation, a clearer structure — and §7 permits it. The remedy
  is to regenerate the snapshot and let the diff land in the pull request.

The second is not a weaker check. The plan text is what a user sees when they run `explain`, so a
restructured plan is a user-visible change even when every verdict is identical, and a corpus that
recorded only verdicts would let it through silently.

### 6.3 What the corpus can and cannot prove

The results over captured values are **evidence** that a plan change was safe, not proof. Proving two
plans equivalent — that they accept the same set of values — is the subsumption problem again over a
richer language than shapes, and it is out of reach (shape spec §8 declares the same limit for
shapes). A corpus case says "these plans agree on these values". Choosing values that make that
worth something is the corpus author's job, and task 3.7 is where the coverage obligation lands: every
shape operator, representative v1–v4 constructs, and both compilation paths for the same input (§4.4).

## 7. Versioning and stability

### 7.1 Plans are renderings; the grammar is the record

The question the RFC leaves open is what stability a plan owes. The answer follows from asking what
records one:

| Where a plan appears | Lifetime | Read by another engine version? |
|---|---|---|
| `verification/explain` result | request/response | no |
| `verification/executed-plan` event | event stream | no |
| CI logs and failure artifacts | archived | in text, by a human |
| AI diagnosis traces (design 2.10) | within a run | no |
| golden corpora | checked into this repo | no — regenerated with the engine |
| **component-contributed fragments** (task 8.4) | **shipped separately from the engine** | **yes** |

Only the last crosses a version boundary, and it is not a recorded plan — it is an *authored*
plan-shaped document. **No pact file contains a plan**, and none ever should: it would freeze a
compiler's output into a contract, so improving the compiler would break pacts nobody can re-run, and
it would put a second source of truth beside the shape that the verifier actually matches against
(shape spec §7.3).

So:

- **The grammar is a versioned compatibility surface**, evolving additively under the protocol's rules
  (protocol spec §11.2) and enforced by the same CI checker. A version is `v<major>` or
  `v<major>.<minor>` (`v1` is `v1.0`); an additive change is a new minor, anything else a new major, and
  a reader reads its own major at any minor no newer than its own (component-interfaces spec §12.3, ADR
  0022). Node kinds, the value model, action naming and result kinds are what a fragment author writes
  against. A reader MAY read another major where a rule says how, and never by guessing: v1's rule for
  v0 is §4.7, and an engine lists every grammar it reads so a component can tell.
- **An action's semantics are frozen once published.** A component emitting `match:regex` breaks if an
  engine redefines it. New behaviour is a new action name — ADR 0007's commitment, for the same
  reason.
- **The plan produced for a given input is not stable across engine versions, and nothing may depend
  on it being so.** A newer engine may compile a better plan for the same shape. Corpora record plans
  knowing they get regenerated (§6.2); nothing else records them at all.

### 7.2 The contrast with variant sampling

Design 2.3 made variant selection deterministic *and* froze it under a name (`janus-ipog-v1`), because
the selected sample is recorded in a pact file. Plan compilation gets the first half and not the
second. The discriminator is exactly whether the artifact outlives the engine that produced it, and
stating it that way is what makes the two policies consistent rather than arbitrary.

## 8. Errors

Codes are the Engine Protocol's (protocol spec §10.2). This specification constrains three:

| Code | Category | When |
|---|---|---|
| `interaction-invalid` | `document` | a shape that cannot be compiled — including one whose operator is in a reserved namespace (§5.3) — with `problems[].path` at the node |
| `component-unavailable` / `component-failed` | `component` | a component's action whose component is missing, or which the component rejected (shape spec §3.7's table, applied to actions) |
| `internal` | `internal` | an action in a reserved namespace that the engine does not implement, or a compiler emitting an action §4.5 forbids it — both are engine bugs, not user errors |

A node with fewer children than its action takes (§4.2) is none of these codes when an interpreter
meets it: it is an `error` result on that node, like a mismatch, and the run carries on. Refusing it
earlier is the fragment check's job (`fragment-invalid`, component-interfaces spec §6.3).

A mismatch is **not** an error in this sense. It is an `error` *result* on a node (§2.3), carried in
the executed plan and reported through the plan's result form — which is what design 2.1 means by
`mismatches` being "documents in the plan grammar's result form".

## 9. What this takes from the v2 engine

ADR 0004 forked `pact_matching/src/engine` and said the kernel "inherits code written for matching
rules and carries that shape until 2.2/2.4 restructure it". This is that restructuring, stated at the
granularity of what survives.

**Kept, substantially unchanged:**

- The **node kinds** and the tree model. `container`, `action`, `value`, `resolve`,
  `resolve-current`, `pipeline`, `splat`, `annotation` are a generic expression tree with nothing
  matching-rule-specific in them.
- The **result model** — `ok` / `value` / `error`, with conjunction at containers.
- The **text forms and their sigils** (§3), which are already tested and already what `explain`
  prints.
- The **generic action families**: control, value, structural assertions, checks — 17 of the forked
  engine's 32 dispatched actions.
- The **legacy matching family**, which is the half already validated against 803 specification test
  cases (§4.4).

**Changed:**

- The **value model** collapses from fourteen kinds to eight (§2.2). `json`, `xml`, multi-string maps
  and string lists go; a decoded body is a document and a header set is an object.
- **Fourteen content- and transport-specific actions** become namespaced component contributions
  (§4.6): `json:*`, `xml:*`, `header:*`, `form:parse`, `multipart:parse`.
- The **matching family gains a shape half** (§4.3), derived from the operator set rather than from
  v1–v4 matcher names, and `legacy:only-entries` becomes legacy-only and unreachable from shapes
  (§4.5).

**Dropped:**

- `EMPTY` as a node kind — a Rust `Default`, not a grammar concept (§2.1).
- `NAMESPACED` as a value kind — the forked interpreter carries it with a `todo!()` at the point where
  it would be resolved. An unfinished concept is not an inheritance.

None of this trips ADR 0004's tripwire, which fires if "compiling shapes to the forked node grammar
proves fundamentally awkward — grammar changes so deep the fork stops paying for itself". The grammar
spine and the interpreter both survive; what changes is a value type that predates ADR 0006's document
model and an action set that was always going to need a second half. What *would* trip it is
discovering in task 3.3 that the spine cannot carry shape compilation at all.
