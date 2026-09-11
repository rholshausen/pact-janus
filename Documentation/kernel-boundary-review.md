# Kernel-boundary review

Plan task **3.8** (`[explore]`). Tests bet **B3** — "the kernel knows nothing about HTTP or JSON" —
against what tasks 3.1–3.6 actually built, rather than against what they were supposed to build.
Findings feed [component-interfaces spec §13](specs/component-interfaces/spec.md#13-where-full-symmetry-hurts)'s
"day-one components" report (which names 3.8 directly as one of the three inputs it is waiting on)
and Phase 4's transport/content-component work (tasks 4.1–4.2).

**Method.** A grep-driven pass over `engine/kernel/src/` for HTTP- and JSON-specific knowledge —
content-type sniffing, header/query/method conventions, hardcoded request/response shape, and any
place `serde_json::Value` is doing more than carrying the protocol's own JSON-typed documents (ADR
0002 already commits the *protocol* to JSON; that is not a B3 question) — followed by reading every
hit in context. Six findings, all cited to file and line so a later task can act on them without
re-deriving where they live.

## Verdict

**Holds for the part of the kernel that is supposed to be protocol-agnostic forever; does not hold,
by design and necessity, for the v1–v4 compatibility layer.** `plan::value::RuntimeValue`,
`plan::resolve`, `plan::render`, `plan::compile` (the shape compiler), `interaction_spec` and
`contract::model` are clean — checked below. The legacy compiler (design 3.5) is a compiler *for a
format that is inherently HTTP-shaped*, and reading a v1–v4 pact necessarily means knowing what a
request and a response are; that is not a bug to fix, it is what task 3.5 was asked to build. The
two findings worth real attention are the one place the kernel *guesses* rather than defers
(finding 1, already self-flagged in the code) and one place this session's own work reached for an
unnamespaced core action where a namespaced, component-owned one belongs (finding 2).

## Findings

### 1. The kernel sniffs content type by pattern-matching bytes — already self-flagged

`engine/kernel/src/plan/interpret.rs:897-905`:

```rust
fn detect_content_type(value: &RuntimeValue) -> Option<String> {
  let text = /* ... */;
  if text.starts_with('{') || text.starts_with('[') {
    Some("application/json".to_string())
  } else if text.starts_with('<') {
    Some("application/xml".to_string())
  } else {
    None
  }
}
```

Backing `match:content-type`, whose shape-language definition (spec §4.2) is explicit that
detection is a content component's job: "octets detected as that content type by the
content-detection rules of the content components in play." Component-interfaces spec §7.1 states
the intended design directly — "`json:parse` is a component action in a spec that has no JSON in
the kernel" — and this function is exactly the JSON the kernel is not supposed to have. It is
already named as this: the function's own doc comment (`interpret.rs:893-895`) says recognising it
as a stand-in "is exactly the finding task 3.8's kernel-boundary review exists to make." This
review confirms it is still there, unchanged, and still the clearest violation in the codebase.
**Action for 4.2**: the JSON content component (and whatever XML/other components follow) should
own detection; `match:content-type`'s dispatch should call out to whichever component is loaded,
erroring `component-unavailable` when none is, rather than guessing.

### 2. `match:header-value` is an unnamespaced core action with HTTP-specific semantics — new this session

`engine/kernel/src/plan/interpret.rs:801-815` (the action), `:920-950` (`header_values_match`,
`mime_parts`); emitted from `engine/kernel/src/plan/legacy.rs:385`. Implements v1–v4's default
header-value comparison: comma-separated values tolerate whitespace, and a MIME-shaped value
(`type/subtype; param=value`) compares type and parameters as a set. Plan-grammar spec §4.1 reserves
unnamespaced action names for the specification itself — "a component MUST NOT define one" — which
by the same logic means the kernel should not define one whose meaning is header/MIME-syntax
knowledge either. This is new leakage from task 3.5, not inherited from earlier work: the header
comparison quirk is real (confirmed against the 803-case corpus, `tests/fixtures/spec_testcases`)
but landed as a bare core action for lack of anywhere namespaced to put it, since no HTTP transport
component exists yet to own it. **Action for 4.2**: either the HTTP transport component contributes
`header:match-value` and the legacy compiler emits that instead, or — if the semantics are judged
genuinely transport-agnostic enough to justify staying core (a real design question, not decided
here) — that should be a recorded decision, not an artifact of where 3.5 happened to need it.

### 3. The legacy compiler's part/slot shape is hardcoded HTTP — expected, worth naming precisely

