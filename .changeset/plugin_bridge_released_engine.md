---
engine: minor
---

# janus-plugin-bridge ships in every release

Each release now carries `janus-plugin-bridge-<triple>` for the five triples `janus-engine` is built for
(ADR 0112). The files are statically linked, listed in `SHA256SUMS` and attested like the rest, so a
release is 23 files. The bridge's version is the engine's, and it is meant to be used with the engine and
`janus` of its own release. Mixing versions is not supported while the subprocess binding is not
generally available. To verify a pact that names a pact plugin, put the bridge beside `janus`, named
`janus-plugin-bridge` (`.exe` on Windows), or set `JANUS_PLUGIN_BRIDGE` to it. Before a release is
published, its own `janus` and bridge verify a protobuf pact on Linux, macOS and Windows with no Rust
toolchain.

`janus-spec.tar.gz` gains two provider conformance cases, `provider/plugin-bridge-declared` and
`provider/plugin-bridge-as-given`, and the behavioural specification's `verify` now says how an SDK's
verifier declares the bridge. An SDK that runs its gates against this release must pass them.
