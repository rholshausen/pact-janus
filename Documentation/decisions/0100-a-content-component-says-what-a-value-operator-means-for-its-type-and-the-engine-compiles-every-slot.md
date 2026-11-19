# 0100 — A content component says what a value operator means for its type, and the engine compiles every slot

- **Status**: accepted; supersedes [ADR 0022](0022-a-fragment-replaces-its-slots-plan-and-declares-a-grammar-the-engine-says-it-reads.md)
  decisions 1, 3, 4 and 6
- **Date**: 2026-11-19
- **Plan tasks**: S5.4 (gate F)
- **Evidence**: the [plan-fragment stress test](../prototype/plan-fragment-stress-test.md) and Phase 9
  findings [21, 22 and 24](../prototype/phase-9-findings.md#21-contentcompile-is-not-told-the-variant-so-a-fragment-cannot-pin-one);
  `engine/kernel/tests/substitutions.rs`, `engine/component-host/tests/csv_component.rs` (§ substitutions),
  `third-party/janus-csv`; [ADR 0022](0022-a-fragment-replaces-its-slots-plan-and-declares-a-grammar-the-engine-says-it-reads.md),
  [ADR 0026](0026-every-plan-action-is-namespaced-and-core-namespaces-are-reserved.md),
  [ADR 0036](0036-read-text-bodies-through-a-text-component-and-unclaimed-bodies-as-bytes.md),
  [ADR 0049](0049-match-a-content-type-against-the-arrivals-own-octets-and-labels.md)

## Context

A content type can change what a shape operator means. CSV has no types, so every field decodes as a
string, and `integer` of a CSV column means text that spells an integer. The engine's generic plan checks
`match:integer`, which no string passes. So the prototype let a content component answer
`content/compile` with a **plan fragment**, a plan node in the plan grammar that replaced the engine's
plan for the whole slot (ADR 0022). The CSV component's fragment said `csv:integer` where the engine
said `match:integer`, and a CSV consumer could write `integer` and mean it.

Task 8.4's stress test found what that costs, and the staged plan made it gate F: decide before any
third party depends on either model.

- **Finding 21.** `content/compile` is given the slot's shape and not the variant. The engine compiles a
  plan per variant and pins each dimensional operator to the variant's point: an `each-like` of 1–3's
  maximal variant is checked with `expect:count 3`. The fragment checked `expect:size 1..3` for every
  variant, so against a provider with one row, the three-row variant passed under the fragment where the
  engine's own plan failed it. The variant proved nothing and the run said verified. This is gate F's
  trigger condition, demonstrated in `a_fragment_compiled_without_the_variant_widens_a_pinned_variant`.
- **A fragment makes its author a shape compiler.** It replaces the slot's plan, so it must accept
  exactly what the shape admits, for every operator, structure and pinning the engine's compiler
  handles. The CSV component covered one structure and eight operators in 150 lines that mirrored the
  engine's compiler, and declined everything else.
- **Finding 22.** A fragment resolves the decoded document, never the octets, so the specification's
  picture of a fragment that decodes (`csv:parse` as a plan step) has never been true.
- **Finding 24.** An action node has children and no `config`, and `matcher/apply` takes `config`. ADR
  0022 decision 6 mapped further children to `config.arguments`, provisionally.

ADR 0022 named the likely answer as its tripwire: operator-level substitution, where the engine compiles
the slot and a component says only what one value operator means.

## Decision

1. **A content component says which of its actions stands for a value operator under its content type,
   and the engine compiles the slot.** A `content-types` entry in the handshake may carry
   `substitutions`, an object from a shape-language operator name to one of the component's contributed
   actions:

   ```json
   { "media-type": "text/csv",
     "substitutions": { "integer": "csv:integer", "number": "csv:number", "decimal": "csv:number",
                        "boolean": "csv:boolean", "equality": "csv:equality" } }
   ```

   In a slot whose declared type the component decodes, the engine compiles the shape exactly as it
   compiles any other, with every structure and every variant's pinning. Where it would emit the action a
   substituted operator compiles to, it emits the component's action instead, with the same children.
   Plan grammar §5.1's obligation stays the engine's. A component never sees structure, so it cannot get
   pinning wrong, and finding 21 is gone: a variant that pins a cardinality is checked at that
   cardinality whatever decodes the slot. This supersedes ADR 0022 decision 1.

2. **Only value operators can be substituted.** They are `any`, `equality`, `type`, `string`, `number`,
   `integer`, `decimal`, `boolean`, `null`, `regex`, `datetime`, `date`, `time`, `include`, `semver`,
   `not-empty` and `any-of`: the operators that check one value and contain no other shape. A structure
   (`object`, `array`, `each-like`, `each-entry`, `contains`), a modifier (`optional`, `nullable`,
   `forbidden`, `one-of`) and `content-type` cannot be substituted. A structure is how the engine
   pins variants. `content-type` reads the arrival's octets and labels, not the decoded document (ADR
   0049). A narrowed form takes the substitution of the operator whose action it is: a variant that pins
   an `any-of` to one literal is checked as `equality`, and one that pins a `nullable` to null as `null`.
   A component operator is its own component's already and is never substituted.

