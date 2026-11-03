# 0066 — Let a compatibility facade submit today's builder calls, and convert them as `upgrade` does

- **Status**: accepted
- **Date**: 2026-11-03
- **Plan tasks**: S2.5a; S2.5b, S2.5c and S2.12 build on it
- **Evidence**: pact-js's consumer builders (`src/v3/pact.ts`, `src/v3/ffi.ts`, `src/v4/http/`,
  `src/v4/graphql/`), which make every interaction by calling the FFI, and its matchers
  (`src/v3/matchers.ts`), which are data in the FFI's integration JSON; pact-core's bindings
  (`src/consumer/index.ts`), which name the FFI function behind each call; pact-reference's
  `pact_ffi` at `1f1c2e61` (`src/mock_server/handles.rs`, `bodies.rs`, `xml.rs`), which turns those
  calls into a v4 interaction; `engine/kernel/src/legacy/upgrade.rs` (S1.25, ADR 0047, ADR 0048),
  which already turns a v4 interaction into shapes; variant-semantics spec §3.2's `base-only`
  strategy; pact-js's example consumer suites, whose pacts are fixtures in S1.1's corpus

## Context

S2.5 is a facade over today's consumer DSLs, so most existing consumer tests need only mechanical
changes. SDK spec §6 imagined it as a mapping in each SDK, from each old primitive to a new one, with
a `kept`, `adapted` or `dropped` verdict on each. Building pact-js's showed that mapping is the wrong
place for the work.

