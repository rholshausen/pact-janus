# Golden corpora

Executable-specification artifacts: for each case, an input (interaction spec or v1–v4 pact), the
expected compiled plan, and the expected result of executing that plan against captured values. The
corpus format is defined by the plan-grammar design (plan task 2.4, `corpus-case.schema.json`,
spec §6); cases land from Phase 3 (task 3.7).

A case is a directory holding `case.json` (the input, the captured `values`, and the expected
`result` — an assertion), `plan.txt` and `executed.txt` (the pretty and executed text forms, spec
§3 — snapshots, regenerated and reviewed rather than hand-edited). Cases are grouped for
readability:

- `shapes/` — one case per core shape operator (design 2.2), plus `order-payload/`: the RFC's full
  order response, every failure reported in one run, the untaken `:card` alternative never executed.
- `legacy/` — representative v2/v3/v4 matching-rule constructs compiled directly (design 3.5),
  including `v3-cascading-type/` (the precedence algorithm that is design 3.5's own reason to
  exist) and `v4-closed-request-body/` (the request/response closed-object asymmetry). `two-path-
  agreement/` compiles the same interaction both ways — a v1–v4 pact directly, and the equivalent
  hand-authored shape — and asserts they reach the same verdict (plan-grammar spec §4.4).

## Running the corpus

`cargo test -p pact_janus_corpus` is what CI runs: every case must match its `result` exactly, and
its `plan.txt`/`executed.txt` must match the checked-in snapshot byte for byte. `cargo run -p
pact_janus_corpus` runs the same checks with human-readable `✓`/`✗` output.

A `result` diff is a behaviour change — a bug, or a deliberate change that means editing `case.json`
in the same commit; it is never auto-fixed. A snapshot diff is regenerated with:

```sh
cargo run -p pact_janus_corpus -- accept
```

then reviewed in the diff, per `Documentation/specs/plan-grammar/examples/corpus-case.md` §4–5's
two kinds of red.

**Corpora are load-bearing**: any change to matching behaviour must change this directory in the same
commit, and CI executes plans against these cases on every change.
