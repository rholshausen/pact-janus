# TypeScript SDK style guide

This SDK's copy of the [style-guide skeleton](../../Documentation/specs/sdk-specification/examples/style-guide-skeleton.md)
(SDK spec §5). It records how the [behavioural specification](../../Documentation/specs/sdk-specification/behavioural-spec.json)
is *expressed* in TypeScript — never a different semantics. Where this file and the behavioural
specification disagree, the behavioural specification wins and this file has a bug.

## Naming and module layout

- Primitive ids become camelCase functions and methods: `each-like` → `eachLike`, `any-of` → `anyOf`,
  `one-of` → `oneOf`. `janus` is the `Janus` class (`useJanus` under Vitest); `interaction`, `execute`
  and `finalise` are its methods; `given`, `request` and `response` are methods of the builder
  `interaction` returns. The `literal` primitive is not a function — it is what happens to a plain
  value written where a shape is expected. `when-variant` and `variant-cases` are `whenVariant` and
  `variantCases`, written as a `given` param's value; `variantCases`' default is a trailing optional
  argument, so "no default" and "a default of `undefined`" cannot be confused.
- A primitive whose `signature` is empty is still a function, called with no arguments —
  `forbidden()`, not a `forbidden` constant. Every shape helper is then written the same way, and a
  call is a fresh node rather than one document fragment shared by every use of it.
- Layout, by SDK layer (SDK spec §2):
  - `src/generated/` — layer 1, the generated bindings (task 6.1). Never edited, never linted.
  - `src/engine/` — the embedding: `pipe.ts` (the frozen frame pipe), `subprocess.ts` (the
    `janus-engine` embedding), `engine.ts` (typed operations over the pipe).
  - `src/shapes.ts`, `src/interaction.ts`, `src/janus.ts`, `src/errors.ts` — the idiomatic layer.
  - `src/vitest.ts` — test-framework integration, a separate entry point (`pact-janus/vitest`) so the
    core never imports a test framework.
- Generated types are used inside the idiomatic layer and appear in its public types where the
  protocol's own document *is* the answer (`Variant` is `protocol.VariantDescriptor`; `build()`
  returns `contract.InteractionSpec`). Users import from `pact-janus`, not from `generated/`;
  `pact-janus/bindings` exists for hosts that speak the protocol directly.

## The async model

- Every operation that reaches the engine returns a `Promise`. Building an interaction is synchronous:
  it makes no call.
- The `execute` closure may be sync or async; it is awaited per variant, so variants never overlap. A
  thrown error or rejected promise fails that variant, is collected, and does not stop the remaining
  variants; `execute` then rejects once with every failure.
- One `Janus` object serves one suite, sequentially. Two `execute` calls in flight on one object at
  once are not supported: the engine's mock arms one variant at a time per transport (plan task 4.5's
  exchange loop), so concurrent tests would serve each other's variants. Vitest runs a file's tests
  sequentially by default; `test.concurrent` must not be used with a shared `Janus`.

## The error surface

| Failure | Type | What it carries |
|---|---|---|
| any engine error frame (engine-protocol spec §10) | `JanusError` | `code`, `category` (absent → `internal`), `details` and `problems` verbatim; the message leads with the interaction's description and lists each problem's `pointer` |
| one or more closures threw | `VariantsFailedError` | `failures[]`: each variant descriptor and its cause; the message names each by label and id |
| no contract written | `ContractWithheldError` | the engine's `results`, and `failedTests` — interactions whose `execute` failed |
| the embedding itself (no `JANUS_ENGINE`, the process died) | `Error` | what the pipe saw; these are not engine outcomes |
| a provider run whose status is `failed` | `VerificationFailedError` | `result` (status, summary, results) and `failures`: the failed results that are not pending |
| a verifier naming no source, or two | `TypeError` | thrown by `verify` before anything is sent |

One `JanusError` class, not one per code: codes are an open vocabulary (engine-protocol spec §2.2), so
a subclass per code would be a closed set pretending to be open. `details` stays a plain record for
the same reason; `problems` is the one member promoted to a typed accessor, because two primitives'
`errors` entries name it.

## Builder ergonomics

- A fluent chain, as in the RFC: `janus.interaction(d).given(...).request({...}).response({...})`.
  `request`/`response` take an options bag with the slot names as keys.
- A shape helper returns an instance of the `Shape` class; `compile` recognises helpers by
  `instanceof Shape`, so a plain object literal with a `shape` member is still a plain object
  (behavioural spec `literal`).
- Bare values follow the `literal` rule exactly; headers and query follow `request`'s name-to-list
  rule: `headers: { Accept: 'application/json' }` means `accept` with the one value
  `application/json`. A value may be a string, a whole number (`{ 'X-Count': 3 }` sends
  `X-Count: 3`), a boolean, a list of those, or a shape helper; `MultiValue` is that type, and
  anything else — a fractional number, a `Date`, `null` — is refused at the call, as are two names
  that collide once lower-cased (ADR 0019). Note that this is an exact comparison — see
  [Phase 9 finding 1](../../Documentation/prototype/phase-9-findings.md) for why a provider adding
  `; charset=utf-8` to a *response* header will fail it today.