pact-js's matchers are not functions over values. `like(v)` returns `{ "pact:matcher:type": "type",
"value": v }`, data in the FFI's *integration JSON*. pact-js's builders pass that data, call by call,
to the FFI, and `pact_ffi` turns it into a v4 interaction with matching rules and generators. Tests
depend on the data: `Matchers.reify` and `extractPayload` read it, and users write integration JSON by
hand. pact-python, pact-go, pact-net and pact-php are built the same way, on the same
FFI calls and the same integration JSON. pact-jvm's DSL builds the same rules in Java.

What a v4 interaction means as shapes is already decided once, in the engine. `janus upgrade` (S1.25)
converts a pact's interaction into shapes, reporting every loss as a finding, and S1.1's harness
verifies each upgraded fixture against the provider that answers its pact. A facade that mapped each
matcher to a shape in TypeScript would decide it a second time, and pact-jvm's a third. Each copy
would have to agree with `upgrade` by hand. SDK spec §6's own worked example shows the drift: it
classifies `like(object)` as `dropped`, while `upgrade` converts the same rule faithfully, writing
`type` at each position the rule reaches.

## Decision

1. **`add-interaction` takes today's builder calls.** `consumer-session/add-interaction` takes either
   `interaction`, an interaction specification, or `pact`: `{ specification?, steps }`. `steps` lists,
   in order, the calls a consumer library makes to `pact_ffi` to build one interaction. Each step is
   `{ call, … }`, named after the FFI function without its `pactffi_` prefix and carrying its
   arguments by name. `specification` is `V2`, `V3` or `V4` (the default), the version the library
   would write. In S2.5a the calls are the ones pact-js's HTTP builders make: `upon_receiving`, `given`,
   `given_with_params`, `with_request`, `with_query_parameter`, `with_header_v2`, `with_body`,
   `with_binary_file`, `with_multipart_file_v2`, `with_matching_rules` and `response_status_v2`. A
   value the FFI takes as integration JSON in a string is that JSON value in the step, not text. A file
   the FFI would read is read by the host: the step carries its name and its octets, base64-encoded.
   The engine reads no file. The vocabulary is open: an S6.1 SDK, or S2.5c's message calls, adds the
   calls it makes.
2. **The engine applies each call as `pact_ffi` does**, to an interaction that starts empty, in
   `engine/kernel/src/legacy/builder.rs`. The integration JSON, XML builder and multipart handling is
   ported from `pact_ffi` at the commit above, with its provenance recorded, and stays with the other
   v1–v4 code (S1.11). A step the FFI would reject, such as an unknown call, a missing argument, or an
   XML builder document it cannot read, is `interaction-invalid`, with a problem pointing at
   `/pact/steps/<n>`.
3. **Then the interaction is converted as `janus upgrade` converts one.** It is upgraded as a
   one-interaction v4 pact, and the converted interaction becomes the session's interaction
   specification, so everything after it, from variants to the recorded contract, is a normal
   interaction's. `add-interaction` answers `{ handle, findings? }`. `findings` lists the conversion's
   findings in contract-file spec §8.4's vocabulary, with pointers into the built interaction, and is
   present when there are any. The contract a facade test records is therefore the one `janus upgrade`
   writes from the pact today's library would have written, and S1.1's harness already holds those to
   the pact's verdicts.
4. **The facade keeps today's data and calls, and selects the base variant only.** pact-js's matchers
   are its integration JSON, returned unchanged, and its builders record the calls they would have
   made. Each `executeTest` asks for variants with `strategy: base-only` (variant-semantics spec §3.2),
   so a test runs once, against the example, as it does today. A test that wants variants moves to the
   Janus DSL.
5. **A primitive's verdict is the conversion's.** For a facade built this way, SDK spec §6.1's three
   verdicts describe what the conversion does with the rule the old primitive writes. `kept`: the rule
   converts with no `lossy` or `judgement` finding. `adapted`: it converts with a `judgement`, which the
   migration note states. `dropped`: it is refused, or converts with a `lossy` finding, and the note
   says what to write instead. The verdicts are published in each language's migration note, not on
   behavioural-spec primitives, since the facade maps onto `pact` steps rather than onto a primitive.
   SDK spec §6.2 says so.
6. **What has no conversion is refused by name, at the call.** `usingPlugin` is refused, as a v1–v4
   interaction that needs a pact plugin is refused by `janus verify` (ADR 0045). So is `tls: true`: the
   mock serves plain HTTP. What only decorated today's pact file is accepted, with no effect: `pending`
   (a contract has none, ADR 0051), comments, test names and references. The migration note lists
   each.
7. **A multipart body's boundary is derived from its parts, not drawn at random.** When no boundary is
   given, `pact_ffi` draws one at random, so the same test records different bytes on every run, and
   two runs merging into one contract (ADR 0065) conflict. The engine derives the boundary from a hash
   of the part's name and octets instead (ADR 0008: no randomness, anywhere). A part's `Content-Type`
   is the content type the call names, where `pact_ffi` guesses one from the file's extension.

## Alternatives considered

- **Map each matcher to a shape in the SDK**, as SDK spec §6 sketched. The conversion would be decided
  per language and checked against `upgrade` by hand. The facade budget would grow by the size of the
  FFI's body processing, in TypeScript and again in Java.
- **A declarative document: a v4 interaction whose values may be integration JSON.** It reads better
  than a list of calls, but `pact_ffi`'s result depends on their order. A body's content type falls
  back to a `Content-Type` header set before it, the body sets that header only when no call has, and a
  second multipart file joins the first. A document would have to fix one order and lose the others.
  Every FFI-based library already is a list of these calls, so recording them costs it nothing.
- **Port the FFI's processing to TypeScript.** The facade would then send a plain v4 interaction, but
  each FFI-based SDK would port the same processing again.
- **Depend on `pact_ffi`.** It brings a mock server, a plugin driver and a C ABI into the kernel, and
  cannot build for `wasm32-wasip2`.
- **Run every variant the converted shapes allow.** An upgraded `each-like` or `optional` would make a
  pact-js test run several times, against values its author never wrote. That is the Janus DSL's
  feature, and a test opts into it by moving to that DSL.

## Consequences

Easier: the facade holds no matching logic. pact-js's matchers are its own data, and its builders
record calls. pact-jvm's facade (S2.5b) can send the rules its DSL builds. Every FFI-based SDK S6.1
brings in gets the same conversion for the price of recording its calls. A facade test records what
`janus upgrade` would make of today's pact, so S1.1's verdicts and upgrade ledger cover it.

Harder: the kernel's legacy layer grows by the FFI's builder, about a thousand lines of Rust that must
follow `pact_ffi` as it changes. Where they differ, a facade test records something today's library
would not. The kernel tests replay pact-js's examples' calls and compare the result with the pacts
pact-js wrote, which are in S1.1's corpus.

Committed to: `add-interaction`'s `pact` form and its step vocabulary, `findings` in its answer, and
the verdict rule in decision 5.

Not decided here: the one interaction armed per transport and the one exchange per arming are the
kernel's, not this ADR's. A pact-js test that adds several interactions before one `executeTest`, or
sends one request twice, does not pass on the facade until the kernel arms several (S2.13). Messages
are S2.12 and S2.5c.

Tripwire: if pilot suites depend on `pact_ffi` behaviour the port does not reproduce, beyond decision
7's two, the port is following the FFI too loosely, and the kernel tests should replay those suites'
calls.
