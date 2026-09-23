# 0021 — Distribute a component as its own OCI artifact type, described by its handshake, and fetch a pin by digest alone

- **Status**: accepted
- **Date**: 2026-09-23
- **Plan tasks**: 8.2
- **Evidence**: `engine/component-host/src/oci.rs` and its tests (`tests/oci.rs`, against an in-process
  registry that tampers on request and against `registry:2`); the round trips recorded below;
  [ADR 0013](0013-component-hosting-is-an-embedding-capability.md), which settled *that* components are
  digest-pinned OCI artifacts and left *which* artifact to this task

## Context

ADR 0013 and component-interfaces spec §10.3 settled the model: an out-of-tree WASM component is an OCI
artifact — "an artifact type identifying it as a Janus component, a config blob carrying its name and
version, and one layer carrying the `.wasm`" — resolved, digest-checked before instantiation, and cached
by digest so a pinned second run fetches nothing. Building it (plan task 8.2) needed four things the spec
left open, and each had a plausible other answer:

- **which media types.** The CNCF's Wasm OCI artifact layout already exists (config
  `application/vnd.wasm.config.v0+json`, layer `application/wasm`), and `wkg` publishes to it;
- **where the config's name and version come from**, when the spec also says the handshake is the only
  source of truth about a component and "there is no manifest to agree with it" (ADR 0012);
- **what a pin plus a tag means**: fetch the tag and compare, or fetch the digest and ignore the tag;
- **how a reference with neither a tag nor a digest resolves.**

## Decision

**1. A Janus component has its own artifact type.** `artifactType`
`application/vnd.pact.janus.component.v1`; config media type
`application/vnd.pact.janus.component.config.v1+json`, which is also the artifact type to a reader that
predates OCI 1.1; exactly one `application/wasm` layer. The layer's type is the registered one, so any
wasm-aware tool recognises the payload; the artifact type is ours, so a reference can be refused as "not a
Janus component" on its manifest alone, before a blob is fetched. The loader requires the config media
type and, when `artifactType` is present, requires it to agree.

**2. The config is written from the handshake, and checked against it on every load.** The config blob
carries `name` and `version` and nothing else — not interfaces, not contributions, which the handshake
alone declares. `janus component push` instantiates the component under no grants, asks it who it is,
and writes that. The loader compares the two after its own handshake and refuses a disagreement as
`artifact-mismatch`. So the config is a *cached answer*, not a second authority: something a registry,
a broker or a person can read without running the component, and which can never be wrong for long
enough to mislead them.

**3. A pin is fetched by digest, and the tag is never consulted.** With a declared digest, the manifest is
requested by that digest; the tag in the same reference is a label for people. The alternative — resolve
the tag, then compare — would make a pinned run depend on the registry being up and the tag not having
moved, which is the opposite of what a pin is for, and would contradict §10.3's "a second run of the
same pinned component fetches nothing". Every manifest and blob is hashed on receipt, from the registry
and from the cache; a registry's `Docker-Content-Digest` header is never trusted.

**4. No implied `latest`.** A reference names a tag or a digest, or it is refused. Docker's convention
assumes `latest`; a component resolved from a tag nobody wrote down is a component nobody chose, which
is exactly what "declared, never discovered" (spec §10.2) rules out.

Smaller calls, recorded in spec §10.3 rather than argued here: loopback registries are plain HTTP and
everything else HTTPS; credentials come from `JANUS_OCI_USERNAME`/`JANUS_OCI_PASSWORD` (a deliberate
exception to design 2.7 §7.1 — [finding 16](../phase-9-findings.md)); the cache is an OCI image layout's
`blobs/sha256/` directory, re-verified on read; `sha256` digests only.

## Alternatives considered

- **The CNCF Wasm OCI artifact layout, unchanged.** Rejected because its config
  (`application/vnd.wasm.config.v0+json`) describes a WASM *module or component* — architecture, OS,
  layer digests, the component's WIT exports — and says nothing about which Janus component it is; and
  because "any WebAssembly component" is the wrong thing to accept by type. A generic WASM artifact
  would be accepted by media type and then refused by the handshake, which is a worse place to learn it
  — after a fetch and a compile, rather than on the manifest. The cost is interop with `wkg`, which is
  [finding 17](../phase-9-findings.md): a real one, but a door that stays open (accepting the CNCF config
  as well is additive).
- **A config with interfaces and contributions**, so a broker could index what a component contributes
  without running it. Rejected as a second source of truth: ADR 0012's point is that the handshake is
  the only description of a component's contributions, and a copy that only *usually* agrees is worse
  than none. Name and version pass the bar because they are checked on every load.
- **The config as a free-form, publisher-written blob**, as `oras push --config` makes easy. Kept
  *possible* — `oras` produced a loadable artifact in the evidence below — but not trusted: the loader's
  check is what makes a hand-written config safe.
- **Resolve the tag even when pinned, and fail if it moved.** Rejected in decision 3: it turns a pin into
  "a pin, as long as the registry is up and nobody retagged" — and "the tag moved" is information for
  whoever maintains the pin, which `janus component pull` gives them when they ask.
- **A general OCI client crate** (`oci-client`). Rejected as the wrong size: it is async, needs a
  runtime a synchronous `load` would have to block on, and covers a surface (image indexes, platform
  resolution, chunked uploads) a component never uses. The client here is the handful of distribution-API
  calls a push and a pull need, on `ureq`, which the tree already had.

## Consequences

Easier: a component's identity is checkable at three points that all agree — the manifest says it is a
Janus component, the config says which one, the handshake confirms it — and a pinned project runs
offline after its first fetch. Publishing needs no Janus tooling: `oras push --artifact-type
application/vnd.pact.janus.component.v1 --config config.json:application/vnd.pact.janus.component.config.v1+json
component.wasm:application/wasm` produces an artifact `janus component pull` accepts.

Harder: authors who publish with `wkg` must publish again, differently (finding 17). And the config
cannot grow into a capability listing without either a check against the handshake for each new member
or a superseding ADR.

Evidence, 2026-09-23: pushed to `registry:2` and pulled back by tag and by digest, with the registry's
digest, `oras manifest fetch --descriptor`'s and ours identical; an `oras`-published artifact pulled and
handshaken; against `ghcr.io`, the anonymous token dance worked, and a real image index was refused as
`not-a-component` — after the loader learnt to *accept* the index media type, because ghcr answers an
`Accept` that omits it with a 404 for a reference that exists.

**Tripwire.** If components start shipping from ecosystems that publish only the CNCF layout — so that
"publish again as a Janus artifact" becomes the usual first step for a third-party author — accept that
layout too, identifying a Janus component by its handshake alone, and supersede decision 1.