- `content(mediaType, document)` returns a `Content`, not a `Shape`: it names a whole slot, so it is
  its own class, and `RequestParts.body`/`ResponseParts.body` are the only members typed to accept
  one (`Template | Content`). Anywhere else the type checker refuses it, and `compile` refuses it at
  run time for a caller that cast around the types (behavioural spec `content`, ADR 0019).
- Components are the `components` member of `JanusConfig`, a list of `ComponentDeclaration`s — the
  project configuration's own document, typed loosely because its vocabulary is open. The SDK does
  not read `consumer.janus.yaml`: a YAML parser would be the SDK's first runtime dependency, and the
  behavioural specification asks only for the declarations (third-party component report §4).

## Test-framework integration

- Vitest: `useJanus(config)` at the top of a test file (or `describe`) creates the suite's `Janus`
  and registers `afterAll(() => janus.finalise())`, so the session is released and the contract
  written after the last test, pass or fail — and a withheld contract fails the suite.
- Without Vitest, use `new Janus(config)` and call `await janus.finalise()` yourself, in whatever
  after-all hook the framework has.
- `JANUS_ENGINE` names the `janus-engine` executable (or pass `engine: { command }`); the engine's
  stderr is inherited, with `RUST_LOG` defaulting to `warn` so a request the mock could not match
  is logged and start-up chatter is not.

## Provider verification

- `new Verifier({ providerUrl, pacts | broker | urls, options?, transport?, config?, components?, engine? })`,
  then `.state(name, setup | { setup, teardown })`, `.requestFilter(filter, { changes? })` and
  `.message(description, produce)`, each returning the verifier, then `await verifier.verify()`
  (ADR 0055, ADR 0056). Nothing is sent before `verify`. `transport` is the HTTP transport's options
  beside `base-url` (`headers`, `client`), passed as given.
- The SDK's own options are camelCase; the documents it passes through — `broker`, `urls`,
  `options` — are spelled as the protocol spells them (`"consumer-version-selectors"`,
  `"provider-version"`), because they are the protocol's documents and the SDK translates none of
  them.
- A state's `setup` returns an object of provider-state values, or nothing. A handler that cannot
  reach its state throws `StateUnsupportedError` with the reason; anything else it throws is a
  failed setup.
- A request filter receives `{ method, path, query, headers, … }`, each slot's content as the engine
  sent it (headers as lists of values), and returns the request; returning nothing changes nothing.
- A message producer receives today's request for the message (`{ description, providerStates,
  request? }`) and returns `{ body, contentType?, metadata? }`. The body is a `Uint8Array`, a string,
  or any other value, written as JSON. With no `contentType` it is unlabelled, and the engine reads
  its type from the metadata, else its octets.
- `verify` resolves with the result for `verified` and `filtered` runs: a test that must not pass a
  filtered run checks `result.status`. It rejects with `VerificationFailedError` for `failed`.

## pact-js's provider API (`pact-janus/verifier`)

- `Verifier`, `MessageProviderPact` and `providerWithMetadata`, taking pact-js's options by pact-js's
  names (ADR 0056 decision 4). Each option becomes the protocol member S1.13 maps today's CLI option
  to; [`provider-test-migration.md`](../../Documentation/provider-test-migration.md) has the table.
- It is `src/facade/`, the thinness audit's `facade` layer, with a budget of its own: the one place
  in this SDK that spells someone else's API, so the one most likely to grow logic.
- An option with no Janus meaning that would change a run is refused by name with a `TypeError`
  before an engine starts (`beforeEach`, `providerStatesSetupUrl`, …). One that only tuned pact-js's
  proxy or logs is accepted and does nothing.
- It imports no test framework, so it runs under Jest and Vitest alike.

## Packaging and distribution

- npm, ESM only, Node ≥ 22.6 (the generator script uses type stripping). Not published: this is the
  prototype's `pact-janus` package, `private: true`.
- The engine is not bundled: the subprocess embedding needs the `janus-engine` binary, located by
  `JANUS_ENGINE`. A published SDK would pin an engine version and ship or fetch the matching binary
  (or, once Phase 9 finding 3 is resolved, a WASM component).
- The protocol version this SDK speaks is fixed at 1 (`engine/engine.ts`) and offered in `engine/hello`.

## Deviations from the behavioural specification

| Primitive | Deviation | Reason |
|---|---|---|
| `execute` | Each selected variant is a line in `execute`'s failure, not a separately reported test | Vitest cannot add tests to a test that is already running, and naming variants at collection time would need an engine session before any test runs. The failure message names every failed variant by label and id, which task 4.6 found is what makes a failing variant identifiable (SDK spec §5 SHOULD). |
| `finalise` | "Members in the order the engine returned them" holds except for integer-like member names, which are written first, in ascending numeric order | The contract arrives parsed, inside a frame, and JavaScript enumerates integer-like keys first whatever order they were written in; `JSON.stringify` of the parsed document is otherwise the engine's canonical form ([Phase 9 finding 6](../../Documentation/prototype/phase-9-findings.md)). |
