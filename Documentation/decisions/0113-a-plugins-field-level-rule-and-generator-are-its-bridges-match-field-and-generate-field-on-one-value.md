# 0113 — A plugin's field-level rule and generator are its bridge's match-field and generate-field, on one value

- **Status**: accepted
- **Date**: 2026-11-27
- **Plan tasks**: S5.2g (matcher-only plugins, and version 2's field operations); S1.8 and ADR 0110 decision
  5, whose last refusal for a plugin's catalogue this closes but for S5.2f's
- **Evidence**: [ADR 0110](0110-a-v1-v4-plugin-body-compiles-to-its-bridges-compare-contents-and-one-node-reports-each-mismatch-it-found.md),
  [ADR 0109](0109-a-bridged-plugins-content-matcher-is-one-compare-contents-action-and-its-catalogue-a-handshake-capability.md),
  [ADR 0034](0034-apply-a-pacts-request-generators-through-a-generator-component-fed-by-state-values.md);
  pact-plugins' proposal 006 (field-level matchers and generators) and its reference plugin,
  `plugins/creditcard`, with the credit card example's pact (`examples/creditcard/creditcard-consumer-jvm`);
  pact-plugin-driver 1.2.7's `field.rs` (`FieldMatcher`, `FieldGenerator`, `FieldValue`) and
  `catalogue_manager::resolve_capability`; `pact_matching`'s `field_rules.rs` (`apply_plugin_rule`, the
  path and category it scopes, and mismatches joined with `, `) and `core_capabilities.rs`
  (`DriverPluginSupport::generate`); `pact_models`' `MatchingRule::Plugin`, `Generator::Plugin` and
  `can_cascade`, which nothing in `pact_matching` consults for a plugin rule; `pact_verifier_cli` 1.5.0,
  run against the pinned plugin

## Context

A plugin of interface version 2 can register a `MATCHER` and a `GENERATOR` catalogue entry: a rule and a
generator that apply to one value inside somebody else's content, a field of a JSON body, a header, a
message's metadata. pact-plugins' `creditcard` plugin is the reference: a JSON body whose
`$.card.number` must be a Luhn-valid number of a brand, written in the pact as an ordinary rule whose name
no core rule has (`{ "match": "creditcard", "brand": "visa" }`), with the plugin named in the metadata's
`plugins` and nowhere in the interaction. Today's verifier resolves the name against the plugin catalogue
and calls the plugin's `MatchField` with the value, its example, its path and its part; a request generator
of such a name goes to `GenerateField`.

`janus-plugin-bridge` describes these entries under its `pact-plugin` capability and contributes nothing
for them, and the legacy layer refuses every interaction with a plugin rule (ADR 0110 decision 5, `rules`).
Four questions:

- **What the bridge contributes.** The other entries became one action each, over a whole body or a whole
  exchange. These are over one value.
- **How a rule reaches its action.** The rule's name is all the pact says. Which plugin provides it, and so
  which bridge, is known only from the run's bridges, which the legacy compiler's rule-by-rule walk does
  not see.
- **What the plugin is handed.** `MatchField` takes the value's path as a pact writes it (`$.card.number`)
  and the part it came from (`body`, `header`, …), to place what it reports. The engine's paths are plan
  paths (`$.response.body.card.number`), and an element inside a `for-each` is relative until it runs.
- **Generators.** A request generator of a plugin's name is today sent to the generator component, which
  does not know it, so the recorded value is sent where today's verifier sends a fresh one.

## Decision

1. **Each is one action of the bridge, applied to one value.** A plugin of interface version 2 that
   registers a `matcher` entry has its bridge contribute `<component>:match-field`, and a `generator` entry
   `<component>:generate-field`; the catalogue entry names it as its `action`. A version 1 plugin's
   `matcher`, which no driver ever called, stays described and not contributed. The bridge builds the
   plugin's `MatchFieldRequest` and `GenerateFieldRequest` as pact-plugin-driver's `FieldMatcher` and
   `FieldGenerator` build them for today's verifier: the value as a `FieldValue`, whose whole numbers stay
   whole and whose octets stay octets, no plugin configuration and an empty test context for a match, the
   provider's test mode and the provider-state values as the test context for a generation. A rule or
   generator is named by its entry's key or a longer form of the catalogue key ending in it
   (`matcher/creditcard`, `plugin/creditcard/matcher/creditcard`), as `resolve_capability` matches one.
