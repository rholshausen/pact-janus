---
engine: major
---

# A content component says what a value operator means for its type; plan fragments are withdrawn

Staged plan S5.4, ADR 0100 (gate F). A content component's handshake may give a `content-types` entry
`substitutions`: a value operator's name to one of the component's contributed actions, such as
`"integer": "csv:integer"`. In a slot whose declared type the component decodes, the engine compiles the
shape itself, every structure and every variant's pinning included, and emits the component's action
where the operator's core action would be, with the same children, run through `matcher/apply`. A slot
with substitutions begins with an annotation naming them, which `explain` prints. A variant is now
checked at its point whatever decodes its slot: a fragment used to check every variant against the
unpinned shape (Phase 9 finding 21).

- Only value operators can be substituted. The loader refuses a substitution for a structure, a
  modifier or `content-type`, of an action the component does not contribute, or by a component that
  does not implement `matcher`, as `component-invalid`. An operator name it does not know is ignored.
- **Breaking for a component that contributed a plan fragment**: `content/compile`'s `fragment` and
  `grammar-version` are ignored (and logged), so its slots compile with the core actions. Declare
  `substitutions` instead. `content/compile` now answers success, or fails as `compile-failed`.
- `component/hello` no longer sends `plan-grammar-versions`, and `matcher/compile` is withdrawn. The
  schema members stay, deprecated. The `grammar-skew` and `fragment-invalid` refusals are gone.
- An action is applied with the value under test and its further children's values as
  `config.arguments`, now normatively: a substituted action gets its core action's arguments, and a
  component operator's action gets its members as one argument.
- `pact_janus_kernel::plan::fragment` is gone: the grammar-version rules are `plan::versions`, and
  `plan::compile_substituted` compiles with a `SlotSubstitutions`. `CompileResult` has no fields, and
  `Resolved` has `substitutions`.