`engine/kernel/src/plan/legacy.rs:79-118` (`compile_request`/`compile_response`) hardcode the slots
`method`/`path`/`query`/`headers`/`body` and `status`/`headers`/`body` directly in Rust function
structure. Contrast `interaction_spec::InteractionSpec.parts: BTreeMap<String, Part>` (design 3.2),
whose slot names are open strings by design — `interaction_spec/model.rs`'s own doc comment: "which
slots a part has is the transport and content components' business, never this model's." The shape
compiler (task 3.3) never hardcodes what a part or slot *is*; the legacy compiler always does. This
is not a defect — v1–v4 pacts are HTTP request/response documents by specification, not by this
compiler's assumption — but it means **the legacy compiler is, permanently, an HTTP-specific
compiler**, and nothing in the module tree currently marks that boundary. See finding 6.

### 4. Method case-insensitivity is a hardcoded string comparison, not data

`engine/kernel/src/plan/legacy.rs:353`:

```rust
None if category_name == "method" => Node::action(
  "match:equality",
  vec![Node::action("lower-case", vec![resolve]), Node::action("lower-case", vec![...])],
),
```

v1–v4's "methods compare case-insensitively" default is expressed as a literal string comparison
against the category name inside the compiler, rather than as configuration a component would
supply. Small — one branch, well-contained, correctly tested (spec test case `method/method is
different case`) — but it is the same pattern as finding 2 in miniature: an HTTP convention encoded
as kernel `if` logic because there is nowhere else for it to live yet.

### 5. The legacy body compiler is typed to `serde_json::Value`, not the protocol's document model

`engine/kernel/src/plan/legacy.rs:51-76` (`LegacyRequest`/`LegacyResponse`, `body:
Option<serde_json::Value>`), `:609` (`compile_body_object`, over `serde_json::Map`), `:649`
(`compile_body_array`, over `&[serde_json::Value]`). The cascading/precedence walk that is design
3.5's actual contribution recurses over `serde_json::Value` at *compile time*, not over
`plan::value::RuntimeValue` — the content-agnostic model everything else in the kernel already uses
(finding-clean list, below). This matches the compiler's documented JSON-only scope (module docs,
`legacy.rs:33-38`) and a non-JSON body degrades safely to a flat scalar comparison rather than
panicking, but it means the *type signature*, not just the logic, would need to change before a
future content component could hand this compiler an XML- or form-decoded document. Worth flagging
now, before 4.2 picks a shape for that handoff, rather than after.

### 6. `legacy_pact.rs`'s adapter is necessarily HTTP-typed — flagging the crate-boundary question, not the fact

`engine/kernel/src/legacy_pact.rs:42-134` (`http_interactions`, `legacy_request`,
`legacy_response`, `convert_query`, `convert_headers`, `convert_body`) import and operate on
`pact_models::v4::http_parts::{HttpRequest, HttpResponse}` and `pact_models::bodies::OptionalBody`
directly. This has to be true — there is no way to read a v1–v4 pact's interactions without knowing
they are HTTP-shaped (or message-shaped, which this adapter explicitly excludes) — so this is not a
finding against the code, it is a finding about where the code *lives*. `legacy.rs` and
`legacy_pact.rs` sit as ordinary sibling modules of `engine/kernel/src/plan/` and
`engine/kernel/src/`, indistinguishable by location from `plan::compile`, `plan::render` or
`contract::model`, which are supposed to stay protocol-agnostic forever. Nothing in the module
tree currently signals "this half of the kernel is the permanent v1–v4 compatibility layer; that
half is the core."

## What is clean

Checked and confirmed free of HTTP/JSON-specific knowledge:

- **`plan::value::RuntimeValue`** (`plan/value.rs`) — exactly the protocol's document-model kinds
  (null, bool, number, string, array, object, bytes, entry); no JSON- or HTTP-named variant.
- **`plan::resolve::CapturedValues`** (`plan/resolve.rs`) — its own doc comment states the rule
  directly: "What is deliberately not here: decoding wire bytes or JSON text into a document."
- **`plan::render`** (`plan/render.rs`) — renders `RuntimeValue` generically; the one
  content-specific-looking choice (a placeholder text form for bytes) is a display convenience, not
  a decoding assumption.
