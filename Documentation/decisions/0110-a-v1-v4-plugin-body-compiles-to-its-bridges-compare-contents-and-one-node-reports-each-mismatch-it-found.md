# 0110 — A v1–v4 plugin body compiles to its bridge's compare-contents, and one node reports each mismatch it found

- **Status**: accepted
- **Date**: 2026-11-26
- **Plan tasks**: S5.2d (verifying a v1–v4 pact's plugin content); S5.2e–S5.2g, which bridge the rest of a
  plugin's catalogue; S1.8, whose refusal this narrows
- **Evidence**: [ADR 0045](0045-refuse-an-interaction-that-needs-a-pact-plugin-by-name-and-verify-the-rest.md),
  [ADR 0106](0106-bridge-pact-plugins-through-a-child-process-that-speaks-the-subprocess-binding.md),
  [ADR 0109](0109-a-bridged-plugins-content-matcher-is-one-compare-contents-action-and-its-catalogue-a-handshake-capability.md);
  plan-grammar spec §2.3 and §3.2; engine-protocol spec's `Mismatch`; `pact_matching` 1.x's
  `match_response`, `match_message` and `match_sync_message_response` (every body a verifier compares is
  compared with `DiffConfig::AllowUnexpectedKeys`), `plugin_support.rs` (the configuration a plugin is
  handed) and `generators/mod.rs` (a plugin body is generated only when it has generators); the CSV and
  protobuf example pacts in pact-plugins' `examples/`

## Context

S5.2c built `janus-plugin-bridge`, which serves one of today's pact-plugins as the component
`pact-plugin-<name>`: its content matcher is the action `<component>:compare-contents`, and its handshake's
`pact-plugin` capability says which content types the plugin claims. Nothing uses it yet. ADR 0045 still
refuses, by name and before anything runs, every v1–v4 interaction that needs a plugin. S5.2d makes the
ones whose bodies a bridged content matcher compares verifiable, and leaves four questions:

- **One node, several mismatches.** A plugin compares a whole body and answers every difference it found,
  each located in its own words (`row: 1, column: 2`, `$.square`). The plan grammar gives a node one
  result, and an `error` one message and one path (§2.3). The bridge sends them all in its result's
  `mismatches` member, and the engine ignores it, so a run reports one mismatch with every message joined
  into it. Today's verifier lists each.
- **Which bodies go to a plugin,** decided where. The kernel may not know any plugin. The legacy layer may
  know v1–v4 pacts, and a pact says which plugin built an interaction; the bridge's handshake says which
  content types it compares.
- **Who declares the bridge.** A component is declared, never discovered (component-interfaces spec
  §10.2), and ADR 0106 decision 5 has `janus verify` declare one for each plugin a pact names. But a pact
  fetched from a broker or by URL is fetched inside the run, after the run's components were loaded, so
  the CLI cannot see what it names; and an SDK's verifier passes pacts too.
- **What a request body needs.** Today's verifier passes a plugin request body through the plugin's
  `GenerateContent` when, and only when, the body has generators.

## Decision

1. **An `error` result may carry the mismatches it found.** Plan-grammar §2.3's `error` result gains an
   optional `mismatches` member: a list of what one node found, each a `message` and an optional `at`,
   where the node's own words locate it inside the value under test. A component action's result
   document carries it (component-interfaces spec §7.2), and so may a core action's when it ever needs to.
   The node keeps its one `message`, which names them all, and its one `path`, the absolute path of the
   value under test. `at` is not a path in the grammar's sense: a plugin's location is in a document only
   the plugin decodes, and the engine passes it on as written.
2. **A run reports each of them as a mismatch of its own.** A node whose error carries `mismatches`
   contributes one entry per mismatch to a result's `mismatches` and to `explain`, each with the node's
   `path` and `action`, the entry's `message`, and its `at`, a new optional member of the engine
   protocol's `Mismatch`. So a run counts what today's verifier counts. The executed form (§3.2) is
   unchanged: the node's result is `ERROR(<message>)`, the message that names them all. A text report
   writes an entry `<path> (<at>): <message>`. The bridge's entries name their location `at`, not `path`
   as ADR 0109 first had it, since a result's `path` is an absolute path.
3. **The legacy compiler compiles a plugin body to the bridge's action.** In a v1–v4 interaction that
   needs plugin *P* (ADR 0045 decision 1), a body the verifier compares (a response body, or a message's
   contents) whose content type the run's `pact-plugin-P` component claims, through a `content-matcher`
   entry of its `pact-plugin` capability that names an `action`, compiles to that action applied to the
   body's slot, in place of the compiler's own walk of the body. Its argument is ADR 0109 decision 5's:
   the body's `content-type` as the pact records it, the `expected` body, the pact's `body` rules in the
   v4 form, `allow-unexpected-keys: true` as `pact_matching` compares every body a verifier compares,
   and `plugin-configuration`: the interaction's configuration for *P* (its `response` member's, when the
   pact keeps it by part) and the configuration the pact's metadata records for *P*. The header checks
   stay the compiler's, `Content-Type` among them (ADR 0043).
