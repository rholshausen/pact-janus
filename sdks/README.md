# Language SDKs

Thin SDK prototypes (plan Phase 6): `typescript/` and `jvm/`. An SDK is generated protocol bindings +
an idiomatic DSL and test-framework integration + the conformance suite run. No matching logic, no
orchestration beyond the engine-protocol operations. The
[SDK specification](../Documentation/specs/sdk-specification/spec.md) (task 2.9) governs what goes
here — each SDK's own `STYLE.md` is a copy of that design's style-guide skeleton, filled in once per
language.

## Installing a release

Until they graduate into pact-js and pact-jvm (staged plan S2.9), each SDK is released from this
repository as files on a GitHub release of its own, tagged `typescript-sdk/v<version>` or
`jvm-sdk/v<version>`, and published to no registry
([ADR 0060](../Documentation/decisions/0060-release-each-sdk-as-files-on-a-github-release-of-its-own-and-fetch-its-pinned-engine.md)).
Each release's notes say how to install it, with its version filled in:

```bash
npm install --save-dev https://github.com/rholshausen/pact-janus/releases/download/typescript-sdk/v<version>/pact-janus-<version>.tgz
```

```kotlin
repositories {
  mavenCentral()
  ivy {
    url = uri("https://github.com/rholshausen/pact-janus/releases/download")
    patternLayout {
      artifact("jvm-sdk/v[revision]/[module]-[revision](-[classifier]).[ext]")
      ivy("jvm-sdk/v[revision]/[module]-[revision].ivy.xml")
    }
    content { includeGroup("io.pact.janus") }
  }
}

dependencies {
  testImplementation("io.pact.janus:janus-sdk:<version>")
}
```

Neither carries the engine. Told no other one to run (`JANUS_ENGINE`), an SDK runs the
`janus-engine` release its pin names, which it fetches on first use into `JANUS_ENGINE_CACHE` (by
default `pact-janus/engines` in the user's cache directory), and refuses any file whose SHA-256 is not
the pinned one. A network that cannot reach GitHub names a server holding the same files as
`JANUS_ENGINE_MIRROR`, laid out as `<mirror>/v<version>/janus-engine-<triple>`. Node's `fetch` uses
`HTTPS_PROXY` only when `NODE_USE_ENV_PROXY=1` is set; the JVM uses its own proxy properties
(`https.proxyHost`).

`release-check/` in each SDK is a project outside it that installs a release and uses it as a user
would, with nothing naming an engine. `release-sdks.yml` runs it before publishing, on each operating
system an engine is released for.

## TypeScript (task 6.2)

[`typescript/`](typescript/) runs the RFC's consumer example near-verbatim against the real engine —
see [`test/vitest-integration.test.ts`](typescript/test/vitest-integration.test.ts). Its DSL is the
[canonical behavioural specification](../Documentation/specs/sdk-specification/behavioural-spec.json),
spelled as [`STYLE.md`](typescript/STYLE.md) records. It embeds the engine as the `janus-engine`
subprocess, named by `JANUS_ENGINE`; `npm test` builds one first. That is every SDK's primary embedding
([ADR 0023](../Documentation/decisions/0023-the-subprocess-is-the-primary-embedding-and-wasm-serves-offline-operations.md)):
a WASM engine cannot host a consumer test's mock server —
[Phase 9 finding 3](../Documentation/prototype/phase-9-findings.md).

## JVM (task 6.3)

[`jvm/`](jvm/) is Java 17, written from the SDK specification alone, without reading the TypeScript
SDK. That is plan task 6.3's test of whether the specification transmits behaviour. The result is
[`Documentation/jvm-sdk-from-spec-report.md`](../Documentation/prototype/jvm-sdk-from-spec-report.md). The
idiomatic layer is the Gradle project `jvm/sdk` (`io.pact.janus.sdk`), spelled as
[`STYLE.md`](jvm/STYLE.md) records, with a JUnit Jupiter extension that finalises after each test class.
The RFC's consumer example runs against the real engine in
[`OrderConsumerTest`](jvm/sdk/src/test/java/io/pact/janus/sdk/OrderConsumerTest.java). It embeds the
engine as the `janus-engine` subprocess, named by `JANUS_ENGINE`. `./gradlew build` (from `jvm/`) builds
the engine with cargo before the tests run. Chicory, which the superseded ADR 0003 made the JVM's
primary, cannot host the mock server either ([Phase 9 finding 3](../Documentation/prototype/phase-9-findings.md)).
The `io.pact.janus.sdk.engine.FramePipe` interface is where one would plug in.

## Conformance (task 6.4)

Layer 3 of every SDK, and the only one that can fail a build for the right reason: both SDKs run the
shared corpus under [`conformance/`](../conformance) as part of their own tests — TypeScript in
[`test/conformance.test.ts`](typescript/test/conformance.test.ts), the JVM in
[`ConformanceTest`](jvm/sdk/src/test/java/io/pact/janus/sdk/conformance/ConformanceTest.java) — and
each writes a report `cargo run -p pact_janus_conformance -- check` reads. That, and not a
maintainer's assertion, is what "conformant" means ([ADR 0017](../Documentation/decisions/0017-sdk-conformance-is-suite-passing-not-prose-matching.md)).
What the first run of it found is in
[`Documentation/conformance-suite-report.md`](../Documentation/prototype/conformance-suite-report.md): both
SDKs pass every case, including the same recorded interaction content for the RFC example.

A driver knows only how its language spells a primitive; it never decides what one means. Adding a
case means adding JSON to the corpus, not code to either SDK.

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
