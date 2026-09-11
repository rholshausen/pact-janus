# Plan grammar and core action set specification (v0, final)

Plan task: **2.4**. Status: **final**.

A shape says what a value must be; a **plan** is what the engine actually executes to find out, and
what `explain` prints when someone asks why a match succeeded or failed. This document specifies the
node grammar, the value model, the core action set, the two text forms, how a shape compiles to a
plan, the golden-corpus format, and the versioning policy the RFC lists as an implementation unknown.

It is written against a working prototype rather than from nothing: [ADR
0004](../../decisions/0004-fork-v2-engine-as-kernel.md) forked the v2 matching engine from
pact-reference as the kernel's starting point, and §9 records exactly what this specification takes
from it and what it does not. The versioning decision is [ADR
0010](../../decisions/0010-plans-are-renderings-the-grammar-is-the-record.md).

The schemas under [`schemas/v0/`](schemas/v0/) are the **specified surface**, on the same terms as the
other Phase 2 designs': this prose defines their semantics, the schemas define their shapes, and they
follow the Engine Protocol's open-world authoring rules
([protocol spec §2.2](../engine-protocol/spec.md#22-open-world-authoring-rules)).

**Why v0 and not v1.** Every other Phase 2 schema set is `v1`, because the documents they govern are
authored by users and recorded in artifacts that outlive the engine. A plan is neither (§7). The
grammar is a real compatibility surface — plugins author fragments against it (plan task 8.4) — but it
has had no external consumer yet, and calling it v1 before one has ever tried to extend it would be
claiming a stability nobody has tested. Plan task 8.4 is the test; v1 is what it earns.

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
**result** (§2.3). v0 defines eight kinds:

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
current: the element a `for-each` is on, or the running value of a `pipeline`. Those are the only two
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
owns them, except where an action is **lazy** — `if` evaluates its condition before its branches, `or`
stops at the first success — and every lazy action MUST declare it (§4.2), because a reader of an
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
        %join (
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
schema: the sigils and layout above are fixed for v0, and a renderer MUST produce exactly this form.

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
        %join (
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

## 4. The action set

### 4.1 Naming and namespacing

An action name is kebab-case, optionally `:`-separated into a family and a name (`expect:empty`,
`match:regex`). The rules mirror the shape language's (shape spec §3.5), for the same reasons:

- **An unnamespaced action name is reserved for this specification, forever.** A component MUST NOT
  define one, and an engine MUST NOT resolve an unnamespaced name to a component.
- A component contributes actions under its own identifier: `protobuf:decode`, `http:status-class`.
  The namespace is the component's; the requirement that pulls it in travels in the interaction spec,
  not in the plan node.
- An **unknown action is a named failure**, never an ignored node (shape spec §3.7's rule, applied
  here): silently skipping a node drops a constraint while still reporting success.

The families a `:` prefix names are conventions, not scopes: `match:*` asserts a value is in a set,
`expect:*` asserts a structural property, `check:*` yields a boolean rather than failing, `convert:*`
transforms. `json:parse` is a *namespaced* action belonging to the JSON content component; `expect:*`
is a *core* family. The difference is whether the first segment names a component, and §4.6 is
explicit about which is which.

### 4.2 Core actions (v0)

| Family | Actions | Lazy |
|---|---|---|
| control | `and`, `or`, `if`, `error`, `apply`, `for-each`, `tee` | `or`, `if` |
| value | `join`, `join-with`, `length`, `lower-case`, `upper-case`, `to-string` | |
| structural assertions | `expect:empty`, `expect:not-empty`, `expect:count`, `expect:size`, `expect:entries`, `expect:only-entries`, `expect:absent` | |
| checks | `check:exists`, `check:equals`, `check:null` | |
| matching | `match:*`, one per shape operator (§4.3) | |

The `check:` family is what the `expect:` and `match:` families are not: a check **yields a boolean**,
an assertion **fails**. Both are needed and they are not each other's negation in any useful sense — a
presence *branch* asks a question and carries on either way, while a `forbidden` member makes a claim
that can be wrong. Every conditional in §5.2 takes a `check:` action as its condition, and no `match:`
or `expect:` action ever appears in one; an assertion in a condition position would fail the
interaction while deciding a branch, which is not a thing a reader could be expected to predict.

`expect:count` asserts an exact size and `expect:size` a range, with `NULL` for an unbounded end.

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

The v1–v4 compiler (design 3.5) emits from the same set. Most of it overlaps: shape spec §4.2 fixed
`regex`'s dialect and anchoring to match v1–v4 exactly, so `match:regex` serves both compilers with
one semantics. Where they differ is small and worth naming:

| Action | Emitted by | Note |
|---|---|---|
| `expect:only-entries` | legacy only | v1–v4's request bodies are closed by default; response bodies (like every shape) are must-ignore — design 3.5 verified this asymmetry against the 803 specification test cases rather than assuming a single default, and only the request side ever emits this action. No shape compiles to it either way — ADR 0007 commitment 4 refuses closed objects, and §5.3 makes that a checkable rule rather than a convention. |
| `match:array-contains` | legacy only | the old `arrayContains`; shapes reach it through the opaque `contains` operator |
| `match:min-type`, `match:max-type`, `match:min-max-type` | legacy only | v1–v4's `MinType`/`MaxType`/`MinMaxType`: a type check plus a collection-size bound, enforced only when the resolved value is actually a collection (design 3.5's write-up of exactly why). The shape language has no single operator combining the two — `type` and `each-like`'s cardinality are separate operators there. |
| `match:header-value` | legacy only | v1–v4's default (no matching rule) header comparison — not plain string equality: a MIME-shaped value compares type and parameters as a set (order- and case-insensitive on the parameter values, extra actual parameters allowed), a comma-separated one tolerates whitespace around the commas, anything else is exact and case-sensitive. Shapes match a header's value with the ordinary value operators (§4.3) instead. |
| `match:any-of`, `expect:absent` | shapes only | v1–v4 has no enumeration or absence assertion |

This is why the forked action set is worth keeping rather than replacing: it is the legacy half of the
answer, already validated against pact-reference's 803 specification test cases. What v0 adds is the
shape half.

**Both paths must agree.** A v1–v4 pact can reach a plan two ways: compiled directly (design 3.5), or
upgraded to shapes (design 2.5) and compiled as shapes. Where both paths exist for the same input they
MUST produce the same verdicts over the same values. They need not produce the same plan — and §6
requires corpus cases that check exactly this, because two compilers agreeing is a claim, and an
unchecked claim about migration is how migrations break.

### 4.5 Which compiler may emit what

An action carries a set of compilers permitted to emit it. An engine MUST reject a plan in which the
shape compiler emitted a legacy-only action, as an internal error — this is the mechanism that keeps
2.2's must-ignore guarantee true rather than merely intended. A guarantee that depends on nobody
writing the wrong line is not a guarantee.

### 4.6 Component actions

Everything content- or transport-specific is namespaced and contributed: `json:parse`, `xml:parse`,
`header:parse`, `form:parse`, `multipart:parse`. None of them is core, and the kernel resolves none of
them itself.

This is the largest single change from the forked engine (§9), and it is not a stylistic preference:
it is the architecture rule that the kernel knows nothing about HTTP or JSON, applied to the one place
where the knowledge would otherwise be invisible — inside the interpreter's dispatch table rather than
in an obviously content-shaped module. Whether the JSON component ships in-tree is design 2.6's
question and does not change this: in-tree components implement exactly the same interface.

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
| `object` | a `container` per named member, each holding the member's compiled shape; no assertion about unnamed members |
| `array` | a `container` per index, plus `expect:count` |
| `each-like` | `expect:size` for the cardinality, plus `for-each` over a `splat` of the elements, with the item shape compiled once and applied to `resolve-current` |
| `each-entry` | the same over entries, with `entry` values (§2.2) feeding key and value shapes |
| `optional` | `if` on `check:exists`: present branch compiles `of`, absent branch is `ok` |
| `forbidden` | `expect:absent` |
| `nullable` | `if` on `check:null`: null branch is `ok`, otherwise compile `of` |
| `one-of` | nested `if` on `check:equals` of the discriminator, one branch per alternative, `error` on no match naming the value read |
| `any-of` | `match:any-of` with the options as `value` children |
| `contains` | `match:contains` — opaque, per shape spec §4.1 |

The must-ignore default (shape spec §4.3) is visible here as an *absence*: `object` compiles no
assertion about members nobody named. Extra fields are admitted because nothing in the plan looks at
them, which is the most inspectable form the guarantee could take.

### 5.3 What the shape compiler may not emit

The shape compiler MUST NOT emit `expect:only-entries` or any other action that fails on an unnamed
member. §4.5 makes this checkable; ADR 0007 commitment 4 is why it is a rule.

## 6. Golden corpora

### 6.1 A case

Schema: [`schemas/v0/corpus-case.schema.json`](schemas/v0/corpus-case.schema.json). A case is a
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

### 6.2 Snapshots and assertions are different failures

Both halves fail a corpus run, and a reader must be able to tell which kind of red they are looking
at, because the remedies are opposite:

- **A `result` diff is a behaviour change.** The engine now accepts or rejects something it did not.
  Either the change is a bug, or it is a deliberate change to matching behaviour — in which case
  `corpora/` changes in the same commit, which is the rule `CLAUDE.md` already states.
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
  (protocol spec §11.2) and enforced by the same CI checker. Node kinds, the value model, action
  naming and result kinds are what a fragment author writes against.
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
| `interaction-invalid` | `document` | a shape that cannot be compiled, with `problems[].path` at the node |
| `component-unavailable` / `component-failed` | `component` | a namespaced action whose component is missing, or which the component rejected (shape spec §3.7's table, applied to actions) |
| `internal` | `internal` | an unnamespaced action the engine does not implement, or a compiler emitting an action §4.5 forbids it — both are engine bugs, not user errors |

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
  v1–v4 matcher names, and `expect:only-entries` becomes legacy-only and unreachable from shapes
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
