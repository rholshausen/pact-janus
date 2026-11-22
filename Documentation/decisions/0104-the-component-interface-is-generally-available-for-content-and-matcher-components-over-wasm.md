# 0104 — The component interface is generally available for content and matcher components over WASM

- **Status**: accepted
- **Date**: 2026-11-22
- **Plan tasks**: S5.1d; S5.9 (generators and variant dimensions from a declared component), S5.10 (a
  component conformance corpus), S5.5 (out-of-process components)
- **Evidence**: [the component author guide](../component-author-guide.md) and the component written
  from it alone, `third-party/janus-ndjson`; `engine/component-host/tests/ndjson_component.rs`;
  `engine/kernel/src/component/loader.rs`'s handshake tests;
  [ADR 0012](0012-one-interface-two-bindings.md), [ADR 0021](0021-a-component-artifact-is-its-own-type-and-a-pin-is-fetched-by-digest.md),
  [ADR 0097](0097-record-each-declared-component-a-session-or-run-loaded-by-the-digest-a-declaration-would-pin.md)
  to [ADR 0103](0103-a-content-component-asks-for-embedded-content-in-its-answer-and-the-registry-decodes-it.md)

## Context

S5.1 was split so that "generally available" would come after the changes that were still reshaping the
component interface: operator-level substitution (S5.4, ADR 0100), what a decode failure is and `encode`
being given the shape (S5.3, ADR 0101), a check comparing a declared component's operators (S5.7, ADR
0102), and embedded content (S5.6, ADR 0103). With those done, a third party can be told what to depend on.

Saying so means saying what is built, and the specification describes more than is. A declared WASM
component's `content` and `matcher` interfaces are called. Its `matcher/apply`, `compare` and `prefix`
are forwarded. Nothing forwards `generator/generate` (spec §7.3) or `matcher/variant-space` (§7.4), the
WASM loader binds no transport (sockets are what its sandbox withholds, §13) and no hook, and the
subprocess binding (§9.3) is loaded by nothing until S5.5. A component declaring any of those loaded
anyway, and the part it declared was silently never used. Spec §9.4 also says CI runs a component
conformance corpus through both bindings. The schema exists, and the corpus does not.

There was no guide either. Task 8.1 built `janus-csv` from the specification, and found the WIT world
unpublished and the engine unable to say whether it could load anything. A specification is not a guide:
it says what must hold, not what to do first.

## Decision

1. **Generally available means a third party may depend on these, within component protocol v1:**
   - the WIT world `pact:janus-component@1.0.0` (`wit/component.wit`), which never changes;
   - the schemas under `component-interfaces/schemas/v1/` and the semantics the specification gives
     them, for the `component/*`, `content/*` and `matcher/{apply,compare,prefix}` operations, evolved
     only additively under engine-protocol spec §11.2, as CI's `schema-compat diff` already enforces on
     every change;
   - the declaration (`component-config.schema.json`), the two OCI artifact layouts a loader accepts
     (ADR 0021, ADR 0098), digest pinning and Sigstore signature verification (ADR 0099), and the record
     a contract and a verification summary keep of each component loaded (ADR 0097);
   - the error codes the specification lists, and the deprecated members it keeps (`content/compile`'s
     `fragment`, `component/hello`'s `plan-grammar-versions`), which an engine goes on accepting and
     ignoring for the whole of v1.
2. **Not covered:** the native binding's Rust API, which is for in-tree components and changes with the
   kernel (S5.6 added members to it); the subprocess binding, until S5.5 loads one; and every operation
   below. The specification keeps them, as what a later engine will call.
3. **What a handshake declares, the engine can call, or the component does not load.** A declared
   component that implements an interface its loader bound nothing for (`hook`, or `transport` over WASM),
   contributes `generators`, or declares an operator's `variant-facet`, fails to load as
   `component-invalid`, naming what it declared and the plan item that would make it callable. A
   component built for a later engine meets that refusal where it was silent, and the refusal names why.
   Generators and variant dimensions from a declared component are S5.9.
4. **A guide for component authors**, `Documentation/component-author-guide.md`: building against the WIT
   world, the frames, the handshake, writing a content component and a matcher, testing one natively and
   in the engine, publishing with `janus component push` or `wkg oci push`, signing with `cosign`,
   declaring, and what is recorded. It links the specification for every rule rather than restating it,
   and says what is not available.
5. **The guide was tested by a component written from it alone.** An author with no other knowledge of
   the engine, told to read the guide and what it links and nothing else, wrote `third-party/janus-ndjson`,
   an `application/x-ndjson` content component. What they could not find in the guide was fixed in it.
   The engine's own test pushes that component with `wkg` (when `JANUS_WKG` names one, and with
   `janus component push` otherwise), signs it, declares it pinned and signed, and runs a consumer session
   and a verification that record it.
6. **The component conformance corpus is a plan item, S5.10**, and spec §9.4 says it is not built. The
   checks a load makes, and the engine's own tests of the built-in components through the WASM binding's
   documents, are what hold the interface together until then.

## Alternatives considered

- **Declare everything the specification describes.** A third party would build a generator against a
  promise no engine keeps, and find out from silence.
- **Remove the uncalled operations from the specification until they are built.** They are additive when
  they arrive, and the specification is where a component author looks to see what is coming. Refusing
  them at load is enough to stop anyone depending on them by accident.
- **Wait for S5.9 and S5.10.** Content and matcher components are what the stage's exit needs (a second
  content component written outside the core team), and neither item changes what they depend on.
- **Have the guide's author be the engine's implementer, as task 8.1 was.** An author who already knows
  the engine reads a guide's gaps as obvious. The exit is a test of the guide, so its author knew nothing
  else.

## Consequences

- A third party can build, publish, sign and declare a content or matcher component, and an engine
  upgrade within v1 will not break it.
- A component that declares a generator, a variant facet, a hook or a WASM transport is refused where it
  loaded. No component outside this repository did, and the CSV worked example's handshake no longer
  declares the generator and facet it illustrated.
- `third-party/` holds a second component written from documents alone. Its author read only the guide
  and documents it links, and nothing else in the repository. They built a 183 KB component with 30
  frame-level tests that loaded and verified on the first try. They reported 17 findings, and every one
  is answered in the guide:
  - The largest, a quarter of their time: the guide showed an interaction where `janus verify` needs a
    contract, and gave no way to get one without an SDK. The guide now shows a complete contract as the
    engine writes one, and says why each variant must be produced by the provider.
  - The skeleton had no place for the one bit of state an instance keeps (whether it was greeted); it
    now shows one.
  - What the guide did not say, and they had to guess: what empty octets decode to, what a malformed body
    is answered with, how detection is weighed (it is not: the first component to name a type wins),
    which number distinctions to keep, where `compile` finds the operator list, and how to see that the
    component was used at all.
  None of the 17 needed an engine change.
- **Tripwire**: a change to anything decision 1 lists that is not additive. That is component protocol v2,
  a new WIT package only if the pipe itself changes, and a superseding ADR.