2. **The legacy layer resolves a plugin rule to its action before the compiler sees it.** For an interaction
   verified through its bridges, each rule `pact_models` reads as `MatchingRule::Plugin` that one of the
   interaction's bridges provides, and exactly one, is rewritten in the legacy layer's own copy of the
   interaction: its name becomes the bridge's `match-field`, and its values the action's argument, the
   rule's own `name` and `values`, the `mismatch-type` today's verifier hands the plugin for the rule's
   category, and the `root` of the slot it applies in. The compiler compiles a plugin rule so named to that
   action on the value at the rule's path, with the pact's example and the argument as its children: on a
   scalar, an object or an array, and whether or not it cascaded there, as `pact_matching` applies one. A
   body a plugin's content matcher compares keeps its rules as they are, since they are that plugin's to
   read (ADR 0110 decision 3).
3. **The bridge works out the path a pact writes.** The engine sends each value's absolute path with it
   (§7.2), the element's own inside a `for-each`; the bridge hands the plugin that path with the argument's
   `root` taken off, and no path for the request path or the status, as today's verifier hands none. What
   the plugin answers is one result: `ok`, or one `error` whose message is its mismatches' joined with
   `, `, as `pact_matching` joins them, or its error when it could not apply the rule. Several mismatches,
   or one it located elsewhere, are also the error's `mismatches`, each `at` the plugin's path (ADR 0110
   decision 1).
4. **A plugin request generator is filled through its bridge.** A request generator `pact_models` reads as
   `Generator::Plugin` that one of the interaction's bridges provides is applied through its
   `generate-field`, where the request's other generators are applied (ADR 0034), with the recorded value as
   the example and its path as a pact writes it. A generator that fails sends the recorded value and is
   reported, as every request generator is. An interaction with a plugin generator needs the pact's plugins
   as one with a plugin rule does.
5. **A rule or generator no bridge provides is refused by its name.** ADR 0110 decision 5's `rules` refusal
   now names the rules and generators none of the interaction's bridges provides (a misspelt core rule, or a
   plugin of version 1), instead of refusing every plugin rule. The rest of the pact is verified either way.

## Alternatives considered

- **Thread the run's bridges through the compiler,** so a rule finds its action where it is compiled. Every
  one of the compiler's paths to a rule (a header, a query, metadata, XML, a multipart part) would carry them, for a v1–v4 notion the compiler
  otherwise never sees; resolving the name once, beside the other plugin decisions, keeps the compiler's
  input a pact's rules.
- **A thread-local scope while compiling,** as `pact_matching` scopes a plugin rule's path. It hides an
  input of compilation in ambient state, which the kernel avoids everywhere else.
- **The engine computes the plugin's path.** It would need the pact path at the rule's node, which a
  relative element does not have until it runs; the absolute path every application already carries does.
- **One action per catalogue entry** (`pact-plugin-creditcard:creditcard`). A plugin's keys are open text,
  not action names, and a plugin with several rules would contribute as many actions for one operation.
- **Leave generators for later.** The credit card example's own generator is on its response, which a
  verifier does not fill. But a request generator would then send a stale value where today's verifier
  sends a fresh one, the silent difference ADR 0110's generators were added to avoid.

## Consequences

- S5.2g's exit is S1.1 fixtures for the credit card example (passing, failing the Luhn check, failing the
  brand) and a request generator the provider recognises only when it is fresh, all agreeing with
  `pact_verifier_cli` 1.5.0, with the plugin pinned by digest. The plugin has no release, so its Lua files
  are pinned at the commit that last changed them (`plugin-bridge/pact-plugins.json`).
- A plugin rule's argument shows in `janus explain`, so a reader sees which bridge applies it and with what.
- ADR 0110 decision 5's refusals for a plugin's catalogue are down to an `interaction` entry's (S5.2f), and
  a rule or generator nobody provides.
- A content plugin that delegates a field to a field-level plugin (proposal 006's host callbacks) is not
  bridged: the bridge answers no callback.
- **Tripwire**: a plugin rule whose answer depends on what today's verifier hands the plugin beyond what
  decision 1 sends (its plugin configuration, a test run id). The argument would then need to carry it.
