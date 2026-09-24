# Pact Janus
> Prototype of the rebuild/redesign of the Pact framework from the ground up

<table>
<tr>
<td width="140">
  <img src="./Documentation/pact-janus-logo.svg" width="120" alt="pact-janus logo" />
</td>
<td>

A ground-up redesign and rebuild of the Pact contract testing framework based on the [RFC Pact MkII](https://github.com/pact-foundation/roadmap/pull/146).

Janus is the Roman god of doorways, transitions, and duality (two faces looking in opposite directions).

Pact is a testing framework with duality (consumer and provider looking in opposite directions).

</td>
</tr>
</table>

**Status:** the prototype is complete: every phase of the [project plan](Documentation/project-plan.md) is done. It existed to test the RFC's
bets against real code, so its findings are the deliverable, along with a [staged plan for the real build](Documentation/staged-implementation-plan.md).
It is not production software, it is not published anywhere, and it is not "Pact 6". **Start with the
[community report](Documentation/prototype-report.md).**

![The demo: a consumer test, verification with hooks, janus check, widening and fixing](demo/demo.gif)

## What it is

The RFC keeps Pact's core idea: consumer-driven contracts, captured by the consumer and replayed
against the provider. It rebuilds everything underneath. Janus is that design, built far enough to
find out whether it holds:

- **One engine, thin SDKs.** A single Rust engine behind a versioned, document-based protocol.
  TypeScript and JVM SDKs a few hundred lines each, both proven identical by one shared conformance suite.
- **Declarative interactions, compiled plans.** An interaction compiles to a matching plan that
  `janus explain` prints. Existing v1–v4 pacts compile to plans too, and verify unchanged.
- **Everything is a component.** HTTP, JSON and OAuth2 are components behind the same interfaces a
  third-party plugin uses. A `text/csv` plugin written from the docs alone runs as WASM, distributed
  as an OCI artifact.
- **Shapes with honest optionality.** `optional`, `anyOf`, `oneOf`, `eachLike`. Each declared variation
  is a variant the consumer test must pass. Providers can publish their own shapes, and `janus check`
  finds what they may send that a consumer never tested.
- **Scriptable lifecycle.** Hooks for auth, provider state and more, declared in configuration, with
  JavaScript as the escape hatch.

What held, what changed and what is still open is in the [community report](Documentation/prototype-report.md),
and in more detail in [RFC feedback](Documentation/rfc-feedback.md).

## Try it

You need a recent stable Rust toolchain (see `rust-toolchain.toml`), libclang (the engine embeds
QuickJS), and Node 22.6 or later with npm.

```sh
demo/run.sh                  # the RFC's whole loop, step by step (press Enter between steps)
```

Or drive the pieces yourself:

```sh
cargo build                                                   # engine, CLI (janus, janus-engine), sample provider
target/debug/janus explain samples/order-service/pacts/web-app-order-service.json   # the matching plan
target/debug/janus upgrade samples/order-service/pacts/web-app-order-service.json   # v3 pact -> contract, and what changed
target/debug/janus check samples/order-service/pacts/web-app-order-service.json \
  --provider-shape samples/order-service/shapes/                                   # what the consumer never tested
```

`janus verify` needs a running provider. The [demo](demo/README.md) and the
[sample provider's README](samples/order-service/README.md) show how.

## Building and testing

```sh
cargo test                                    # Rust workspace: engine, CLI, tools, golden corpora
(cd sdks/typescript && npm ci && npm test)    # TypeScript SDK, against a real janus-engine
(cd sdks/jvm && ./gradlew build)              # JVM SDK (JDK 17)
cargo run -p pact_janus_conformance -- check target/conformance/*.json   # both SDKs conformant
```

CI also runs clippy, rustfmt, the `wasm32-wasip2` build, schema-compatibility checks, the SDK thinness
audit and `demo/run.sh --ci`. [CONTRIBUTING.md](CONTRIBUTING.md) has the practicalities.

## What is where

| Path | |
|---|---|
| [`engine/`](engine) | The kernel (plan compiler and interpreter, shapes, variants, subsumption, the protocol), the built-in HTTP, JSON and OAuth2 components, the hooks host, and the WASM component loader |
| [`cli/`](cli) | `janus` (`verify`, `check`, `explain`, `upgrade`, `component push/pull`) and `janus-engine`, the subprocess every SDK embeds |
| [`sdks/`](sdks/README.md) | The TypeScript and JVM SDKs, with generated protocol bindings |
| [`conformance/`](conformance) | The shared corpus of cases both SDKs must pass |
| [`corpora/`](corpora/README.md) | Golden matching corpora: input, expected plan, expected result |
| [`samples/order-service`](samples/order-service/README.md) | The sample provider, with deliberate variance, auth and v3 provider states |
| [`demo/`](demo/README.md) | The RFC loop as a runnable, recorded walkthrough |
| [`third-party/janus-csv`](third-party) | A plugin written as a third party would, from the published specs only |
| [`benchmarks/`](benchmarks) | The `pact_ffi` baseline and the Janus performance harness |
| [`spikes/`](spikes/README.md) | Time-boxed experiments, each with its findings |
| [`Documentation/`](Documentation) | The charter, plan, specifications, decisions (ADRs) and reports |

## Key documents

- [Community report](Documentation/prototype-report.md): the short version, for everyone.
- [RFC feedback](Documentation/rfc-feedback.md): every question the RFC left open, answered with evidence.
- [Staged implementation plan](Documentation/staged-implementation-plan.md): what carries over, the
  build order, the gates that could change it, and the questions for the community.
- [Charter](Documentation/charter.md) and [project plan](Documentation/project-plan.md): what the
  prototype set out to prove, and how.
- [Specifications](Documentation/specs): the engine protocol, shape language, variant semantics, plan
  grammar, contract file, component interfaces, lifecycle hooks, subsumption check and SDK
  specification. Normative for the code.
- [Decision records](Documentation/decisions/README.md): every contested choice, and what would reopen it.
- [Phase 9 findings](Documentation/phase-9-findings.md): what using the prototype turned up.

## Get involved

The design discussion happens on the [RFC pull request](https://github.com/pact-foundation/roadmap/pull/146).

## License

[Apache 2.0](LICENSE).