- **`plan::compile`** (the shape compiler, task 3.3) — walks the abstract shape tree, never an
  actual transported value, at compile time. `serde_json::Value` appears only as the *protocol's
  own* document-authoring format (a shape's `example`, ADR 0002), which is a correct and intentional
  use, not a content-type assumption — the distinction this review draws throughout.
- **`interaction_spec`**, **`contract::model`**, **`common::Transport`** — parts and slots are open
  `BTreeMap<String, _>`/`String` throughout; `Transport.kind: String` carries no hardcoded `"http"`
  anywhere. Confirmed by grep: no protocol-name literal appears in any of these modules.
- **`shape/*`** — the shape language's `content-type`, `regex` and `datetime` operators are all
  explicitly specified as deferring their real semantics to components (shape spec §4.2); the only
  place that promise is currently broken is the interpreter's fallback (finding 1), not the shape
  compiler itself.

## What this means for 2.6 and Phase 4

- Findings 1 and 2 are the concrete inventory for task 4.2's "day one" JSON and HTTP components:
  content-type detection and header-value MIME comparison are the two pieces of logic that need a
  real component home, not just a namespace change. Finding 4 (method case-insensitivity) is a small
  third item for whichever component ends up owning HTTP request defaults.
- Finding 5 says design 3.5's JSON-only scope is currently load-bearing at the type level, not just
  the documented-intent level — worth resolving explicitly (retype over `RuntimeValue`, or accept
  the coupling permanently) before content components multiply.
- Finding 6 is a structural question for whoever writes 2.6's day-one-components report: whether the
  v1–v4 compatibility layer (`legacy.rs` + `legacy_pact.rs`, and everything findings 3–5 describe)
  should move to a clearly labelled boundary — a submodule with its own `mod.rs` stating the
  exception, or eventually its own crate — so "the kernel knows nothing about HTTP or JSON" stays
  checkable by reading the module tree rather than by re-running a review like this one every phase.
- None of the six findings block Phase 4; all six are pre-existing, scoped, and (five of six) either
  self-documented or a direct, necessary consequence of design 3.5's own stated scope. The one net
  new risk worth carrying forward is finding 2: an unnamespaced action whose semantics are not
  actually protocol-agnostic is the kind of thing that gets harder to rename the longer it sits in
  the corpus (`corpora/legacy/v3-cascading-type/plan.txt` already renders it), so it is worth
  resolving at or before 4.2 rather than after.

## Resolution (task 4.2)

Task 4.2 built the JSON content component and the HTTP transport component (native binding, plan
task 2.6) and, in doing so, checked findings 2/4/5 against plan-grammar spec §4.4 — final, and more
authoritative than this exploratory review — before acting on them.

- **Finding 1 — fixed, with a tracked gap.** `plan::interpret`'s `detect_content_type` stand-in is
  gone. `match:content-type` now calls a real `ContentDetector` (`plan::interpret::execute_with_content`),
  backed by `engine/component-json`'s `detect`, which in turn uses `pact_models::content_types`'
  detector rather than a one-line `{`/`[` guess. No detector loaded is now its own distinct
  interpreter message rather than a silent guess. `corpora/shapes/content-type` is the proof:
  unchanged expected output, now produced by the real component (`tools/corpus` wires it in).
  **Gap**: `ContentDetector` is a single optional slot (`Option<&dyn ContentDetector>`), not a
  registry — every caller (`tools/corpus`, kernel tests) hands in one hardcoded `JsonContent`
  directly. There is no resolution mechanism yet answering component-interfaces spec §2.3
  ("the engine MUST collect the union of requirements across all interactions... and resolve to
  loaded components"), and no task currently owns building one. This is fine while exactly one
  content component exists; it stops being fine at Phase 8 task 8.1, the first point a second
  (third-party WASM) content component actually needs to coexist with this one — that task should
  either build a real `ContentRegistry` or explicitly re-scope to keep deferring it.
- **Finding 2 — not a defect; no change.** Plan-grammar spec §4.4's legacy-action table already lists
  `match:header-value` as *legacy only*, deliberately core, in the same family as
  `match:array-contains` and `match:min-type`/`max-type`: it encodes v1–v4's own *specified* default
  header comparison (validated against the 803-case pact-reference suite), not a kernel guess. This
  review's own finding 3/6 logic — "the legacy compiler is permanently HTTP-specific, and that's
  correct" — applies here too; the review left this open ("a real design question, not decided
  here") and §4.4 turns out to already have decided it. Separately, `header:parse` (§4.6, and the
  component-interfaces worked example's "header-value syntax is its own small content component,
  named `header`") is the *shape*-language's answer for matching header values with ordinary
  operators — a genuinely different feature, with no compiler emitting it yet on either path, so
  building a `header` component now would have no caller. Left for whichever task first needs it.
- **Finding 4 — not a defect; no change**, same reasoning as finding 2: `legacy.rs`'s method
  case-insensitivity compiles to *structure* over core actions (`match:equality`/`lower-case`), not
  a bespoke action, encoding a v1–v4-specified default exactly as the legacy compiler is supposed to.
- **Finding 5 — recorded, not changed.** v1–v4 pacts are JSON documents by the pact specification
  itself, permanently — there is no future non-JSON v1–v4 pact this compiler will ever need to read.
  Retyping the legacy body compiler over `RuntimeValue` would buy nothing a real caller needs.
- Findings 3 and 6 (the legacy compiler is permanently HTTP-shaped; nothing marks that boundary in
  the module tree) remain open, still feeding 2.6's day-one-components report rather than 4.2.
