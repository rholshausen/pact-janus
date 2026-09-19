# Language SDKs

Thin SDK prototypes (plan Phase 6): `typescript/` and `jvm/`. An SDK is generated protocol bindings +
an idiomatic DSL and test-framework integration + the conformance suite run. No matching logic, no
orchestration beyond the engine-protocol operations. The
[SDK specification](../Documentation/specs/sdk-specification/spec.md) (task 2.9) governs what goes
here — each SDK's own `STYLE.md` is a copy of that design's style-guide skeleton, filled in once per
language.

## TypeScript (task 6.2)

[`typescript/`](typescript/) runs the RFC's consumer example near-verbatim against the real engine —
see [`test/vitest-integration.test.ts`](typescript/test/vitest-integration.test.ts). Its DSL is the
[canonical behavioural specification](../Documentation/specs/sdk-specification/behavioural-spec.json),
spelled as [`STYLE.md`](typescript/STYLE.md) records. It embeds the engine as the `janus-engine`
subprocess, named by `JANUS_ENGINE`; `npm test` builds one first. The WASM embedding ADR 0003 names as
Node's primary cannot host a consumer test's mock server yet —
[Phase 9 finding 3](../Documentation/phase-9-findings.md).

## Generated bindings (task 6.1)

Layer 1 of every SDK: typed views of the spec schemas [`bindings.json`](bindings.json) names — the
engine protocol, shapes, variants and the Janus contract, the four designs whose documents an SDK
builds or reads. They are checked in, so a schema change shows up as a reviewable binding diff and
task 6.5's thinness audit can count the generated layer, and they are **never hand-edited**:

| SDK | Generated into | Generator |
|---|---|---|
| TypeScript | `typescript/src/generated/` — one module per set, plus `<set>.vocabulary.ts` | json-schema-to-typescript |
| JVM | `jvm/bindings/src/main/java/io/pact/janus/bindings/<set>/v1/` — one class per type, plus `Vocabulary` | jsonschema2pojo (Jackson 2 annotations) |

Regenerate both with `cargo run -p pact_janus_bindings -- generate` from the repository root; CI does
the same and fails on any difference. [`tools/bindings`](../tools/bindings/README.md) explains what the
pipeline does to a schema on its way to a generator, and why.
