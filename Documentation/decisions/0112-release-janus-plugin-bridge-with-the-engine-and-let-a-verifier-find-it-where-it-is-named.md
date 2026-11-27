# 0112 — Release janus-plugin-bridge with the engine, and let a verifier find it where it is named

- **Status**: accepted
- **Date**: 2026-11-27
- **Plan tasks**: S5.2h (releasing the bridge); amends
  [ADR 0030](0030-a-release-carries-the-sdk-gates-and-the-executable-specification.md)'s
  list of what a release carries; refines
  [ADR 0110](0110-a-v1-v4-plugin-body-compiles-to-its-bridges-compare-contents-and-one-node-reports-each-mismatch-it-found.md)
  decision 6 for the SDKs
- **Evidence**: [ADR 0106](0106-bridge-pact-plugins-through-a-child-process-that-speaks-the-subprocess-binding.md)
  decision 1 (the bridge's own workspace); [ADR 0060](0060-release-each-sdk-as-files-on-a-github-release-of-its-own-and-fetch-its-pinned-engine.md)
  (a package released on its own); [ADR 0104](0104-the-component-interface-is-generally-available-for-content-and-matcher-components-over-wasm.md)
  (what is generally available, and that the subprocess binding is not); `.github/workflows/release.yml`;
  `cli/src/plugin_bridge.rs`; a static `x86_64-unknown-linux-musl` build of the bridge, 15 MB stripped,
  needing no shared library

## Context

S5.2d and S5.2e verify a v1–v4 pact's plugin content and plugin transports through
`janus-plugin-bridge`, but only for someone who builds `plugin-bridge/` from source, with a Rust
toolchain. Nothing ships the bridge, so the users S5.2 exists for, the ones with a protobuf or gRPC pact,
cannot use it yet. The plan left one question open: whether the bridge is an asset of the engine's release
(ADR 0030's names) or a release of its own, as ADR 0060 made the SDKs'.

The bridge is a separate Cargo workspace on purpose (ADR 0106 decision 1): pact-plugin-driver's gRPC,
protobuf and Lua stay out of every Janus binary. That is a reason to *build* it apart, not a reason to
*release* it apart. What decides the release question is how the bridge and the engine depend on each other.
They speak the subprocess binding, which ADR 0104 left out of what is generally available, and the bridge
reaches parts of it nothing else does: a driven transport kind, the `pact-plugin` handshake capability,
`compare-contents`, `generate-content` and `verdict`. Three planned items change both sides of that
conversation: S5.2f (a plugin transport's mock), S5.2g (plugin matchers and field operations) and S5.2i (a
second call between preparing a request and verifying it). If the two were versioned apart, each of those
items would need rules for an old bridge with a new engine and for a new bridge with an old engine.

The SDKs need an answer too. ADR 0110 decision 6 has `janus verify` look for the bridge where
`JANUS_PLUGIN_BRIDGE` names it, then beside `janus`, and says an SDK's verifier *can* set
`options.plugin-bridge`. Neither SDK does, so a provider test verifies a plugin pact only if it writes the
whole declaration into its options by hand.

## Decision

1. **The bridge is an asset of the engine's release.** Each `v<version>` release carries
   `janus-plugin-bridge-<triple>` (`.exe` on Windows) for the five triples `janus-engine` is built for.
   Each is built from the commit being released, in the same matrix job as `janus` and `janus-engine`
   (with `--manifest-path plugin-bridge/Cargo.toml`, from its own lockfile, `--locked`), checked for
   static linking by the same step, run before it ships (`--version` must print the release's version),
   listed in `SHA256SUMS` and attested. ADR 0030's list therefore becomes: `janus-engine-<triple>`,
   `janus-<triple>`, `janus-sdk-gates-<triple>` and `janus-plugin-bridge-<triple>` for the five triples;
   `janus-engine.wasm`; `janus-spec.tar.gz`; and `SHA256SUMS`. That is 23 files, and the asset name
   joins the names a change to which is a superseding ADR.
2. **Its version is the engine's.** The bridge stays a workspace of its own, but its package version is the
   one in `[workspace.package]`. `knope prepare-release` writes both, and refreshes the bridge's lockfile
   as it refreshes the benchmark harness's. CI's `checks` job fails a bridge whose version differs from the
   engine's. A bridge is built and tested for the engine from its own release, and mixing versions is
   not supported. Until S5.5 opens the subprocess binding, the engine and the bridge may change it between
   them, in one commit. The engine does not check the bridge's version: what it needs from a bridge, it
   reads from the bridge's handshake, and it refuses by name an interaction an older bridge cannot verify
   (ADR 0110 decision 5).
3. **A change to the bridge ships in the engine's release.** The bridge's sources, its manifest and its
   lockfile join the engine's paths in CI's `Change file` workflow. `pact-plugins.json` does not: it pins
   what the tests run against, not what ships.
4. **`janus` finds the bridge where it did** (ADR 0110 decision 6): where `JANUS_PLUGIN_BRIDGE` names it,
   else beside the `janus` executable. The release notes say to download the bridge for the platform into
   the directory `janus` is in. There is no lookup on the `PATH`, for ADR 0060's reason: a
   `janus-plugin-bridge` on the `PATH` is whatever was installed last, perhaps from another release.
5. **An SDK's verifier finds the bridge by its own option, then by `JANUS_PLUGIN_BRIDGE`.** Given either,
   the verifier sends `options.plugin-bridge` as `janus verify` sends it. That is a `subprocess` source
   naming the program, and the grants and deadline `janus verify` gives, which the behavioural
   specification states. A program named by a relative path is resolved against the working directory,
   as a `file` component's reference is. A `plugin-bridge` the run's own options already give is sent as
   given, and wins. Nothing is checked or fetched: whether the bridge starts is the engine's answer, when
   a pact needs it. `JANUS_PLUGIN_BRIDGE` is read as `JANUS_ENGINE` is, as where a program is and not a
   run option. The behavioural specification's rule that the SDK reads no environment variable for any
   option still holds for everything the run sends.
6. **The SDK does not fetch the bridge.** It fetches its pinned engine on first use (ADR 0060 decision 6),
   but whether a run needs a bridge is known only once a pact is read, which for a broker source is after
   the run has started. Fetching first would download 15 MB or more into every provider test for the few
   that name a plugin. Because the bridge is in the engine's release, the engine pin can carry its digests
   on the day an SDK should fetch it (the tripwire below).
7. **A release proves the bridge before it is published.** `release.yml`'s `plugin-bridge` job runs on
   each operating system an engine is resolved on. It takes the release's own `janus` and
   `janus-plugin-bridge`, side by side, and the pinned protobuf plugin, installed by
   `.github/scripts/pact-plugins.sh`. A provider written in JavaScript answers the
   `corpora/verifier/plugins/protobuf-message` fixture's routes. With no Rust toolchain reachable,
   `janus verify` passes that fixture's pact and fails its mismatch twin's. `publish` needs the job, and
   every rehearsal runs it. This is S5.2h's exit, run on every release.

## Alternatives considered

- **A release of its own** (`plugin-bridge/v<version>`, a fourth knope package and its own workflow, as
  ADR 0060 made the SDKs'). It would release on the driver's cadence rather than the engine's. But it puts
  a version boundary across a binding that is not stable, and each of S5.2f, S5.2g and S5.2i would carry
  compatibility rules in both directions. It would also give a user two versions to match by hand, where
  one release has everything that works together.
- **The bridge inside `janus`** (a feature, or a subcommand). This is what ADR 0106 decision 1 rejected:
  the driver's gRPC, protobuf and Lua would enter every Janus binary, including the engine every SDK runs.
- **Look on the `PATH`.** This is convenient for someone who installs tools that way. But a stale bridge
  from an earlier release would be used silently, which is the mixed-version case decision 2 does not
  support. `JANUS_PLUGIN_BRIDGE` serves anyone who wants it somewhere else.
- **The SDK fetches the bridge on first use, as it fetches the engine.** That means a download in every
  provider test for a need few of them have (decision 6). Fetching only when the engine asks would need
  a host callback the protocol does not have, for a convenience no pilot has asked for yet.
- **The engine checks that the bridge's version is its own.** That would turn every patch-level mismatch
  into a refusal, where the handshake already says precisely what a bridge can do. It would also give the
  engine a second, coarser way to answer the same question.

## Consequences

Easier: a provider with a protobuf or gRPC pact downloads two files from one release, puts them in one
directory, and runs `janus verify`, with no toolchain. An SDK user sets `JANUS_PLUGIN_BRIDGE` and changes
nothing in their test. Every release proves that its bridge verifies a protobuf pact on Linux, macOS and
Windows before it is published. The engine and the bridge keep changing their binding together, through
S5.2f, S5.2g and S5.2i, with no compatibility matrix.

Harder: a release builds the driver's dependency tree five more times, a few minutes each from cold, in
jobs that already build the engine. A bridge-only fix, such as a driver bump, is an engine release. The
release is 23 files. The bridge's lockfile is one more that `knope prepare-release` keeps current.

Committed to: the asset name `janus-plugin-bridge-<triple>`, the rule that a bridge's version is its
engine's, and `JANUS_PLUGIN_BRIDGE` as the SDKs' way to find one, beside each SDK's own option.

Tripwires:

- **A pilot finds installing the bridge by hand a burden**, or runs where they cannot set a variable: the
  engine pin gains the bridge's digests (`tools/engine-pin` reads them from the same `SHA256SUMS`), and an
  SDK fetches the bridge the first time a run needs one. That needs either a host callback or the
  SDK reading the pacts' metadata, and the choice between those is an ADR.
- **S5.5 makes the subprocess binding generally available**, or the bridge moves to pact-plugins'
  maintainers: the binding then has compatibility rules of its own, the bridge can be released on its own,
  and this ADR is superseded.