3. **Substitutions are checked when the component loads.** Each one must name an action the component
   contributes (so in its namespace, §2.4), and the component must implement `matcher`, whose `apply`
   runs it. A substitution of a structure, a modifier or `content-type` is refused too. Any of these
   fails the load as `component-invalid`, before any interaction is looked at. An operator name the
   engine does not know is ignored, so a component can name an operator a newer shape language adds
   without failing on an older engine. There is nothing left to check when an interaction arrives, so
   `grammar-skew` and `fragment-invalid` go. `content/compile` is still asked for each declared slot
   when the interaction arrives, of the component that decodes it, declared or in-tree. Its only answer
   is now success or failure: it lets a component refuse a shape it cannot carry, as the built-in `text`
   component refuses one that admits no string (ADR 0036), as `compile-failed`. This supersedes ADR
   0022 decision 4.

4. **An action is applied with the value under test and its arguments.** The first child of an action
   node is the value under test, a `MatchValue` in `matcher/apply`'s `values`, with the path it was
   resolved from. Every further child's value is in `config.arguments`, in order. A substituted action
   gets the children the core action would have had: `regex` its pattern, `equality` and `type` the
   example, a temporal operator its format when it has one, `any-of` each literal, and the rest none. A
   component operator's action gets one argument, the operator's members as an object. This is now
   normative, where ADR 0022 decision 6 called it provisional. The plan grammar gains no `config` member.
   No case has needed configuration that is not an argument, and an action's configuration in its
   children is what `explain` prints.

5. **Components contribute no plan.** `content/compile` no longer returns a fragment, and
   `matcher/compile` is withdrawn: nothing ever called it, and a component operator already compiles to
   its own action (decision 4). An engine ignores a `fragment` a component still returns. The schema
   members stay, deprecated, since a v1 schema never loses one. With no plan crossing from a component,
   `component/hello` stops sending `plan-grammar-versions`, which existed so a component could choose a
   grammar to write a fragment in. This supersedes ADR 0022 decision 3. Decision 2, how a plan document
   declares and is read under a grammar version, still governs every plan document the engine reads,
   and decision 5, the reserved namespaces, still governs every action a component contributes.

6. **Decoding is not a plan step (finding 22).** The component that handles a slot's declared type
   decodes it before the plan runs, as the engine has always done. The plan grammar stops promising
   `json:parse` and its kin as actions. A slot with substitutions begins with an annotation naming its
   content type, the component and the operators it stands for. So `explain` says why `csv:integer` is
   where `match:integer` would be, and the plan still shows every check the engine makes. Where a decode
   failure and a degradation are shown, a node of this slot or the run's result, is S5.3's to decide
   (finding 14).

7. **Nothing a contract records changes.** A contract records shapes, never plans (plan grammar §7.1).
   `integer` over `text/csv` is recorded as `integer`, and a substitution decides how it is checked
   against a value, not what the shape says. The subsumption walk compares the two shapes' operators as
   it does today.

## Alternatives considered

- **Tell `compile` the variant** (finding 21's option A). Rejected: every component that contributes a
  fragment would then implement pinning, the engine compiler's subtlest part, and a component that gets it
  wrong reports a variant verified that proved nothing. It would also keep a component a shape compiler.
- **Refuse a fragment for a slot with a dimensional operator** (option C). Rejected: that is nearly every
  slot worth a fragment. A CSV body is an `each-like`.
- **Keep fragments beside substitution, for whatever a substitution cannot say.** Rejected: no case
  needs one, and keeping fragments keeps finding 21 and the grammar-skew machinery that only they need.
  A future case can propose them again with its evidence.
- **Answer the substitutions from `content/compile`, per slot.** Rejected: the handshake is the source of
  truth about what a component contributes (component-interfaces spec §3.2). A static table is checked
  once, when the component loads, and costs no call per slot. A component that must decline a shape
  still fails `compile`.
- **Key substitutions by core action name (`match:integer`).** Rejected: that ties the component
  interface to the plan grammar's version. A shape operator's name is frozen by the shape language, and
  a component author already writes in its vocabulary.
- **A `config` member on action nodes** (finding 24's option B). Rejected: it is a grammar change, and no
  action needs configuration that is not an argument.
- **Decoding as a plan step** (finding 22's option A). Rejected for now: no component writes a plan any
  more, so nothing needs to see octets in one, and S5.3 can show a decode failure without a new core
  action.

## Consequences

- A content component's obligation shrinks from "compile the slot" to "say what one operator means". The
  CSV component's 150-line fragment compiler is a five-entry table, and it covers every structure a
  CSV body can be shaped with, not one.
- A variant is checked at its point whatever decodes its slot. The test that demonstrated finding 21
  now shows the three-row variant failing against a provider with one row.
- A component that contributed fragments loses them silently: its slots compile generically and fail
  where its type changes an operator's meaning. Only `third-party/janus-csv` did, before S5.1d invites
  anyone to depend on the interface.
- The plan grammar no longer crosses a version boundary through components. It still versions the plan
  documents the engine reads, which is a smaller surface to keep stable.
- Tripwire: if a content type needs a check that no single value operator stands for (a whole-slot
  invariant, say a checksum over the octets), propose the smallest contribution that carries it, not
  fragments again.
