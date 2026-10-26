# 0054 — Carry an error's causes as a chain, and mark display text with segments beside it

- **Status**: accepted
- **Date**: 2026-10-26
- **Plan tasks**: S1.33 and S1.34, which build it; S1.29 and S1.31, which found the need
- **Evidence**: `cli/src/engine.rs`'s `render_error`, which special-cases `component-failed`;
  `engine/http-client`'s `describe`, which flattens a cause chain into one string;
  `cli/src/highlight.rs`, which styles plans by calling the kernel's `plan::tokens` and the
  `check` page by parsing its markers; engine-protocol spec §5.3 and §10.1; plan-grammar spec §3.3;
  rustc's `--error-format=json`, whose diagnostics carry structure beside a `rendered` string

## Context

Everything the engine writes for a person is a plain string: an error's `message`, a mismatch's
`message`, a plan's `text`, the `can-i-deploy` page's `text`. A host that wants more than to print
it has to recover structure the engine had and threw away. S1.29 and S1.31 met this three ways:

- **Errors are flattened.** A cause chain, such as reqwest's, becomes one string joined with `: `
  where it is produced. The CLI then guesses at the layers: `render_error` leads a
  `component-failed` error with the component's own code, a rule no other host knows.
- **Plans are re-tokenized.** The CLI colours a plan by calling `plan::tokens` in the kernel. That
  works only because the CLI is Rust and links the kernel. Neither SDK can, and the CLI is meant to
  reach the engine only through the protocol.
- **Pages are parsed.** S1.31 colours the `check` page by reading its markers, although the
  structured `report` arrives beside the `text`.

Two constraints bound the answer:

- **The text is load-bearing.** The corpora pin it, pipes and logs receive it, and an old host reads
  only it. It must not change.
- **The engine does not style** (plan-grammar spec §3.3). It does not know where its text is going.
  It may say what a piece of text *is*. How that looks is the host's choice.

## Decision

1. **An `EngineError` may carry `chain`**, its layers outermost first:
   - Each entry is `{ code?, message }`, where `message` is that layer's own words only.
   - The first entry is the error itself, and the last is the root cause.
   - `message` keeps its meaning: the complete sentence, so a host that reads only it loses
     nothing. It is normally the layers' messages joined.
   - A layer that only names the library in the way (reqwest's "error sending request") is left out
     by whoever produces the chain, as `describe` leaves it out of the string today. That knowledge
     belongs with the producer.
2. **A component's error document may carry the same `chain`** (component-interfaces spec). For
   `component-failed`, the kernel writes its own layer first, then the component's chain, or the
   component error's `{code, message}` when it has none. That copies the component's layers into
   place without interpreting them, so §10.1's "MUST NOT translate" holds. `details.error` stays as
   specified.
3. **A host renders a chain by one rule:** lead with the innermost layer that has a `code`, then
   the root cause, when it is a different layer. The CLI prints:

   ```
   janus verify: broker-unreachable: could not reach https://broker.example
     caused by: Connection refused
   ```

   A global `--error-chain` option prints every layer. `render_error`'s special case goes.
4. **A display string may have a `segments` sibling**: for a member `text`, `text-segments`, and for
   `message`, `message-segments`.
   - It is a list of `{ text, kind? }` whose texts, joined, are exactly the string. A host may
     check this, and must print the plain string when it fails.
   - `kind` names what the piece is, never how it looks. Without one, the piece is plain.
   - Pieces of text, not character offsets: Rust counts UTF-8 bytes, JavaScript and the JVM count
     UTF-16, and a list of pieces has no units to disagree about.
   - It is flat. Nesting would serve an interactive viewer, which no host is.
5. **The kinds are one open vocabulary**, defined once in the engine-protocol spec with
   `x-known-values`. A host maps each kind to its own look, and prints a kind it does not know plainly:
   - outcome: `pass`, `fail`, `warn`, `review`, `quiet`;
   - reference: `path`, `code`, `heading`;
   - value: `string`, `number`, `literal`;
   - plan grammar: `label`, `action`, `operator`, `annotation`, `arrow`.
6. **Segments are asked for.** A host declares the `segments` capability in `engine/hello`
   (§5.3), and the engine writes segments for the rest of the session. Without it, nothing
   changes, on the wire or in its size.
7. **Text and its segments come from one writer.** The kernel's renderers write through a builder
   that produces both, so they cannot drift apart. The plan tokenizer becomes how the plan
   renderer's segments are checked: a corpus test asserts that every recorded text form's joined
   segments are its text.
8. **First carriers:** the plan's pretty and executed text forms, wherever they are sent; the
   `subsumption/decide` and `subsumption/check` text; and a mismatch's `message`, where the path
   and the values it quotes are the pieces worth marking. Other display strings gain segments when
   a host needs them, under the same rules.

## Alternatives considered

- **Markup in the string (XML, HTML, Markdown).** Every quoted value would need escaping, every host a
  parser, and a host or log that prints the string as it is would show tags. It changes the text
  that the corpora pin and an old host reads.
- **Structured documents only, with each host rendering its own text.** Most of the data exists
  already, as `report` does beside `text`. But every SDK would grow a renderer, they would drift, and
  "SDKs are thin" rules it out. The engine writes the text so that every host says the same thing.
- **Offsets into the string** (LSP's semantic tokens, chat platforms' message entities). These are
  compact, but they need a counting unit, and the three languages that read this protocol count
  differently.
- **Build chains with anyhow's `.context()`, or error-stack.** anyhow's context layer is only text,
  so the code the rendering rule needs would have to be attached and downcast. An `anyhow::Error`
  cannot be cloned, serialized or carried across the WASM component boundary, where chains also
  arrive as data from components in other languages. The kernel already declines anyhow for this
  reason (`engine/kernel/src/error.rs`). error-stack carries codes as attachments, but it would be a
  third error type to convert at every boundary. The standard library's `source()` and a small
  `Context` trait the kernel owns do the same work and keep errors as values (S1.33).
- **A recursive `cause` member.** This mirrors `Error::source()`. But an array is simpler to walk in
  every SDK language, and no reader needs the nesting.
- **Make `message` the outermost layer only.** This removes duplication. But a host pinned to
  protocol v1 would lose every root cause, which §11.1 rules out.

## Consequences

- Both members are additive under §11.2, with no protocol version bump. The SDKs' generated
  bindings gain them on the next regeneration.
- The CLI stops linking the kernel to style plans, and stops parsing the `check` page:
  `cli/src/highlight.rs` becomes a map from kinds to styles. Any SDK can do the same.
- A chain maps onto each language's own causes: JavaScript's `Error.cause`, the JVM's exception
  causes. Whether an SDK surfaces it that way is the behavioural specification's to decide, with
  conformance cases.
- A session that asks for segments roughly doubles the size of the strings that carry them. This is
  why they are asked for.
- Every kernel renderer that writes a carrier moves to the builder, and every producer of a chained
  error stops flattening it. `describe` keeps its noise rule and drops those layers from the
  chain.
- **Tripwire:** if a host needs nesting (an error inside a path inside a message), or needs to
  target a piece, as an editor's quick-fix would, flat segments are not enough. Revisit with a
  superseding ADR then, not by nesting segments ad hoc.
