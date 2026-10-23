# 0045 — Refuse an interaction that needs a pact plugin by name, and verify the rest

- **Status**: accepted
- **Date**: 2026-10-23
- **Plan tasks**: S1.8
- **Evidence**: pact-reference's `pact_verifier` (`verify_provider_async` loads `pact.plugin_data()`
  before verifying) and `pact-plugin-driver` 1.2.6 (`load_plugin` installs a missing plugin from
  the index); the pacts in pact-plugins' `examples/` (CSV, JWT, credit card, protobuf, gRPC, JSON-RPC,
  Avro); `pact_verifier_cli` 1.3.5 run with an empty `PACT_PLUGIN_DIR`

## Context

A v4 pact can need a pact-plugins plugin: protobuf and gRPC, Avro, CSV, JWT and others. Janus cannot
load one until the bridge (S5.2). Before this decision, `janus verify` did one of two things with
such an interaction, and both were wrong:

- **An HTTP one was verified with the built-in components.** A CSV body was compared as text, with
  its `column:1` rules ignored. Such an interaction could pass while verifying only part of what the
  pact asserts.
- **A message one was skipped without a word**, as every message interaction is until S1.7. A gRPC
  pact verified nothing and could not fail.

Today's verifier loads every plugin a pact's metadata names before it verifies anything. A missing
plugin is installed from the plugin index, and one that cannot be installed ends the whole run with
no results.

A pact says an interaction needs a plugin in one of two ways. The examples show both:

- An interaction a plugin built names it in its `pluginConfiguration`. Every CSV, protobuf, gRPC,
  JSON-RPC, JWT and Avro example does.
- A plugin that contributes only a matcher or a generator, such as `creditcard`, leaves no
  configuration. The pact names it in `metadata.plugins`, and the interaction carries a rule
  `pact_models` does not know, which it reads as `MatchingRule::Plugin`.

## Decision

1. **An interaction needs a plugin** when its `pluginConfiguration` names one. It also needs one when
   it uses a matching rule `pact_models` reads as a plugin's and the pact names plugins. Then it
   needs every plugin the pact names, because the pact does not say which one provides the rule.
   Today's verifier loads them all too. A rule like that in a pact that names no plugin is nobody's:
   the interaction is HTTP, and its rule compiles to an error node, as an unknown rule always has.
2. **`janus verify` refuses such an interaction by name before anything runs for it.** No state is
   set up, and nothing is sent. Its result is `failed` with `component-unavailable`.
   `details.component` names the first plugin as `pact-plugin/<name>`, `details.plugins` lists
   every one, and `details.reason` is `pact-plugin`. The summary counts it as a failure, so a pact
   that holds one is never `verified`.
3. **The rest of the pact is verified.** Where today's verifier stops at a plugin it cannot load,
   Janus reports each interaction it can verify, and refuses the others.
4. **`janus upgrade` drops such an interaction** with an `interaction-dropped` finding naming the
   plugin, and **`janus explain` refuses it** with the same error, so neither shows a contract or a
   plan that would verify less than the pact.

## Alternatives considered

- **Refuse the whole pact, as today's verifier refuses a run.** That matches today when the plugin
  is missing. But today installs a missing plugin, so a provider rarely sees that. And a refused pact
  would report nothing about the interactions Janus can verify, which is what a provider trying
  Janus beside today's verifier wants to compare.
- **Refuse any interaction in a pact that names plugins.** That is simpler. But the pacts mix them:
  pact-js-core's gRPC pact has a plain JSON interaction beside the gRPC one.
- **Infer the plugin from a content type.** The pact does not map a content type to a plugin, and a
  plugin can claim `application/json` (JSON-RPC).

## Consequences

- A CSV, JWT or other plugin-backed HTTP interaction that passed under Janus now fails as refused.
  That is the point: it was verified only in part.
- A gRPC, protobuf or Avro message interaction is now reported, as a refusal, where it was skipped.
  Message interactions that need no plugin are still skipped, until S1.7.
- S1.1's harness has no plugin fixture. Today's verifier installs a missing plugin from the
  internet, so a fixture would download plugin binaries on every CI run. The refusals are tested
  offline, in the kernel's tests and the CLI's.
- S5.2 closes this. Its bridge loads the plugins named here, and the refusal is what an engine
  without the bridge still answers.
- **Tripwire**: if pilot providers have pacts whose plugin matchers sit beside several named plugins,
  and a refusal naming all of them confuses, the pact's per-rule plugin data is worth reading.
