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
  order response, every failure reported in one run, the untaken `:card` alternative never executed;
  and `structural-kind-guard/`, a value of the wrong kind given to operators whose members and
  elements would otherwise have nothing to fail on (phase-9 finding 8); and
  `cardinality-pinned-region/`, a request list pinned to `min+1` admitting any length above the
  minimum (ADR 0024, phase-9 finding 25) — the corpus's one case compiled under a variant; and
  `reserved-namespace-operator/`, a shape that asks for core plan actions by name and is refused
  (staged plan S0.13) — the corpus's one case with no plan; and `member-order/`, members declared out
  of name order and compiled by name, with an example and a body that hold the same members in other
  orders compared as equal (ADR 0037); and `member-name-quoted/`, members whose names a path segment
  cannot hold (a dot, a brace, a quote, no name at all), each resolved by a quoted segment (ADR 0047). `number-kinds-as-written/` and its `-mismatch` twin are `integer` and `decimal` reading a
  number's kind from how it is written, `100.0`, `1e2` and `-0` being decimals, while `equality` holds
  `100` and `100.0` to be one value (ADR 0072). `xml-text-as-written/` and its `-mismatch` twin are a contract's XML
  text compared as written: whitespace around it, and a CDATA section's contents, match only text
  written the same way, while a member shaped on `#trimmed` matches whatever whitespace is around it
  (ADR 0075).
- `legacy/` — representative v2/v3/v4 matching-rule constructs compiled directly (design 3.5),
  including `v3-cascading-type/` (the precedence algorithm that is design 3.5's own reason to
  exist) and `v4-closed-request-body/` (the request/response closed-object asymmetry). `two-path-
  agreement/` compiles the same interaction both ways — a v1–v4 pact directly, and the equivalent
  hand-authored shape — and asserts they reach the same verdict (plan-grammar spec §4.4). Every
  `upgrade-*/` case does the same with the contract `janus upgrade` converts the pact into, and a test
  in `tools/corpus` holds its `also-compiled-from` to what the upgrade writes. `upgrade-text-body/`,
  `upgrade-binary-body/`, `upgrade-form-body/`, `upgrade-xml-body/`, `upgrade-multipart-body/` and their
  `-mismatch` twins are the body kinds that are not JSON, each in its component's document (ADR 0047).
  `upgrade-generated-request-slot/` and `upgrade-generated-response-slot/` carry generators on their
  shapes, and admit a generated value where the example was (ADR 0048). `upgrade-number-kinds/` and its `-mismatch` twin are a v4 `decimal` and `integer` rule, which
  read how a number is written in the pact and in its contract alike (ADR 0072). `upgrade-message/` and its
  `-mismatch` twin are a v3 message and the emissive interaction it converts into, its metadata
  member by member and its contents a body (ADR 0067). `upgrade-array-any-length/` is a bare `type` rule on an
  array, which names no length: v1–v4 admit `[]`, and so does the `each-like` with `min: 0` it
  upgrades into (ADR 0079).
  `v1-text-body/`, `v1-binary-body/`, their `-mismatch` twins and `v3-text-body-regex/` are bodies
  that are not JSON, each compiled as one value at `$` (ADR 0036); a binary case tags its body's
  value as base64 in `encoded` (plan-grammar spec §6.1). `v1-form-body/` and `v3-form-body-mismatch/`
  are form bodies, compiled as a query string (ADR 0038). `v3-xml-body/` and `v3-xml-body-mismatch/` are
  XML bodies, walked element by element over the XML component's document, their text compared as
  `#trimmed`, today's reading, so `v3-xml-body/`'s item with whitespace around it matches (ADR 0075),
  and `v3-type-rule-on-text/`
  a type rule read against text, as today's verifier reads it (ADR 0039); `v3-xml-min-children-mismatch/`
  a minimum on an element's children, a mismatch located at the element though the action that counts
  them is not a resolve (staged plan S1.30). `v3-multipart-body/` and
  `v3-multipart-body-mismatch/` are multipart bodies, compared part by part over the multipart
  component's document, each part by its own type (ADR 0040). `v3-values/`, `v4-each-key/`,
  `v4-each-value/`, `v4-array-contains/` and their `-mismatch` twins are the rules about a
  collection's entries, compiled as today's verifier applies them (ADR 0042). `v3-message/`,
  `v3-message-mismatch/`, `v4-message-content-type-mismatch/` and `v4-synchronous-message/` are
  messages, compared as today's verifier compares one: metadata key by key, the contents' content type,
  then the contents (ADR 0046). A message's plan is rooted at the `message` part a `produce-message`
  hook produces, and its case names the interaction by its position in the pact, as every case does.
  `v4-ignore-order-mismatch/` is pact-jvm's `ignore-order`, a rule `pact_models` reads as a plugin's,
  which fails as today's verifier fails it; its upgraded contract names it `pact:ignore-order`, which
  fails by name, so the two reach one verdict with different mismatches, and it is not an `upgrade-`
  case (ADR 0073).
  `v3-or-first-alternative-fails/` is an `OR` a later alternative passes, and `v3-xml-not-empty-text/`
  an element's `notEmpty` met by its text, an `OR` too: neither is failed by an alternative it did not
  need, and `v3-or-every-alternative-fails-mismatch/` reports each alternative's mismatch (staged plan
  S2.18).
  `v4-status-class/` and its `-mismatch` twin are a v4 `statusCode` rule, compiled from the pact and
  from its upgraded contract to the built-in HTTP component's `http:status-class`, which the corpus
  runner registers as the native engine does (ADR 0057). `shapes/http-media-type/` and its
  `-mismatch` twin are the same component's `http:media-type`, which `janus upgrade` writes for a
  `Content-Type` with no rule (ADR 0071); a pact's own header goes through `legacy:header-value`, which
  reads headers in another form, so they are spec cases.

