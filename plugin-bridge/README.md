# janus-plugin-bridge

One of today's [pact-plugins](https://github.com/pact-foundation/pact-plugins), served to the Janus engine
as a component over the subprocess binding (component-interfaces spec
[§9.3.1](../Documentation/specs/component-interfaces/spec.md#931-a-pact-plugins-bridge),
[ADR 0106](../Documentation/decisions/0106-bridge-pact-plugins-through-a-child-process-that-speaks-the-subprocess-binding.md),
[ADR 0109](../Documentation/decisions/0109-a-bridged-plugins-content-matcher-is-one-compare-contents-action-and-its-catalogue-a-handshake-capability.md)).

It is a Cargo workspace of its own, excluded from the repository's, because it is built on
`pact-plugin-driver`, the driver today's verifier loads plugins with, and the driver's gRPC, protobuf and
embedded Lua must not enter the tree of any Janus binary. So plugin discovery, manifests, starting a
plugin, Lua plugins, plugin interface versions 1 and 2, and installing a missing plugin from the plugin
index behave as they do for today's verifier.

## What it is to the engine

`janus-plugin-bridge <plugin> <version>` is the component `pact-plugin-<plugin>`, one process per plugin,
declared like any `subprocess` component:

```yaml
components:
  - name: pact-plugin-protobuf
    source: { kind: subprocess, reference: janus-plugin-bridge, args: [protobuf, 0.8.1] }
    grants: { env: [PACT_PLUGIN_DIR, PACT_DO_NOT_TRACK] }
    limits: { deadline-ms: 30000 }
```

- At `component/hello` it loads the plugin and answers with its catalogue translated: it implements
  `matcher`, contributes `pact-plugin-<plugin>:compare-contents` when the plugin has a content matcher,
  and describes every catalogue entry under the `pact-plugin` capability.
- `matcher/apply` of `compare-contents` compares a whole body through the plugin's content matcher, with
  what a pact says about the body as its one argument (spec §9.3.1).
- What the plugin answers is read as the driver reads it for today's verifier, its failures included. A
  plugin the call cannot reach is the bridge's failure, not a mismatch: the call is answered
  `unavailable`, and the bridge exits, so the engine starts a new one.
- When its standard input ends, it shuts the plugin down and exits.

A subprocess component starts with an empty environment plus what it is granted, so grant it the plugin
directory (`PACT_PLUGIN_DIR`, or `HOME`/`USERPROFILE` for `~/.pact/plugins`), `PACT_DO_NOT_TRACK` to stop
the driver sending usage metrics, and `RUST_LOG` to see its logs on standard error.

Not yet: no engine declares a bridge for a pact, or compiles a pact's body to `compare-contents` (staged
plan S5.2d); plugin transports, content generators and matchers are described in the handshake and not
served (S5.2e–S5.2g); and the bridge is not released (S5.2h).

## Building and testing

```bash
cargo build
cargo test        # against tests/fixtures/plugins/lines-0.1.0, a Lua plugin: nothing is downloaded
JANUS_PACT_PLUGINS=/tmp/plugins cargo test   # and the test that kills a plugin process (Unix), on the pinned CSV plugin
```

`conformance/` holds the bridge's conformance cases (component-interfaces spec §9.4), one directory per
plugin, and `conformance/plugins.janus.yaml` declares a bridge for each. The plugins they are written
against are pinned by digest in [`pact-plugins.json`](pact-plugins.json), and installed with
`.github/scripts/pact-plugins.sh`, which refuses any other bytes. From the repository's root:

```bash
.github/scripts/pact-plugins.sh /tmp/plugins
cp -r plugin-bridge/tests/fixtures/plugins/lines-0.1.0 /tmp/plugins/
cargo build -p pact_janus_cli --bin janus
export JANUS_PLUGIN_BRIDGE=$PWD/plugin-bridge/target/debug/janus-plugin-bridge PACT_PLUGIN_DIR=/tmp/plugins PACT_DO_NOT_TRACK=true
for plugin in lines csv protobuf; do
  target/debug/janus component check --config plugin-bridge/conformance/plugins.janus.yaml \
    pact-plugin-$plugin plugin-bridge/conformance/$plugin
done
```

CI's `rust` job does this on Linux, macOS and Windows.
