# 0109 — A bridged plugin's content matcher is one compare-contents action, and its catalogue a handshake capability

- **Status**: accepted
- **Date**: 2026-11-26
- **Plan tasks**: S5.2c (the bridge and its handshake); S5.2d, which compiles a v1–v4 body to the action
  and reads the capability; S5.2e–S5.2g, which bridge the rest of a catalogue
- **Evidence**: [ADR 0106](0106-bridge-pact-plugins-through-a-child-process-that-speaks-the-subprocess-binding.md);
  pact-plugins' `proto/plugin.proto`; `pact-plugin-driver` 1.2.7 (`content.rs`'s
  `ContentMatcher::match_contents`, `catalogue_manager.rs`'s `matches_pattern`, `lua_plugin.rs`);
  `pact_matching`'s `plugin_support.rs` (the configuration today's verifier hands a plugin); the CSV and
  area-calculator example pacts in pact-plugins' `examples/`; `plugin-bridge/`, which this item built

## Context

ADR 0106 decided that each of today's pact-plugins reaches the engine through `janus-plugin-bridge`, a
child process speaking the subprocess binding, as a component named `pact-plugin-<name>` whose handshake
is the plugin's catalogue translated, and whose content matcher becomes a matcher action applied to a
whole slot's octets. It left four things to the item that built the bridge:

- **What the handshake declares.** A plugin's catalogue has five kinds of entry, and the engine may only
  be told of what it can call (ADR 0104). The bridge built here answers a content matcher and nothing
  else, yet S5.2d's legacy compiler has to know which bodies a plugin compares, and that is in the
  catalogue.
- **How many actions.** A plugin may register several content matchers, each for content types of its
  own, and `CompareContents` names no entry: the plugin decides from the body's content type.
- **What the action is told.** `CompareContents` takes an expected and an actual body, each with a
  content type, the pact's rules for the body, whether unexpected keys are allowed, and the plugin's
  configuration for the interaction and the pact. A matcher action gets the value under test and its
  arguments (spec §7.2).
- **What it answers.** A plugin answers mismatches keyed by path, a content-type mismatch, or an error
  string. A plan node has one result, and an `error` has one message and one path (plan grammar §2.3).
  How one node reports several located mismatches is S5.2d's decision (ADR 0106's consequences).

## Decision

1. **A bridge is `pact-plugin-<name>` at the version of the plugin it loaded.** Its arguments are the
   plugin's name and version, both required. The handshake's version is the loaded manifest's, which is
   what answers, since the driver may load a compatible one. A plugin whose name does not make a
   component name (spec §2.1), and arguments that name no plugin, are answered at the handshake as
   `unavailable`, so the engine reports them against the declaration rather than as a process that died.
2. **The plugin is loaded at the handshake, by pact-plugin-driver,** because only a loaded plugin can say
   its version and catalogue. A plugin that cannot be loaded is `unavailable`, with `plugin` and `version`
   in its details and the driver's causes as its `chain` (ADR 0054). The plugin lives as long as the
   bridge's standard input: `component/shutdown` changes nothing, and when the input ends the bridge
   shuts the plugin down and exits. A bridge the loader ends takes its plugin process with it, since the
   loader ends a component's process group, or on Windows its process tree.
3. **The handshake contributes only what the bridge answers.** It implements `matcher`, and contributes
   one action, `<component>:compare-contents`, when the catalogue has a content matcher, however many it
   has: the body's content type selects the entry, as it does for the driver. Transports, content
   generators, matchers and interaction entries are contributed by S5.2e–S5.2g, when the bridge answers
   for them. The bridge declares no operator, so `matcher/compare` and `matcher/prefix` are
   `operation-unsupported`.
4. **The catalogue is the handshake's `pact-plugin` capability.** It names the plugin, its version, its
   interface version and the bridge, and lists every catalogue entry: its type as the driver names it
   (`content-matcher`, `content-generator`, `transport`, `matcher`, `interaction`, `generator`), its key,
   its values as the plugin registered them, the `content-types` a content matcher or generator claims,
   and, for an entry the bridge serves, the `action` it is reached through. A content type is a pattern,
   claimed as pact-plugin-driver claims it: a regular expression, anchored at both ends, over a media
   type's type and subtype with any suffix, parameters stripped. Component-interfaces spec §9.3.1 and the
   handshake schema define the capability, so S5.2d's compiler reads it from the specification rather than
   from a member the specification does not define (spec §3.3).
5. **`compare-contents` is told what the pact says about the body.** The value under test is the actual
   body's octets, in any of a `MatchValue`'s encodings. The action's one argument is an object:
   `content-type`, the expected body's as the pact records it, parameters included, which selects the
   content matcher and from which the protobuf plugin reads the message type; `expected`, the expected
   body as a `MatchValue`, absent when the pact has none; `actual-content-type`, defaulting to the
   expected one; `rules`, the pact's rules for the body in the v4 pact form, keyed by path as the pact
   writes them; `allow-unexpected-keys`, defaulting to false; and `plugin-configuration`, whose
   `interaction` and `pact` members are what today's verifier hands the plugin: the interaction's
   configuration for the plugin (for the part compared, when it is kept by part) and the configuration
   the pact's metadata records for it. The bridge builds the plugin's request as the driver's
   `match_contents` builds it for today's verifier, so the plugin is asked the same question, and calls
   the plugin through the driver itself, so it can tell from the call's own error whether the plugin
   answered (decision 6).