`contracts/` pins how a contract is written, not how one matches (contract-file spec §2.4, ADR 0063):
each case is a `pact.json` and the `contract.janus.json` that `upgrade/pact` writes for it, compared
byte for byte by `engine/kernel/tests/contract_text.rs`. `numbers/` holds every way a number is
spelled, and `strings/` every way a string is escaped. `redacted/` is a pact whose credential headers
are recorded as placeholders, a regex keeping its pattern and an `equality` becoming a `type`, and
listed under `redacted` (contract-file spec §5.6, ADR 0074). A dependency that moves a spelling fails it;
a deliberate move changes the case in the same commit, and needs a change file, since every contract
holding that spelling gets a new hash at a broker.

`verifier/` is a different kind of corpus, with its own [README](verifier/README.md): pacts and the
providers that answer them, which staged plan S1.1's harness verifies with today's verifier and with
`janus verify`, comparing the verdicts. It holds no `case.json`, so the corpus runner below does not
read it.

## Running the corpus

`cargo test -p pact_janus_corpus` is what CI runs: every case must match its `result` exactly, and
its `plan.txt`/`executed.txt` must match the checked-in snapshot byte for byte. `cargo run -p
pact_janus_corpus` runs the same checks with human-readable `✓`/`✗` output.

A case may instead say that its input is refused: `result` is then `status: invalid` with the
`problems` the engine must report — each the pointer it names and words its message must contain — and
the case has no `plan.txt` or `executed.txt`, because there is no plan (plan-grammar spec §6.1).

A `result` diff is a behaviour change — a bug, or a deliberate change that means editing `case.json`
in the same commit; it is never auto-fixed. A snapshot diff is regenerated with:

```sh
cargo run -p pact_janus_corpus -- accept
```

then reviewed in the diff, per `Documentation/specs/plan-grammar/examples/corpus-case.md` §4–5's
two kinds of red.

**Corpora are load-bearing**: any change to matching behaviour must change this directory in the same
commit, and CI executes plans against these cases on every change.
