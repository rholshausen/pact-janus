# 0098 — Accept a component published in the CNCF Wasm layout, recognised by the pipe its config says it exports

- **Status**: accepted; supersedes [ADR 0021](0021-a-component-artifact-is-its-own-type-and-a-pin-is-fetched-by-digest.md) decision 1
- **Date**: 2026-11-19
- **Plan tasks**: S5.1b
- **Evidence**: [Phase 9 finding 17](../prototype/phase-9-findings.md#17-a-component-published-by-wkg-is-not-a-janus-component),
  the [CNCF Wasm OCI artifact layout](https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/),
  `wkg` 0.16.2's `oci push` of `third-party/janus-csv` (below), `engine/component-host/tests/oci.rs`

## Context

ADR 0021 gave a Janus component an artifact type of its own and refused anything else as
`not-a-component`, on its manifest, before a blob was fetched. The WebAssembly ecosystem's own tool,
`wkg oci push`, publishes the CNCF Wasm OCI artifact layout instead, so a component an author published
with it would not load (finding 17). ADR 0021 left a tripwire: accept that layout too if publishing
again as a Janus artifact becomes the usual first step for a third-party author. S5.1 makes components
generally available, which is when that step starts to cost authors.

ADR 0021 rejected the CNCF layout because a generic WASM artifact "would be accepted by media type and
then refused by the handshake", after a fetch and a compile. That is not so. The CNCF layout says a
component's config MUST have a `component` member, and that member lists the component's WIT exports.
Pushing the CSV component with `wkg` 0.16.2 wrote:

```json
{ "created": "2026-11-19T00:12:33.271264184Z", "author": null, "architecture": "wasm", "os": "wasip2",
  "layerDigests": ["sha256:d4705c9a…"],
  "component": { "exports": ["pact:janus-component/pipe@1.0.0"],
                 "imports": ["wasi:io/poll@0.2.12", "…"], "target": null } }
```

with config media type `application/vnd.wasm.config.v0+json`, one `application/wasm` layer, and no
`artifactType`. The export is the pipe's name exactly as the kernel spells it.

## Decision

1. **The loader accepts two layouts.** Janus's own (ADR 0021) and the CNCF Wasm OCI artifact layout,
   whose config media type is `application/vnd.wasm.config.v0+json`. Both have exactly one
   `application/wasm` layer, and anything else is `not-a-component` as before.
2. **A CNCF artifact is a Janus component's when its config says it exports the pipe.** The config's
   `component.exports` must name `pact:janus-component/pipe@1.0.0`, in any spelling it has had (the
   persisted `COMPONENT_INTERFACE`). That is checked on the config, before the layer is fetched, so an
   artifact that is not a Janus component is still refused before a byte of it is fetched or compiled.
   It costs one small blob more than ADR 0021's check on the manifest. A config with no `component` is
   a module and is refused as one, and a component that exports something else is refused naming what
   it exports. The CNCF layout defines no `artifactType`, so for it one is not read.
3. **The handshake alone says which component a CNCF artifact is.** Its config has no name or version,
   so there is nothing to check against the handshake, and `artifact-mismatch` cannot arise. The
   handshake was always the source of truth (ADR 0012). An annotation such as
   `org.opencontainers.image.version` is not read: it labels a package, which may not be the
   component's own version, and checking it would refuse artifacts nobody mislabelled.
4. **`janus component push` still writes Janus's layout.** Its config says which component an artifact
   is without running it, which a registry, a broker or a person can read, and its push is
   reproducible. `janus component pull` takes the component's name and version from the handshake for
   either layout, so the declaration it prints is right for a `wkg` artifact too.
5. **Pinning and integrity do not change.** The pin is the manifest's digest, and every blob is hashed
   on arrival and when read from the cache. `wkg` writes a `created` timestamp into the config, so
   each push of the same bytes is a new manifest digest, where `janus component push` gives one digest
   however often it pushes. An author who pins by digest repins after every `wkg` push.

## Alternatives considered

- **Keep one artifact type and document the `oras` recipe** (finding 17's option A). Rejected because it
  makes every `wkg` user publish twice, at the point S5.1 invites them in.
- **Accept any WASM artifact and let the handshake decide** (option B as finding 17 stated it). Rejected
  as unnecessary: the config already says what the component exports, so the early refusal stays.
- **Ask upstream for a Janus annotation in the CNCF config** (option C). Not needed, since the exports
  already identify the component, and it would make the loader depend on a convention only Janus
  authors follow.
- **Read the imports too, and refuse at the config a component whose sockets imports exceed its
  grants.** Not done: the config's imports are the publisher's claim, and the loader already checks
  the component's real imports at load (component-interfaces spec §9.2). It would save a fetch only for
  a configuration that is wrong anyway.

## Consequences

- An author publishes with `wkg oci push`, `janus component push` or `oras`, and each loads.
- `not-a-component` for a CNCF artifact names what it exports, which tells an author who pushed the
  wrong file what they pushed.
- A broker or registry UI that wants a component's name without running it gets it only from Janus's
  layout. The CNCF layout's config carries the exports, which say it is a Janus component, but not
  which one.
- `tests/oci.rs` pins the layout `wkg` writes, and pushes with `wkg` itself when `JANUS_WKG` names one,
  which CI's Linux job installs.
- Tripwire: if the CNCF layout gains a name and version for a component, check them against the
  handshake as ADR 0021 decision 2 checks Janus's config.