6. **What the plugin answers is one result; a plugin that does not answer is the bridge's failure.**
   No mismatch is `ok`. Mismatches are one `error`, whose `message` names each, prefixed by the path the
   plugin gave it and joined with `; `, in path order, and whose `mismatches` member keeps each one whole:
   its `message`, where the plugin located it (`at`, as ADR 0110 names it; this decision first called it
   `path`), and the `expected`, `actual`, `diff` and `type` the plugin gave. Everything the
   plugin answered is read as `match_contents` reads it for today's verifier: a content-type mismatch, the
   plugin's own error string, and a call the plugin answered with a failure of its own (a gRPC status such
   as the CSV plugin's `Aborted` for a body it cannot read, or a Lua plugin's script error) are
   mismatches. A call that never reached the plugin is not: the driver could not connect, or gRPC answers
   `Unavailable`, `Cancelled` or `DeadlineExceeded`. Today's verifier reports that as one more mismatch,
   which blames the provider for a dead plugin, and every later comparison fails the same way. The bridge
   answers it as `unavailable`, with `plugin` and `version` in its details and the cause as its `chain`,
   and then exits, so the engine's next call starts a new bridge and a new plugin: out of process the
   process is what is recreated (ADR 0108 decision 3). The engine reports each of a result's mismatches
   as a mismatch of its own (ADR 0110).
7. **The bridge refuses in the specification's words:** `unknown-action` for an action it did not
   contribute, `unsupported-content-type` for a content type none of the plugin's content matchers
   claims, `invalid-config` for an argument it cannot read, each a `document` error (spec §11.1).
8. **A bridge's environment is its grants.** A subprocess component starts with an empty environment
   plus what it is granted (spec §9.3), so a provider grants the bridge `PACT_PLUGIN_DIR`, or the home
   directory's variable where `~/.pact/plugins` is used, and `PACT_DO_NOT_TRACK` when the driver should
   send no usage metrics. Installing a missing plugin needs the network, which the binding does not
   enforce either way. The bridge logs to standard error at the level `RUST_LOG` names, when it is granted.
9. **The bridge's tests run everywhere, and the plugins are pinned.** A Lua plugin under
   `plugin-bridge/tests/fixtures/plugins/` runs inside the bridge's process, so its own tests download
   nothing. The pact-plugins releases its conformance cases are run against are pinned by digest in
   `plugin-bridge/pact-plugins.json` and installed by `.github/scripts/pact-plugins.sh`, never from the
   plugin index during a test.

## Alternatives considered

- **An action per content matcher** (`compare-contents-<key>`). `CompareContents` names no entry, so
  the actions would differ only in name, and the legacy compiler would have to pick one by the same
  content-type match the bridge makes anyway.
- **The content types as members of the action's entry in `contributes`.** Spec §3.3 says an engine
  must not depend on an entry member the specification does not define, and an action entry is the wrong
  place for a plugin's other entries. A capability is the protocol's mechanism for what one party offers
  beyond the required, and defining this one in the specification is what makes it readable.
- **Contributing the transport and generator entries now.** ADR 0104's rule: a handshake that declares
  what nothing calls is refused. They come with the items that answer them.
- **The rules in `CompareContents`' own form** (`{path: [{type, values}]}`). It is a second spelling of
  a pact's rules, which the legacy compiler already has in the pact's own form, and the driver converts
  from the pact form anyway.
- **Every plugin error as a `ComponentError`**, its error string and any failed call alike. It reads as
  more honest, since a provider did not mismatch. But the string, and a status the plugin answers with,
  say the same thing whether the plugin or its configuration is broken or the provider's body cannot be
  read, and the bridge has only free text, different in every plugin, to tell them apart by. A body that
  cannot be read is a mismatch at its slot everywhere else (spec §6.1, §11.3), and blaming the plugin for
  a provider's bad response sends a reader to the wrong place. Today the difference would be only the
  wording: a component's `matcher/apply` error is put on the node as its error, so the verdict and the
  exit code are the same, which S5.3 left so on purpose (ADR 0101). Were it ever made `component-failed`
  at the engine boundary, as spec §11.3 reads, a provider's unreadable response would stop being a failed
  verification (exit 1) and become a run that could not be made (exit 2). Only a plugin that could not be
  reached is unambiguously not the provider's, and only that is an error (decision 6).
- **Loading the plugin at the first `apply`.** The handshake could not then name the plugin's version or
  contributions, and the handshake is the only source of truth about a component (spec §3.2).

## Consequences

- S5.2c's exit holds: `janus component check` passes the bridge's cases against pact-plugins' CSV plugin
  (interface version 2) and the protobuf plugin (version 1), through the subprocess binding, and against
  the Lua fixture on every operating system.
- S5.2d compiles a v1–v4 body whose content type a plugin's `content-matcher` entry claims to the action,
  with the argument decision 5 describes, and decides how the mismatches decision 6 carries are reported.
- `tests/bridge.rs` kills the CSV plugin under a running bridge and sees the next call answered
  `unavailable` and the bridge leave; the classification is unit-tested against each gRPC status.
- A plugin's mismatches are the plugin's words and paths (`row:    1, column: 1`, `$.square`), not the
  engine's: a pact path is not a Janus path, and the bridge does not translate one.
- **Tripwire**: S5.2d needing to know something about a particular plugin that the capability does not
  say, or a plugin whose `CompareContents` needs more than decision 5's argument. Either means the
  capability or the argument is missing a member.
