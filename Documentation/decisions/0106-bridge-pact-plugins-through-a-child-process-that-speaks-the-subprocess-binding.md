# 0106 — Bridge pact-plugins through a child process that speaks the subprocess binding

- **Status**: accepted
- **Date**: 2026-11-23
- **Plan tasks**: S5.2, split here into S5.2a–S5.2h; S5.5, whose loader moves into S5.2b; S5.12
  (a native protobuf content component, and S4.2's protobuf half)
- **Evidence**: pact-plugins' `proto/plugin.proto` and `plugin_v2.proto`, and `pact-plugin-driver` 1.2.7
  (`drivers/rust/driver`), the driver today's Rust verifier and `pact_ffi` load plugins with; the pacts in
  pact-plugins' `examples/` (CSV, JWT, credit card, protobuf, gRPC, JSON-RPC, Avro); spike 8.3's
  [findings](../../spikes/8.3-subprocess-transport/FINDINGS.md) and its loader, deleted in S0.12
  (`b8d5010` is the last commit with it); [phase-9 findings](../prototype/phase-9-findings.md) 18 and 20;
  component-interfaces spec §9.3; [ADR 0045](0045-refuse-an-interaction-that-needs-a-pact-plugin-by-name-and-verify-the-rest.md)

## Context

S5.2 is the bridge for today's pact-plugins, so that no one who verifies a protobuf, gRPC, CSV or Avro
pact today loses it by moving to Janus. Until it exists, ADR 0045 refuses each interaction that needs a
plugin, by name, and verifies the rest. The plan sized S5.2 as large and said only that "a shim process
speaking both wires is the smaller route" (spec §9.3).

A plugin is a process that serves the `PactPlugin` gRPC service. A driver finds it by its manifest under
`PACT_PLUGIN_DIR` or `~/.pact/plugins`, installing it from the plugin index when it is missing, starts
it, reads its port and bearer key from its standard output, and calls it over gRPC. What a plugin
provides is a catalogue of entries:

- a content matcher, which compares an actual body with an expected one under the pact's rules
  (`CompareContents`);
- a content generator (`GenerateContent`);
- a transport, which serves a mock (`StartMockServer`, `GetMockServerResults`) and verifies a provider
  itself (`PrepareInteractionForVerification`, `VerifyInteraction`);
- matchers (version 2's `MatchField`);
- interaction configuration, which turns what a consumer test wrote into a body, rules and plugin data
  for the pact (`ConfigureInteraction`).

Three things about that interface decide how far a bridge can go:

- **No plugin decodes.** A content matcher takes the octets and does the comparing. A Janus content
  component turns octets into a document, which the engine's plan compares. So a plugin cannot be a Janus
  content component. The most it can be is an action that compares a whole slot.
- **A plugin transport verifies its own exchange.** `VerifyInteraction` sends the request, reads the
  answer and returns a verdict with mismatches. A Janus transport only moves parts, and the engine
  decides.
- **The driver is a dependency tree of its own:** tonic, prost, tokio, a vendored Lua, an HTTP client,
  zip and tar for installing, `sysinfo`. None of it is in the Janus tree.

## Decision

1. **The bridge is a child process, `janus-plugin-bridge`, in a Cargo workspace of its own**
   (`plugin-bridge/`, excluded from ours as `benchmarks/` and `third-party/` are, with its own lockfile).
   gRPC, protobuf and the driver never enter the tree of any Janus binary, nor `Cargo.lock`. The engine
   gains only the subprocess loader, which spec §9.3 already specifies.
2. **It is a component reached through the subprocess binding.** It speaks component frames, framed with
   `Content-Length`, over its standard input and output to the engine. On the other side it is a plugin
   driver, built on `pact-plugin-driver`. So plugin discovery, manifests, process start, Lua plugins,
   plugin interface versions 1 and 2, and installing a missing plugin from the index behave as they do
   for today's verifier, because they are the same code. Like today's driver, it installs a missing plugin
   without being asked.
3. **One bridge process per plugin, and each is a Janus component of its own,** named
   `pact-plugin-<name>` (`pact-plugin-protobuf`), and declared with the plugin's name and version as its
   arguments. Its handshake is the plugin's catalogue, translated. A per-plugin process means a hung
   plugin call ends one plugin's instances, never another's (finding 18). Each plugin's version is pinned
   by its own declaration, as the pact pins it, and its contributions are in its own namespace (spec
   §2.4). A plugin is already a process, so the bridge doubles the processes per plugin, and nothing more.
4. **What each catalogue entry becomes:**
   - a **content matcher** becomes a matcher action, `<component>:compare-contents`, applied to a whole
     slot's octets, with the expected body, the pact's rules for it and the plugin's configuration as its
     arguments. The legacy compiler compiles a v1–v4 body whose content type a plugin claims to that
     action, in place of its own walk of the body.
   - a **transport** becomes a Janus transport kind (`grpc`), driven through `VerifyInteraction` and
     served through `StartMockServer`.
   - a **content generator** fills a whole body before it is sent or served. This is not S5.9's
     generators, which fill one value.
   - **matchers** and **interaction configuration** are bridged last (S5.2g, S5.2f).
   The bridge contributes nothing for `interaction` entries: a v1–v4 interaction's kind is the legacy
   reader's to know.
5. **A pact that names its plugins declares them.** A component is declared, never discovered (spec
   §10.2), and a v4 pact's `metadata.plugins` names each plugin with its version, which is what today's
   verifier loads. So `janus verify` adds a bridge declaration for each plugin a pact names and no
   configuration declares, as it adds a `produce-message` hook (ADR 0046). It finds the bridge beside the
   `janus` executable or where `JANUS_PLUGIN_BRIDGE` names it. An engine with no bridge still answers with
   ADR 0045's refusal, whose message now says where the bridge would be looked for.
6. **The bridge is for today's pacts and today's DSLs.** It serves v1–v4 pacts in verification, and the
   compatibility facades' `usingPlugin` on the consumer side. A Janus contract that has to carry a plugin
   interaction, which the facades need, gets a form for one in S5.2f. Anything new is meant to be a WASM
   component, which needs no bridge.
7. **S5.2 is eight items:**
   - **S5.2a** this decision;
   - **S5.2b** the subprocess loader, moved here from S5.5, deciding findings 18 and 20;
   - **S5.2c** the bridge and its handshake;
   - **S5.2d** verifying a v1–v4 pact's plugin content;
   - **S5.2e** plugin transports in verification;
   - **S5.2f** the consumer side and a contract form for a plugin interaction;
   - **S5.2g** matcher-only plugins and version 2's field operations;
   - **S5.2h** releasing the bridge.
   Stage 5's exit, "a protobuf plugin runs through the bridge", is met at S5.2d, with a protobuf message
   pact. S4.2's protobuf half (`janus derive` from a `.proto`) is not the bridge's, because a plugin cannot
   decode: it moves to S5.12, a native protobuf content component.

## Alternatives considered

- **A bridge inside the engine, linking the driver or a gRPC client.** It is one process fewer, but the
  driver's dependency tree would be in every Janus binary, the WASM build would have to keep it out, and a
  hung plugin call would hang the engine instead of a process the loader can kill.
- **One bridge process for every plugin a run needs.** It is fewer processes, but one handshake would
  have to name every plugin's contributions under one namespace, and a hung call in one plugin would end
  all of them (finding 18).
- **A second gRPC pipe in the component interfaces, so a plugin is loaded as it is.** Spec §9.3 rejected
  gRPC as a pipe for good reasons that still hold, and the frames differ completely either way. The bridge
  would still have to translate.
- **A native protobuf component in place of a bridge.** That is S5.12, and it is the better long-term
  answer for protobuf. But it serves no other plugin, and does not verify a pact as today's protobuf
  plugin does, so it cannot be the drop-in.
- **Declared only, with no declaration added by `janus verify`.** It is purer, but every provider would
  have to write configuration that its pact already says, which today's verifier never asked for.

## Consequences

- The engine's own change is the subprocess loader, which the specification already describes and spike
  8.3 proved. The bridge is a separate program with its own dependencies, its own lockfile and its own
  release (S5.2h).
- Comparing a plugin body is one action, so the plugin's mismatches have to come back through one plan
  node. The plan grammar has one error per node, so S5.2d decides how an action reports several located
  mismatches.
- A plugin transport's verdict is the plugin's, not the engine's. S5.2e decides how it reaches a
  verification result without the engine comparing the same parts a second time.
- A CI run that verifies plugin pacts needs plugin binaries. They are pinned and checked by digest, as
  `pact-verifier.sh` pins today's verifier, never installed from the index during a test.
- **Tripwire**: a plugin that a bridge cannot serve without the engine learning something about that
  plugin, outside the legacy layer. That would mean the line between the kernel and the bridge is in the
  wrong place.
