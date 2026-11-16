# A provider-shape plugin for Drift

A proposal, for staged plan S4.9 and S4.10. Drift (PactFlow's provider drift detection) runs test cases
against a provider from its OpenAPI or AsyncAPI document, and its plugins are WASM components. A Drift
plugin that records what the provider answered into a Janus provider shape would give every Drift user
a `recorded` shape without writing a provider test in an SDK, and it would be the first real host of
`janus-engine.wasm` that is not a broker: ADR 0023 decision 2's offline artifact, embedded in another
WASM component.

It was written on 2026-11-16 against Janus `6a658cb` and Drift `b5683c4`.

## Contents

1. [Where it lives](#1-where-it-lives)
2. [What Drift tells a plugin](#2-what-drift-tells-a-plugin)
3. [Embedding the engine](#3-embedding-the-engine)
4. [The gap in the engine (S4.9)](#4-the-gap-in-the-engine-s49)
5. [The plugin (S4.10)](#5-the-plugin-s410)
6. [Messages (S4.10b)](#6-messages-s410b)
7. [The demonstration](#7-the-demonstration)
8. [Open questions](#8-open-questions)

## 1. Where it lives

In the Drift repository, as `Plugins/provider-shape-plugin`, beside the plugin it is modelled on,
`Plugins/reports-plugin`. Not in `third-party/`:

- **It points the other way.** `third-party/` holds components written *for* Janus from its published
  interfaces, which Janus loads. This is a Drift component that holds Janus: it implements
  `drift:plugins` (`Plugins/plugin-manager/wit/plugin.wit`) and is built on `drift-models`.
- **Its dependencies are private.** Drift is `pactflow/drift`, and its crates resolve from the
  `pactflow` registry, which this repository's CI cannot reach. Copying Drift's WIT here would be
  PactFlow's decision, not this repository's.
- **It takes the engine as a release.** From Drift, the plugin consumes `janus-engine.wasm` as a
  released file pinned by version and SHA-256, as the SDKs pin `janus-engine`
  (`sdks/engine-release.json`), and verifies its attestation. That is the case the offline artifact
  exists for.

It is named for what it does, not for the codename: `rename.sh` cannot reach another repository
(`Documentation/codename.md`), so `provider-shape`, installed as `provider-shape.plugin`, keeps the
name out of Drift.

The Janus side is S4.9, an engine change, and this document. S4.10 is the plugin, tracked here so the
work has an id and an exit; its code is Drift's.

## 2. What Drift tells a plugin

A plugin like `reports-plugin` exports `plugin-manifest` and `event-callback`, names the events it
wants in its manifest, and keeps its state in the instance between events: the host keeps one instance
per plugin for a run. The verifier is sequential, so events for one operation arrive in order. The
recorder needs four things, and the events carry all of them:

| The recorder needs | Drift's event | What it carries |
|---|---|---|
| Each operation's method and path template | `file:loaded` | The source file's details. The OAS plugin's `load-file` gives each operation an `id` (its `operationId`, else `method:path`) and `metadata` with `path`, the template as OpenAPI writes it, `method`, and `idAlias` |
| Which operation each test ran | `testcase:loaded` | The test case file: each operation's `key` and its `target`, written `source-name:operationId` (`target: product-oas:getAllProducts`), resolved as `lookup_target` resolves it, by `id` or alias |
| The answer | `operation:invoked` | `(operation key, response)`, fired after invocation and before validation, for HTTP (`Verifier/verifier/src/operations.rs:271`) and for every async mode (`:338`). An HTTP response is the host's `{ status_code, headers, body }`, `body` as bytes (`Plugins/plugin-manager/src/plugins.rs`, `build_response_data`) |
| A name for the entry | `operation:started` | `(index, description, key, test case title, dataset)` |

`operation:prepared` (`(key, plan)`) also carries the request the OAS plugin built, including the
`servers` the operation may be invoked against, which §5 uses for the base path.

The path template is the key fact. ADR 0086 decision 1 has a provider's own test name its operation,
because nothing else knows the template; in Drift, the OpenAPI document does, so a Drift test names
nothing and the plugin still writes a selector by operation (ADR 0025), as `janus derive` does for
the same document.

## 3. Embedding the engine

A WASM guest cannot host WASM (ADR 0013), so the plugin cannot load `janus-engine.wasm` at run time.
It is composed instead. The plugin's world imports the engine's pipe beside Drift's interfaces:

```wit
package component:provider-shape-plugin;

world provider-shape-plugin {
  import drift:plugins/logger;
  import drift:plugins/config;
  import pact:janus-engine/pipe@0.1.0;

  export drift:plugins/plugin-manifest;
  export drift:plugins/event-callback;
}
```

and the build plugs the released engine into it:

```sh
cargo build --target wasm32-wasip2 --profile=release-plugin
wac plug target/wasm32-wasip2/release-plugin/provider_shape_plugin.wasm \
  --plug janus-engine.wasm -o provider-shape.plugin
```

Drift loads one component, whose imports are Drift's and WASI's. The engine imports only
`wasi:cli`, `wasi:clocks`, `wasi:filesystem`, `wasi:io` and `wasi:random` (`wasm-tools component wit`
on the release build), and Drift's plugin manager links `wasmtime_wasi::p2::add_to_linker_sync`, so
nothing else is needed. The plugin talks to the engine as every host does, one engine-protocol frame
in and one out (ADR 0002), starting with `engine/hello`.

To check first:

- **The WASI version.** The engine imports `@0.2.12`; Drift runs wasmtime 49. A spike instantiates the
  composed component under Drift's plugin manager before anything else is built.
- **Size.** The engine is about 9.8 MB, so the plugin is about ten times `reports.plugin`. It is
  compiled once per run.
- **One engine per instance.** The engine keeps its state in a `thread_local`, so one plugin instance
  is one engine, which is what the recorder wants.

## 4. The gap in the engine (S4.9)

`janus-engine.wasm` registers no transports (ADR 0062, ADR 0092): it has no thread for an exchange
loop. But `provider-shape-session/observe` reads an observation through what the transport for its
kind declares (engine-protocol spec §8.5, ADR 0086 decision 3), and in the engine that is a lookup in
the registered transports (`engine/kernel/src/protocol/engine.rs`, `recording_declarations`). So
inside the WASM engine an `http` observation is `component-unavailable`. The kind `message` works,
because ADR 0090 decision 3 gave it built-in declarations.

A transport's declarations are three things, none of them I/O:

- `exchange-parts`: which part starts an exchange and which answers it, so only the answer is recorded;
- `outcome-slots`: HTTP's `status`, written into the selector so a 404 never widens a 200;
- `credential-members` (ADR 0074): `set-cookie`, `authorization` and the rest, recorded as present or
  absent and never as values. Without them a recording would publish a provider's session cookies.

In `engine/component-http` they are methods of the transport (`src/transport.rs`), compiled only with
the `transport` feature the WASM build leaves off. S4.9 moves them out from behind the feature and
has the WASM engine use them, as ADR 0092 brought the component's matchers in. An ADR decides how
the kernel holds a kind's declarations with no transport: a registry of declarations the recording
reads before the transports, or a transport registered with no serve or drive role, whose
`start-transport` fails by name. The first keeps `engine/hello` from listing a transport the
artifact cannot start; the ADR weighs that.

Its test records the sample provider's answers through `janus-engine.wasm` under the `wasmtime`
crate, as `engine/component-host/tests/engine_wasm.rs` runs the artifact now, and gets the shape the
native engine records from the same observations, byte for byte.

## 5. The plugin (S4.10)

S4.10a is the HTTP half, against OpenAPI documents.

**Configuration**, under the plugin's name in Drift's configuration (`get-plugin-config`):
`provider`, the provider's name, required unless every loaded document has one `info.title`;
`output`, the file under the output directory (default `<provider>.provider-shape.json`, as
`samples/order-service/shapes/` names one); and `base-path`, for a server whose URL has a path the
OpenAPI `paths` leave out.

**A run** is one recording session:

1. `plugin:loaded` or the first event: `engine/hello`, then `provider-shape-session/create` with the
   provider and kind `http`.
2. `file:loaded`: keep each OAS operation's `id`, aliases, `method` and `path`, by source name.
3. `testcase:loaded`: keep each operation key's target.
4. `operation:started`: keep its description for the key.
5. `operation:invoked`: `provider-shape-session/observe` with
   - `operation`: the request's `method` as `equality`, and its `path` as the HTTP component's
     `http:path` with the OpenAPI template, prefixed by the base path;
   - `description`: the Drift operation's description, else the OAS operation's id;
   - no `states`: a Drift test sets up through hooks, not named states;
   - `parts`: the `response` part as the HTTP transport presents it (component-interfaces spec §5.9):
     `status`, `headers`, and `body` as its octets, tagged `base64`, labelled with the response's
     `Content-Type`. The engine decodes it; the plugin decodes nothing.
6. At the end of the run: `provider-shape-session/finalise` with no `write`, and the plugin writes the
   `provider-shape` the answer carries to the output directory, which Drift mounts at `/out`.

**What is recorded.** Every answer `operation:invoked` saw, whether the test then passed or failed. A
shape is what the provider produces (design 2.8 §2.1), and an answer that breaks the OpenAPI document
is exactly what a consumer might be sent. The status keeps it apart from the answers the document
promises. An operation that failed before it was invoked recorded nothing.

**Writing.** Natively, the engine's host writes and merges a shape under a lock (ADR 0086 decision 7).
The WASM engine has no host to write with, so the plugin writes the one run's shape, replacing the
file. Merging the shapes of several runs is the engine's job (`finalise`'s `write`) and is not
reimplemented in the plugin; §8 asks whether it is needed.

**Warnings.** `observe` answers `warnings` (octets a component claimed and could not decode); the
plugin logs them through Drift's logger, at `WARN`.

## 6. Messages (S4.10b)

The AsyncAPI half needs no engine change: the WASM engine already records the kind `message`. An
observation from an async operation's `operation:invoked` names its operation as ADR 0090 decision 1
says, by the values the message itself carries: a metadata member naming its channel or a body member
naming its kind, the selectors `janus derive` writes from the same AsyncAPI document (ADR 0091). A
request/reply operation's reply is a synchronous message, selected by its request. What each async
mode's result carries (inject, observe, inject-capture, request-reply) is to be read from the AsyncAPI
and transport plugins first; only what the provider produced is recorded, which in inject mode may be
nothing.

## 7. The demonstration

The sample provider (`samples/order-service`) already has the three things the exit needs: an
`openapi.yaml`, a v3 pact from a consumer, and the shape its own Rust tests recorded
(`shapes/order-service.provider-shape.json`), which `janus check` matches to the pact by operation and
blocks on the `CANCELLED` status and the empty list (S4.1).

S4.10a's exit: Drift test cases against the sample provider's `openapi.yaml`, exercising the answers
its own tests exercise, record a shape through the plugin, and `janus check` on that shape and the pact
reports the findings the checked-in shape reports. S4.10b's: the same for pact-js's message example's
provider, against the AsyncAPI document `cli/tests/derive.rs` derives from.

## 8. Open questions

1. **When a run ends.** No Drift event says so: `results:written` fires only with an output
   directory, and `plugin:unloaded` only when a plugin is reloaded. The recording needs one place to
   call `finalise`. Proposed: Drift fires `verification:finished`. Until it does, the plugin can keep
   its observations and replay them into a fresh session at each `testcase:finished`, rewriting the
   file each time.
2. **Merging runs.** If a provider's suite is several `drift-verify` invocations, are their shapes
   merged? Natively that is `finalise`'s `write`, with counts per source. In WASM it would need the
   engine's host capability for files, against the `/out` mount, which the artifact does not have.
3. **The base path.** Whether the plugin reads it from the operation plan's `servers`, as the verifier
   chooses a server URL when none is given, or it is only configured.
4. **Provenance.** `recorded` is right for a provider's own tests. A Drift suite is the provider's,
   but if a consumer team ran one it would be closer to `observed`. Is that a configuration member, or
   always `recorded`?
5. **Publishing.** Once a broker stores shapes (S4.3d), does the plugin publish the shape itself, or
   is that `janus publish`'s, after the run?