4. **The kernel carries a component's capabilities, and only the legacy layer reads this one.** A loaded
   component keeps its handshake's `capabilities` as the handshake gave them, and the legacy layer reads
   `pact-plugin` from them (S1.11: the kernel's one pact-shaped module). Nothing outside it names a pact
   plugin.
5. **A plugin interaction is verified when its bridge can verify all of it, and refused by name
   otherwise.** One whose every plugin has a loaded bridge, whose bodies of a plugin's types each have a
   content matcher, which is carried over no plugin transport, and which needs no plugin rule (a
   matcher-only plugin's), is verified. The rest are refused before anything runs for them, as ADR 0045
   refuses them, saying why: no bridge was declared for the plugin (and where `janus verify` looked for
   one), its transport is not bridged yet (S5.2e), or its rules are not (S5.2g). The rest of the pact is
   verified either way.
6. **The engine declares a bridge for each plugin a pact names, as the host tells it how.** A
   verification's `options.plugin-bridge` is the bridge's `source`, `grants` and `limits`, everything a
   declaration has but its name and arguments. Given one, the engine declares `pact-plugin-<name>` for
   each plugin a pact's metadata names, with the plugin's name and version as arguments (the highest
   version, when a pact names a plugin twice), unless the run's components already declare that name.
   Pacts read with the request are declared for before the run's components load; a pact fetched from a
   broker or by URL is declared for when it arrives, and its bridge loaded then, before any of its
   interactions runs. `janus verify` sets the option when it finds the bridge: where
   `JANUS_PLUGIN_BRIDGE` names it, else beside the `janus` executable. It grants the variables
   pact-plugin-driver reads (`PACT_PLUGIN_DIR`, `HOME`, `USERPROFILE`, `PACT_DO_NOT_TRACK`, `RUST_LOG`)
   and a 30 s deadline, since starting a plugin is slower than a call. An SDK's verifier can set the same
   option; neither SDK does yet.
7. **A plugin request body is sent as recorded, unless it has generators.** Then it is passed through the
   plugin's content generator first, as today's verifier passes it, through an action the bridge
   contributes for a `content-generator` entry, `<component>:generate-content`, whose result is the
   generated body as a `value`. The engine applies it through the run's actions before the request is
   sent, as it applies a v1–v4 request's other generators (ADR 0034).

## Alternatives considered

- **Keep one mismatch, its message joining every difference.** It is what the engine does today and
  needs nothing new, but a reader gets one line for every row of a CSV report, a host cannot point at any
  of them, and a run's count is not today's verifier's.
- **Translate a plugin's location into an absolute path** (`$.square` under `$.message.body` as
  `$.message.body.square`). The body is octets to Janus: no document has that path, so `explain` and a
  host would be pointed at nothing. The plugin's location is the plugin's, and is reported as such.
- **One node per mismatch,** the plugin's answer splayed into child nodes at execution. A plan's nodes
  are what its compiler emitted (§2.1), and the executed form would stop being the plan with results
  appended (§3.2).
- **`janus verify` declares the bridges, as ADR 0106 decision 5 first said.** It cannot see the pacts a
  broker serves, which is how most providers verify, and every SDK would have to repeat it. The host still
  decides where the bridge is, which is the part only a host can know.
- **Leave request generators for later.** Neither example this item verifies has one, but a pact that has
  them would then send a stale body where today's verifier sends a fresh one, and pass where it should
  fail, or the reverse. The action is small beside the cost of a silent difference.

## Consequences

- S5.2d's exit is S1.1 fixtures for pact-plugins' CSV example (HTTP) and protobuf example (a message),
  whose verdicts agree with `pact_verifier_cli` 1.5.0's, with the pinned plugins in CI.
- The plan grammar, the engine protocol and the component interface each gain one optional member
  (`mismatches` on an `error` result and a component's result document, `at` on a `Mismatch`): additive,
  so no grammar or protocol version moves.
- Today's verifier and `janus verify` now report a plugin's mismatches alike, in the plugin's words.
- A plugin interaction still has no contract form: `janus upgrade` drops it as ADR 0045 decided, until
  S5.2f gives one.
- **Tripwire**: a plugin whose body the legacy compiler must handle differently from another plugin's.
  The argument decision 3 builds would then be missing something the capability should say.
