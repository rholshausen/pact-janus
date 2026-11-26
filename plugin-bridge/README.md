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
  `matcher`, contributes `pact-plugin-<plugin>:compare-contents` when the plugin has a content matcher and
  `pact-plugin-<plugin>:generate-content` when it has a content generator, and, when the plugin has a
  transport, implements `transport` and contributes it as a kind of the entry's key (`grpc`), driven,
  with `pact-plugin-<plugin>:verdict`. It describes every catalogue entry under the `pact-plugin`
  capability.
- `matcher/apply` of `compare-contents` compares a whole body through the plugin's content matcher, with
  what a pact says about the body as its one argument (spec §9.3.1).
- `transport/send` on a drive instance of a plugin transport is the whole exchange (ADR 0111): the plugin
  prepares the request from the pact it is sent, calls the provider and compares its answer, and the reply
  is the plugin's verdict, which `matcher/apply` of `verdict` reads. A `serve` instance, a plugin
  transport's mock, is not served yet (S5.2f).
- What the plugin answers is read as the driver reads it for today's verifier, its failures included. A
  plugin the call cannot reach is the bridge's failure, not a mismatch: the call is answered
  `unavailable`, and the bridge exits, so the engine starts a new one.
- When its standard input ends, it shuts the plugin down and exits.

A subprocess component starts with an empty environment plus what it is granted, so grant it the plugin
directory (`PACT_PLUGIN_DIR`, or `HOME`/`USERPROFILE` for `~/.pact/plugins`), `PACT_DO_NOT_TRACK` to stop
the driver sending usage metrics, and `RUST_LOG` to see its logs on standard error.

`janus verify` and `janus explain` use it for a v1–v4 pact that names a plugin (staged plan S5.2d, ADR
0110): set `JANUS_PLUGIN_BRIDGE` to it, or put it beside `janus`, and the engine declares one for each
plugin the pact names. A body the plugin's content matcher claims is compared by `compare-contents`, and
a request body with generators is filled by `generate-content`, the action a content generator is
reached through. An interaction carried over the plugin's transport is driven through the bridge's kind
(S5.2e), reached at the `--transports` entry of its name, else the provider's host and port.

Not yet: plugin matchers are described in the handshake and not served (S5.2g), a plugin transport's mock
is not served (S5.2f), and the bridge is not released (S5.2h).

## Building and testing

```bash
cargo build
cargo test        # against tests/fixtures/plugins/lines-0.1.0, a Lua plugin: nothing is downloaded
JANUS_PACT_PLUGINS=/tmp/plugins cargo test   # and the tests on the pinned plugins: one kills the CSV plugin (Unix),
                                             # one drives the protobuf plugin's gRPC transport
```

[`examples/area_calculator.rs`](examples/area_calculator.rs) is the gRPC provider from pact-plugins'
`examples/gRPC/area_calculator`, its messages and service written as prost and tonic generate them, so it
needs no `protoc`: `cargo build --example area_calculator`, then `area_calculator [--port <port>] [--answers
areas|nothing|error]`. The gRPC test runs it, and S1.1's harness verifies the example's pact against it
(`corpora/verifier/plugins/grpc-area-calculator*`, with `JANUS_AREA_CALCULATOR` naming it).

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
