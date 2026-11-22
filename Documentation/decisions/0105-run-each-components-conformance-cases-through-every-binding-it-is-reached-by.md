# 0105 — Run each component's conformance cases through every binding it is reached by

- **Status**: accepted
- **Date**: 2026-11-23
- **Plan tasks**: S5.10; S5.11 (in-tree components answer `component/hello`), S5.9
- **Evidence**: `corpora/components/`, run by `tools/corpus/tests/components.rs`;
  `third-party/janus-csv/conformance/`, run by `janus component check` in `cli/tests/cli.rs`;
  `engine/kernel/src/component/wire.rs` and `conformance.rs`;
  [component-interfaces spec §9.4](../specs/component-interfaces/spec.md#94-dual-binding-conformance);
  [ADR 0012](0012-one-interface-two-bindings.md) decision 4, [ADR 0104](0104-the-component-interface-is-generally-available-for-content-and-matcher-components-over-wasm.md)
  decision 6

## Context

ADR 0012 decided that the built-in components implement exactly the interfaces a third party does, and
that a conformance corpus run through both bindings would make that "a fact rather than an intention".
Spec §9.4 specifies the corpus, a set of `(op, body)` calls with expected results, and its schema,
`conformance-case.schema.json`. No case existed, and nothing ran one. ADR 0104 declared the content and
matcher interfaces generally available without it, and made the corpus S5.10.

Three things stood in the way. The built-in components are Rust crates called through the native trait,
with nothing to compare them against. The engine's half of the WASM binding, which writes a native request
as a frame and reads the result back, was a private copy inside `engine/component-host`, so no test could
reach it except by loading a WASM guest. And a third party had no way to run a case against their
component except by writing a test harness of their own.

## Decision

1. **One codec, at both ends of the pipe, in the kernel** (`component::wire`). `PipeClient` is the
   engine's end: it implements the content and matcher interfaces over any `Pipe`, writing each request as
   its operation's document and reading the result. The WASM binding is a `PipeClient` over a wasmtime
   instance, and `engine/component-host`'s own copy is deleted. `serve` and `serve_frame` are a native
   component's end: they read a request document, call the component and write its result.
2. **The serialising in-process binding is a `PipeClient` over `InProcess`**, which writes every call as
   a request frame's bytes, serves it with `serve_frame` and reads the response frame's bytes back as the
   WASM binding reads a guest's. A built-in component reached this way is reached by the same code that
   reaches every out-of-tree component. Spec §9.4 allows this binding in place of compiling each built-in
   to a WASM guest.
3. **The built-in components' cases are `corpora/components/<component>/<case>.json`**, one case per file,
   in a directory named for the component. Every built-in the native embeddings register has cases:
   `json`, `text`, `form`, `xml` and `multipart`, and the HTTP component's matcher, `http` (36 to begin
   with). They cover each operation a component implements and each one it refuses, with what the
   failures are. `tools/corpus` runs them through the native and serialising bindings
   (`cargo test -p pact_janus_corpus`, and `cargo run -p pact_janus_corpus` beside the golden corpus). It
   validates every case against the schema, and fails when a built-in has no cases. They ship in
   `janus-spec.tar.gz` with the rest of `corpora/`, and the component-interfaces schemas now ship beside
   them.
4. **What a case asserts.** An `ok` is matched member-wise: a member the case does not mention is not
   constrained, an array is matched in order and must be as long, and a number's kind is how it is written
   (ADR 0072). An `error` is matched on its `code` and on every other member it names except `message`. A
   call with no `expect` asserts only that the bindings agree. Every binding must answer every call with
   the same whole document, so where two bindings differ in a member no case mentions, the case still
   fails. Calls in a case share no state. A component that asks for embedded content (spec §6.6) is two
   calls: the question, and the request repeated with the answers the case gives.
5. **The schema says what §9.4 says.** `Expected.error` referenced the whole `ComponentError`, which
   requires a `message`, while the schema's own description said a message is never asserted. It now
   requires only `code`. `serialising` is a known binding. Both changes loosen the schema, so they are
   additive.
6. **`janus component check <component.wasm> <cases>...`** runs a component's cases against it. It loads
   the component as a run does, with no grants, and calls it through the WASM binding, the one a run calls
   it through, with the kernel's runner, the same one `tools/corpus` uses. `component/hello` answers with
   the handshake the load made. Exit 0 when every case passes. Exit 1 when a case fails, or the component
   does not load, as `push` and `pull` exit for an artifact that is not what it claims. Exit 2 when a case
   cannot be read, checked before anything is loaded. Like `push` and `pull`, it does not speak the engine
   protocol: checking a component is not an engine operation. A third party keeps their cases beside their
   component, as `third-party/janus-csv/conformance/` does (12 cases).
7. **What the corpus found, fixed with it:**
   - Every built-in content component answered `decode-failed` and `unsupported-content-type` with
     category `component`. Spec §11.1 puts both in `document`, where `encode-failed` already was.
   - The HTTP component answered `matcher/apply` for an action it does not run with
     `operation-unsupported`. Spec §11.1 says `unknown-action`, and the CSV component already said so.
   Nothing in the engine dispatched on either, so no verdict changes.

## Alternatives considered

- **Compile each built-in to a WASM guest and load it with the WASM loader.** That is the WASM binding
  itself, but it means a guest crate and a `wasm32-wasip2` build per component in every test run. The
  engine's half, which is the part that could drift, is exercised either way.
- **A second codec for the serialising binding, leaving the WASM host's copy where it was.** Two codecs
  that agree prove nothing about the one out-of-tree components are reached by.
- **Compare results whole, with no member-wise matching.** A case would break each time a result grew a
  member additively, which component protocol v1 permits. The bindings are still compared whole against
  each other, so nothing slips through between them.
- **Run third-party cases through the engine protocol.** A run would need a contract to reach a component,
  and the answer would be a verdict, not the component's document. A component author debugging
  `content/encode` needs the document.

## Consequences

- "Corpora are load-bearing" now covers the components: a change to what a built-in component answers
  changes `corpora/components/` in the same commit, or `cargo test` fails.
- A component author has a second thing to run after the handshake, as spec §9.4 promised, and a worked
  example of a corpus beside the CSV component. The author guide says how.
- Not done, on purpose:
  - **An in-tree component has no `component/hello` document.** Spec §9.1 says the handshake happens for
    a built-in too, but the native embeddings register a built-in by name alone (`InTree`). So no case
    asserts a built-in's handshake, and `component/hello` through the native binding is
    `operation-unsupported`. That is S5.11.
  - `generator/generate` and `matcher/variant-space` have no cases, because no binding forwards them. S5.9
    adds them.
  - `third-party/janus-ndjson` has no cases. They are its author's to write.
  - The golden corpus is not run through the serialising binding. Its plans reach components only through
    the registry, and the cases above already hold every operation the registry calls.
- **Tripwire**: a case that can be expressed in one binding and not another. That is ADR 0012's tripwire
  (a), and spec §9.4 already says so. A component that passes `component check` and fails in a run, for a
  reason a case could have caught, means the check has drifted from the binding a run uses.
