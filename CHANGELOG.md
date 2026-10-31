## 0.4.3 (2026-10-31)

### Features

#### `JANUS_LOG_FORMAT=json` writes each log event as a JSON line

With `JANUS_LOG_FORMAT=json`, `janus-engine` and `janus` write each event on standard error as one
JSON object: `timestamp`, `level` (`TRACE` to `ERROR`), `target`, and `fields`, whose `message` is the
event's message. The same events are held back as in text, credentials included. An SDK reads these
members to hand the engine's events to its own runtime's logger (ADR 0061).

## 0.4.2 (2026-10-31)

### Features

#### A test can produce a provider's messages, and a produced message is held only to a content type it has

A provider test's message producers are `produce-message` hooks it serves on its loopback endpoint,
as its state handlers are (ADR 0056). Each is named `message-<n>` and selected by its interaction's
description: `{ name, when: { interaction }, changes: ["parts"], run }`. It answers the whole
`message` part, with the body base64-encoded and labelled only when the producer gave a content type.

The engine changed in one place. A v1–v4 message whose produced metadata has no `contentType` member
at all is now read as carrying the message's own content type there: its body's label, else what its
octets are detected as. Before, it failed on the metadata's expected `contentType`. The `pact-message`
format always writes one, so nothing `janus verify` produces changes. A script or component that
produces a message with no `contentType` in its metadata now verifies where it failed.

For an SDK maintainer: the behavioural specification gains `message-producer`, with four conformance
ids, and `verify.transport-options-as-given`. A verifier's `transport` option is the HTTP transport's
options beside `base-url`. The conformance suite has three provider cases and one live case for them.

#### An SDK can verify a provider inside a test, and the engine loads a project's configuration for it

Staged plan S1.14, ADR 0055.

- **`config/load`** (engine-protocol spec §8.8): `{ path }` → `{ config }`, the project
  configuration file resolved by the loader `janus verify --config` runs. `${VAR}`s come from the
  engine process's environment, scripts are inlined and paths are made absolute. `path` must be
  absolute, or the call is `malformed-frame`.
  - It is behind a new `config-loading` capability, which `janus-engine` and `janus` declare.
  - The errors are the loader's: `hook-unresolved`, which names the `variable` or `path`, and
    `hook-config-invalid`.
  - An engine without a loader, such as the offline WASM artifact, answers `component-unavailable`
    naming `config-loader`.
  - The generated bindings gain `Load`, `LoadResult` and `ResolvedConfig`, and `config/load` joins
    the known operations. They also gain the lifecycle-hooks schemas as a new set, `hooks`
    (`HookContext`, `HookConfig`, `HookEntry`, …), which a provider SDK serves and writes.
- **The behavioural specification has a provider side:** `verifier`, `state-handler`,
  `request-filter` and `verify`, in a new `verification` category. Its schema adds a `hook-entry`
  kind to `produces`.
- **The conformance suite has a `provider` category**, and its scripted engine calls an SDK's hooks
  during a run. An SDK that implements the provider primitives must pass its 14 cases and the new
  live case.

#### A provider state no hook sets up is a warning, or a failure when the run asks

Before, a provider state that no `state-setup` hook admitted (none configured, or every one's `when`
selecting other states) ran its exchange without setup and said nothing, so a missing or misspelled
state handler went unnoticed (ADR 0058).

Now the run emits a `verification/warning` with `code: state-unhandled`, the interaction, the variant
and the `state`, before the result. The exchange still runs without setup and the verdict is
unchanged, as pact-js treats it. `janus verify` prints the warning.

`verification/verify` takes `options.unhandled-states: "fail"` to fail such a variant instead, before
its exchange, with `component-unavailable`, `details.component: hook/state-setup`,
`details.reason: state-unhandled` and `details.state`, as pact-jvm treats it.
`janus verify --unhandled-states fail` sets it.

For an SDK maintainer: the behavioural specification's `verify` result gains `warnings`, every
`verification/warning` payload in order, which the run's report says too (ADR 0059)
(`verification.verify.warnings-collected`, conformance case `provider/warnings-collected`). The
bindings' `VerifyOptions` gain `unhandled-states`.

#### The engine writes a verification run's report

A run that asks with `options.report` (ADR 0059) gets its report on its stream: a
`verification/report` event after each event worth a line, carrying `text` to print as it arrives,
and `text-segments` for a host that declared `segments`. It is the report `janus verify` printed,
laid out by the engine instead of the CLI: the start, each result with its mismatches and the hook
that failed it, warnings, broker fetches and publications, executed plans, and the summary's line.
`options.report.error-chain` says every layer of each error's chain.

Two changes to what `janus verify` prints: each result names its interaction's provider states under
it (`given …`), and a run stopped at its first failure no longer names `--exit-on-first-error`.

For an SDK maintainer: the behavioural specification's `verify` adds `report: {}` to the options
unless the caller gave `report`, and writes each report event's text to standard output, as pact-js
prints the verifier's report (`verification.verify.report-printed`, conformance case
`provider/report-printed`). An SDK no longer prints warnings itself: the report says them. The
bindings gain `Report` and the `verification/report` event kind.

### Fixes

#### A v4 `statusCode` matcher verifies, through the HTTP component's `http:status-class`

A v1–v4 pact whose response status has a `statusCode` rule failed `janus verify`, which refused the
rule. It now compiles to `http:status-class`, which the built-in HTTP component runs (ADR 0057): a
`class` (`information`, `success`, `redirect`, `client-error`, `server-error`, `non-error`, `error`)
or a list of `codes`, each as today's verifier checks it. A contract `janus upgrade` wrote with the
operator verifies too.

`janus upgrade` now writes a list of codes as `codes: [200, 204]`, where it wrote a comma-separated
`class` that nothing read.

For an SDK maintainer: `janus-engine` registers the matcher, so nothing changes for an SDK. An
embedding built on the kernel registers a matcher compiled into it with `Engine::register_matcher`;
the WASM engine has no HTTP component, and fails a plan that reaches `http:status-class` by name.

## 0.4.1 (2026-10-29)

### Features

#### A mismatch is located at the element that failed, and an absent value prints as `ABSENT`

Staged plan S1.30: what the engine reports, from the review of the CLI's output.

- **Every mismatch path is absolute.**
  - One inside a `flow:for-each` template, a `match:contains` entry or a `legacy:array-contains`
    variant is located at the element or entry it was about: `$.response.body.prices.plum`, not
    `~>.value`, and `$.request.body.animals[1].phoneNumber`, not `~>.phoneNumber`.
  - An entry's `.key` and `.value` are located at the entry's member.
  - A mismatch from an action whose value under test is itself computed, such as `expect:size` over
    `legacy:xml-children(...)` or `match:equality` over `value:upper-case(...)`, is located where that
    value came from. Until now it had no path.
  - The plan-grammar spec's §2.3 states the rule. A host that showed relative paths needs no change.
- **The executed text form writes an absent value as `ABSENT`**, and a `null` as `NULL`; both used to
  be `NULL` (plan-grammar spec §3.2). The pretty form is unchanged: no value node can be absent.
- **A container for a member no plain path segment can hold is labelled as its resolve names it**:
  `:"$['']"` and `:"$['a.b']"`, not `:"$."` and `:"$.a.b"`. A label at a slot's root is shortened as
  every other is (`:"$[0]"`, not `:"$.response.body[0]"`).
- **Messages:** "Expected an integer" and "an array", not "a integer" and "a array". A count says
  "1 item" or "2 items", and `subsumption/decide`'s report text, `filter-matches-nothing` and the pact
  plugin refusal use plurals instead of `(s)`.

#### `janus` colours its output on a terminal

Staged plan S1.31. Until now, only the plans that `explain` and `--explain-failures` print were coloured.

- **Every command's text output is coloured on a terminal**, in one palette. A pass is green, a
  failure red, and a warning yellow. A mismatch's path is blue, and a line that explains the one
  above it is dim. That covers `verify`'s result lines and summary, the page `check` prints, and
  `upgrade`'s findings. It also covers `publish`, `component push|pull`, and the `janus <command>:`
  line that says why a command stopped. `--help` uses cargo's colours.
- **Off a terminal, nothing changes.** Through a pipe, into a file, under `NO_COLOR` or with
  `--no-colour`, the output is byte-for-byte what it was. `CLICOLOR_FORCE` asks for colour anywhere,
  as before. `--output json`, contracts and completion scripts are never coloured.
- **`--no-colour` now reaches `--help` and usage errors**, which it did not before.

#### Display text can carry segments, and `janus` colours from them

Staged plan S1.34, ADR 0054.

- **The `segments` capability.** The engine now declares it in `engine/hello`. A host that declares
  it too gets each display string with a `<member>-segments` sibling: pieces whose texts, joined,
  are exactly the string, each with a `kind` such as `pass`, `fail`, `path`, `string` or `action`.
  A kind says what the piece is, never how it looks. Without the host's declaration, nothing on the
  wire changes. See engine-protocol spec §2.7 and `display.schema.json`.
- **Members that carry segments:**
  - `verification/explain`'s `text` and `executed.text`;
  - the `verification/executed-plan` event's `text`;
  - `subsumption/decide`'s `text`;
  - each mismatch's `message`, in explain's result and in `verification/interaction-result` events,
    with the values it quotes marked.
- **The bindings** gain `Segment`, `SegmentsCapability` and a typed `Mismatch` for explain's
  `executed.mismatches`.
- **`janus`** colours plans, the `check` page and the values in mismatch messages from segments,
  where it used to parse the text. Its `--output json` documents carry no segments.

#### An error carries its causes as a chain, and `janus` shows what failed and why

Staged plan S1.33, ADR 0054.

- **`EngineError` may carry `chain`**: its layers, outermost first, each `{ code?, message }` in that
  layer's own words. The first layer is the error itself and the last is its root cause.
  - `message` keeps its meaning, the complete sentence, so a host that reads only it loses nothing.
  - A component's error document may carry the same member.
  - `component-failed` writes its own layer, then the component's chain.
  - The generated bindings gain `ErrorLayer`.
- **Errors that carry a chain:**
  - a request the HTTP transport could not send;
  - a broker that could not be reached, refused a request, or answered with something that is not JSON;
  - an `http` hook that got no answer or a non-2xx one;
  - an `exec` hook that could not start or exited non-zero.

  A failing `exec` hook's message now ends with the last line it wrote to stderr, which used to
  appear only in `details`.
- **`janus` prints an error as what failed and its root cause:** the innermost layer with a code,
  then a `caused by:` line. Use `--error-chain`, a global option, to print every layer.
- **Wording.** A broker's retry-after note now comes before the broker's own explanation. A
  configuration file that cannot be read no longer says `(os error 2)`.

#### `janus explain --description` chooses an interaction by its description

Staged plan S1.32. Until now `explain` took the interaction only by its position, `--index`. In a large
pact, that meant counting interactions or trying one index after another.

- **`--description <TEXT>`** chooses the interaction whose description is exactly the text, or else the
  one whose description contains it, ignoring case. An exact match wins, so a description that is the
  start of another's can still be chosen.
- **Several matches, or none, is a usage error (exit 2)** that lists the candidates. Each line gives the
  position `--index` takes, the description and the provider states, which tell apart interactions a
  pact describes alike.
- It cannot be combined with `--index` or `--spec`.

#### A run that ended early counts what it did not reach, and errors say why in their message

The second part of staged plan S1.29's review of the CLI's output.

- **`variants.not-run`.** The summary of a run that `stop-on-first-failure` stopped, or that a hook
  aborted, now counts in `variants.not-run` the recorded variants it did not reach. No other tally
  counts them, and `total` is unchanged. Until now, a stopped run did not say how much it left, and an
  aborted run said it only in the hook report, which a run with no hooks does not have.
- **Messages carry what a person needs, so a host can show the message alone:**
  - `variant-not-found` from `verification/verify` names the ids the documents do record ("…; they
    record 'base'").
  - An `http` hook whose endpoint answers with an error status quotes up to 200 characters of the body
    in its `hook-http-failed` message. The details still carry the whole excerpt.
  - The excerpt no longer drops the body's last character.
- **Shorter error chains:**
  - The engine's HTTP client leaves out the layers that only name a library ("error sending request",
    "client error (Connect)", "tcp connect error") and the platform's `(os error N)`, so a refused
    connection reads `could not connect: Connection refused`.
  - `broker-unreachable`'s message is the request and its cause, without "the Pact Broker could not be
    reached", which was wrong for a pact fetched by its URL.
  - The OCI loader and the oauth2 component say "could not reach the registry" and "could not connect"
    instead of ureq's `io:`.
  - A registry that wants credentials nobody declared says so in words.

`janus verify` follows:
- **Exit status:** a run the engine ends before it verifies anything exits 2, with the reason on
  standard error. That covers a broker or URL it could not fetch from, no pacts selected, and a variant
  nothing records. It used to exit 1, as `pact_verifier_cli` does.
- **Errors:**
  - An error's `details` are no longer printed as raw JSON.
  - A component's failure leads with the component's own code (`transport-failed: GET …`).
  - A hook that fails an exchange is printed once, under the result it failed.
- **Results and summary:**
  - A v1–v4 pact's results no longer end in `[base]`.
  - The summary leaves out tallies that are zero, apart from verified and failed, uses plurals, and
    counts what a stopped run did not reach: `FAILED: 0 verified, 1 failed, 1 not run (2 interactions)`.
- **Smaller fixes:**
  - File errors drop `(os error N)`.
  - A file that is not JSON says so.
  - `component pull` puts its code first.
  - `explain --index first` says it wants a whole number.

### Fixes

#### Documents that are not what they claim are refused, and `janus` says which file

The first fixes of staged plan S1.29's review of the CLI's output.

- **`upgrade/pact` refuses a document that is not a pact.** A document without `consumer` and
  `provider` objects and an `interactions` or `messages` array used to become an empty contract, reported
  as an exact conversion. It is now `contract-invalid`, with a problem at the empty pointer. The same
  check now covers `verification/explain`'s v1–v4 path.
- **`verification/explain` refuses a variant the interaction does not record**, as `variant-not-found`
  with the `recorded` ids in its details, as a run does (ADR 0041). For a v1–v4 interaction, which
  records only `base`, any other id used to be ignored. For a contract it used to be `malformed-frame`.
- **Messages.** A source document that is not readable is `contract-invalid` with "the document is not a
  contract or pact this engine can read". It used to say "the session's interactions cannot produce a
  valid contract", which is `consumer-session/finalise`'s message. A source that is neither a contract nor a pact
  is now "neither a Janus contract nor a v1–v4 pact". The codes and details are unchanged.

`janus` follows:
- An error about one of `verify`'s files names that file, and its positions are within it.
- `--provider-url`, `--broker-url`, `--url`, `--webhook-callback-url`, `--state-change-url` and
  `publish --broker-base-url` must be URLs with a host, checked as the command line is read.
- `explain --index` past the last interaction is a usage error, and `--spec` is suggested only for a
  document the engine could not identify.
- `publish` no longer says a broker it never reached "did not accept" the contracts.
- `pact_models`' warnings about a pact with no metadata are no longer logged by default.
- `--log-level trace` no longer makes loading a WASM component take seconds: the compiler's own
  `trace` is held back, as the HTTP libraries' is.
- The help now says `trace`, not `debug`, shows the protocol frames.

#### `janus --help` names pact_verifier_cli, and groups the global options

The last part of staged plan S1.29's review of the CLI's output, all in `--help`.

- Where an option behaves as today's verifier's does, the help names it: `pact_verifier_cli`.
- `--log-level` and `--no-colour` are listed under "Global options" in every command's help, not
  among the command's own options.
- Every option read from the environment is shown as `[env: NAME]`. Its value is never shown: help
  printed whatever the environment held, credentials aside, so its output depended on the shell.

#### The engine moves to wasmtime 50 and rquickjs 0.14

Dependency updates with nothing for a host to change; what each one could change is below.

- **Scripted hooks run on QuickJS-ng 0.16.2** (rquickjs 0.14), up from 0.8.0. The script API, its
  interruption at a deadline and its memory and stack bounds are unchanged, and every hook test
  passes, but a script that leans on an engine quirk may behave differently.
- **Out-of-tree WASM components load under wasmtime 50**, up from 47. A component's `fs` grant
  allows what it did: `read-write` is every operation, anything else is read-only. Components built
  with an older wit-bindgen still load.
- The WASM engine is built with wit-bindgen 0.63, and the engine's own base64, sha2 and jsonschema
  move to their current versions. A component digest is still `sha256:` and 64 lowercase hex digits.

## 0.4.0 (2026-10-25)

### Breaking Changes

#### A filtered run is reported as `filtered`, never `verified`, and is not published

`verification/verify`'s `options.variants` (or `options.variant`) narrows a run to some recorded variants.
Until now, such a run reported `status: verified` whenever nothing failed, even when the filter named an id
no document records and nothing was replayed at all. With `options.publish`, every pact it fetched was then
published to the broker as a success (ADR 0041).

- **A filter naming an id no document records is refused.** It fails with `variant-not-found` (category
  `session`). `details.variant` is the first unknown id and `details.recorded` lists the ids the documents
  do record. For an inline source, `verify` answers with the error. For a `broker` source, the run ends
  right after `verification/fetched` with `status: failed` and the error, and nothing runs or is
  published. An empty `variants` list is `malformed-frame`.
- **A run is filtered when its filter left out at least one recorded variant**, not merely when it was
  given one. A filter that admits everything (for example `base`, over v1–v4 pacts) leaves a whole run.
- **A filtered run that nothing failed has `status: filtered`.** Each `pairs` entry can now be `filtered`
  too, after `failed` and `incomplete`. A host that reads a run's status must treat `filtered` as "not
  verified, not failed". The summary's and `verification/started`'s `filtered` member keeps its place,
  with this meaning.
- **A pact the filter narrowed is not published.** Its `verification/published` event and `publication`
  entry have `outcome: skipped` and `reason: filtered`. A pact the filter did not narrow is published as
  before.
- **`subsumption/decide` raises `verification-filtered` per pair**, from the pair's `filtered` status. It
  no longer raises it for every pair of a run that narrowed only one of them.

`janus verify --variant` follows. An id no document records exits 2. A filtered run prints `FILTERED:` and
exits 0, since it did what it was asked. `pact_verifier_cli` passes a filter that matches nothing and
publishes a narrowed pact, and pact-jvm skips publishing it; Janus refuses the first and skips the second.

#### Message pacts are verified

- **Breaking: a v1–v4 message interaction is verified** where it was skipped. A v3 message, and a v4
  asynchronous or synchronous message, is asked of the provider by the run's `produce-message` hooks,
  which produce a `message` part (`metadata` and `body` slots), and compared with the message the
  pact expects as today's verifier compares one (ADR 0046). A message pact whose run used to report
  `verified` with nothing verified now fails unless the provider produces its messages.
- **When no hook produces a message**, the variant fails as `component-unavailable` with
  `details.component` `hook/produce-message` and `details.reason` `message-not-produced`.
- **The `http` hook has `format: pact-message`**: today's request for a message (the hook context's
  new `interaction.pact-message.request`), and the reply read as the message. Its `transports` member
  routes a message as today's `--transports` does. A reply that is not a 2xx fails the hook.
- **`janus verify` asks for messages as today's verifier does** when its configuration names no
  `produce-message` hook, and takes `--transports NAME:PORT[/PATH]`; an `http` entry is where HTTP
  interactions go.
- **A transport names its own parts.** Its contribution declares `exchange-parts` (`initiating`,
  `answering`), and the engine spells no part name of its own. A served request that does not match
  is completed with `transport/dispose`, disposition `reject`, whose new `details` carry the
  mismatches; the HTTP transport still answers it with a 500 naming them. `TransportComponent` has a
  new required method, `exchange_parts`.
- **`verification/explain` shows a message's plan**, rooted at the `message` part, with the new legacy
  action `legacy:message-type`.

#### An interaction that needs a pact plugin is refused by name

- **Breaking: a v1–v4 interaction that needs a pact-plugins plugin fails** as `component-unavailable`
  before anything runs for it: no state is set up and nothing is sent. `details.component` is
  `pact-plugin/<name>`, `details.plugins` lists every plugin it needs, and `details.reason` is
  `pact-plugin` (ADR 0045). Until this release, a CSV or other plugin-backed HTTP interaction was
  verified with its body compared as text, which could pass while checking only part of the pact.
  A gRPC, protobuf or Avro message interaction was skipped and is now reported.
- **An interaction needs a plugin** when its `pluginConfiguration` names one, or when it uses a
  matcher only a plugin provides, which needs every plugin the pact's metadata names.
- **The rest of the pact is verified**, and the summary counts each refusal as a failure.
- **`upgrade/pact` drops such an interaction** with an `interaction-dropped` finding naming the
  plugin, and **`verification/explain` refuses it** with the same error.

#### A component's registry credential is declared in its source, and a run reads no environment for it

A component fetched from an OCI registry used to be fetched with `JANUS_OCI_USERNAME` and
`JANUS_OCI_PASSWORD` from the environment of whatever hosted the engine. So two runs of one configuration
could differ in whether they started, and nothing in the configuration said why (ADR 0052).

- **A component declaration's `source` takes `auth`**, in the broker descriptor's shape: `basic` with
  `username` and `password`, or `bearer` with a registry `token`. A host's loader interpolates `${VAR}` in
  it as in any member, and `hooks-host`'s loader does. A `basic` credential answers a `Basic` challenge and
  is presented to a `Bearer` challenge's token service; a `bearer` one is presented as it stands.
- **Breaking: a run presents that credential and no other.** `verification/verify`, `verification/explain`
  and `consumer-session/create` no longer read the two variables. A registry that wants a credential
  nobody declared refuses with `unauthorized`, inside `component-unavailable`, and the message says the
  credential belongs in the component source's `auth`. A configuration that relied on the variables adds
  `auth: { kind: basic, username: "${JANUS_OCI_USERNAME}", password: "${JANUS_OCI_PASSWORD}" }`.
- **Refused before anything is fetched:** an `auth` whose kind the loader cannot present, and an `auth`
  on a source that is not a registry. A malformed `auth` (not an object, a member that is not a string,
  `basic` without a password, `bearer` without a token) is an invalid declaration, and the message never
  quotes it.
- **The credential is never repeated**: the frame log shows an `auth` member's `kind` alone, as for a
  broker, and no error, event or summary carries it.
- **`janus component push` and `pull` still read the two variables**, having no configuration. When given
  them, the declaration they print carries the `auth` above.

`pact_janus_component_host`'s API follows: `Oci::new` takes no credentials, `Oci::pull`/`push` and
`WasmLoader::pull`/`push` take an `Option<&Credentials>`, and `Credentials` is an enum (`Basic`,
`Bearer`) with `from_declared`.

### Features

#### `values`, `eachKey`, `eachValue` and `arrayContains` rules verify as today

A v1–v4 pact whose body used one of these four matching rules used to fail every interaction that had
one: the legacy compiler compiled each to an error. They now compile as today's verifier applies them
(ADR 0042):

- `values` and `eachValue` on an object compare every entry of the reply with the pact's value for that
  key, or the pact's first value for a key it does not give. Keys are not compared. `eachValue` uses only
  its own rules, and on an array holds every element to them.
- `eachKey` holds every key of the reply to its rules, and compares the pact's own members where the reply
  has them.
- `arrayContains` requires each variant to match some element of the reply's array, in any order; one
  element may satisfy two variants. A variant with no rules is its element compared by equality. A
  variant that nothing matched fails with today's message,
  `Variant at index 0 (…) was not found in the actual list`.

Plan grammar v1's `legacy:array-contains`, which no compiler emitted before, now runs a template against
the array's elements instead of comparing one value: its children are the array, the template, and
optionally the variant's index and element. These rules on a path, header, query, form value or XML
element are still refused. `janus upgrade` still converts them approximately.

#### `verification/explain` compiles what a run compiles, and runs captured values

`verification/explain` now prints the plan a run executes, and can run it (ADR 0050).

- **`components`**: the project's declared components, resolved as `verification/verify`'s
  `target.components` are. A component's plan fragment is now in the plan `explain` prints. A contract's
  `content-types` reach the compiler, which they did not before.
- **An interaction a run would refuse is refused**: one whose requirement, declared content type or
  contributed fragment no loaded component satisfies now fails with `component-unavailable`, as
  `verify` fails it, where `explain` printed a generic plan.
- **`values`**, with optional `encoded`: captured values by path, as a golden corpus case gives them. The
  result then carries `executed`, with `status` (`matched` or `mismatched`), `mismatches` and the executed
  `text`. They are run with the run's content detection and the actions its components contribute.
- The subject's `variant` is now in the schema. The engine already read it.

`janus explain` gains `--config <file>`, which loads a verifier configuration's components, and
`--executed` now runs its values in the engine.

#### Contracts carry generators, filled where a value is produced

A shape node's `generator` member is now `{ "name": "<component>:<generator>", "config": { … } }`, and the
engine fills fresh values from it where a value is produced (ADR 0048). The example stays what a contract
records, so recording the same interaction twice still writes the same contract.

- **`verification/verify` fills a contract's request.** After `state-setup` and before `before-request`,
  every position the request's shapes give a generator gets a fresh value, on the provider's side, from
  the same provider-state values a pact's generators read. A slot carried as octets is decoded by its
  declared type, or the type it is labelled with, and encoded back by the same one. A generator that
  produces nothing keeps the recorded value, with a `generator-failed` warning, as for a pact.
- **A mock fills its reply on every exchange**, on the consumer's side, with the variant's resolved
  state parameters as provider-state values.
- **`generator/generate`'s context has `side`**, `provider` or `consumer`. The built-in `pact`
  component keeps the example for a generator that does not run on that side: `pact:ProviderState` on
  the consumer's.
- **`upgrade/pact` writes a v1–v4 generator onto the shape at its target**, as `pact:<type>` with the
  pact's members as its `config`, where until now it dropped every one. A generator no component
  provides (`MockServerURL`), one in a text, binary or multipart body, or one whose path names no
  position is still `generator-dropped`.
- **`upgrade/pact` declares a JSON body's type** in `content-types`, with no `requires` entry, so a
  reply with no `Content-Type` is decoded as JSON, as the pact's verification decodes it. A converted
  body's evidence is the pact's octets, as text when they are UTF-8.
- **`eachKey` and `eachValue` convert as the legacy compiler compiles them.** `eachKey`'s keys follow
  the rule's own matchers and its values are left open, reported as `rule-unmapped`. `eachValue` on a
  list becomes `each-like` with `min: 0`.
- **A new finding, `pending-dropped`**, for a v4 interaction marked pending, which a contract has no form
  for.
- **The built-in form component writes a number or a boolean as its text**, as the XML component does.

#### Hooks hardened: state calls retried, hook endpoints reached as a provider is, secrets kept out of logs

- **An `http` hook sends through the engine's HTTP client**, the one a provider and a broker are reached
  through, and takes a `client` member: the HTTP client options (TLS, proxies, timeouts, HTTP version). A
  state endpoint behind the provider's mutual TLS or private CA is reached with the same options as the
  provider (ADR 0044).
- **A state call is tried three times.** At `state-setup` and `state-teardown`, an answer of 5xx, 408 or 429
  is tried again after 100 ms and then 1 s. `client.retries` changes the count, and at any other point a
  request is tried once. `timeout-ms` now bounds each attempt. A state endpoint that fails once while it
  warms up passes, as it does under today's verifier.
- **`format: pact-state-change-query`** sends a state as query parameters with no body, as today's
  `--state-change-as-query` does.
- **An `exec` hook's deadline ends everything the command started**: its process group on Unix, its process
  tree on Windows. A command that exits while a child it started holds its output open is ended at the
  deadline too.
- **A scripted hook's memory and stack are bounded**, at 64 MiB and 256 KiB. A script that allocates or
  recurses without end fails as `errored` rather than taking the engine's process with it.
- **No hook secret is logged.** The `trace` log of each frame no longer repeats a hook's `config`, its
  `run.headers` or `run.env`, or the user information in an `http` hook's URL, and a failing `http` hook's
  message no longer repeats that user information.

#### `janus verify` takes today's verifier options, and the protocol gains filters, pacts by URL and a stop

`janus verify` now takes `pact_verifier_cli`'s long spellings and environment variables wherever they
mean what they did, and the Ruby verifier's as aliases (ADR 0053; `Documentation/verifier-migration.md`
has a row for every option).

Protocol, all additive:

- **`options.filter`** on `verification/verify`: `description` and `state` (regular expressions, searched
  for), `no-state`, `consumers`. An interaction it leaves out is skipped; a run it narrows is `filtered`
  and its narrowed pacts are not published (ADR 0041). One that matches no interaction is refused with a
  new error code, `filter-matches-nothing` (category `session`), carrying the `filter` and how many
  `interactions` there were. A filter that cannot be read is `malformed-frame`.
- **A `urls` source**: `{ kind: "urls", urls, auth?, client? }`. Each pact is fetched by its URL in the
  run, is never pending, and is published to its own links when a Pact Broker served it; otherwise its
  publication is `skipped` with `reason: "not-from-a-broker"`. `options.publish` is allowed with it.
- **`options.stop-on-first-failure`**: the run ends at the first failure that fails it (a pending one does
  not), and the summary says `stopped: "first-failure"`.
- `verification.schema.json` now also lists `options.variants` and `options.variant`.

For an embedding with its own broker client: `BrokerClient` gains `fetch_pact`, and `FetchedPact` a
`publishable` member.

CLI:

- Where the provider is: `--hostname`, `--port`/`-p` and `--transport https` when there is no
  `--provider-url`, which is no longer required on its own.
- Sources: `--file`/`-f`, `--dir`/`-d`, `--url`, `--webhook-callback-url`, `-b` for `--broker-url`,
  `-n` for `--provider-name`, and `PACT_BROKER_BASE_URL`, `PACT_PROVIDER_NAME`,
  `PACT_WEBHOOK_CALLBACK_URL`. One kind of source per run; a broker URL only the environment gave yields
  to files or URLs on the command line. `--publish` works with `--url` too.
- `-s`/`--state-change-url`, `--state-change-as-query`, `--state-change-teardown` write the state hooks.
- `--filter-description`, `--filter-state`, `--filter-no-state`, `--filter-consumer`/`-c`.
- `--exit-on-first-error`, `--consumer-version-tags`, `--retries`, `--ignore-no-pacts-error`, `-t` for
  `--token`, and, on every command, `--no-colour` and `--loglevel`, whose levels gain `off` (alias `none`).

#### `upgrade/pact` converts text, binary, form, XML and multipart bodies

Until now, `upgrade/pact` converted only JSON bodies. Any other body was carried across as the pact's octets
with a `body-not-parsed` finding, and nothing was asserted about it, so the contract verified less than its
pact (ADR 0047).

- **A body a built-in component reads declares its type.** A text body that is UTF-8, a form, XML and
  multipart get a `content-types` entry for their slot and a `requires` entry for `content/text`,
  `content/form`, `content/xml` or `content/multipart`. A multipart type is declared without its
  `boundary`. Each body's shape is over that component's document, built as the legacy compiler compares
  the body.
- **Octets stay undeclared**: a binary body, or text that is not UTF-8, is shaped as bytes with a base64
  `example` and `encoded: base64`.
- **A converted body's evidence is the pact's octets**: `encoded: text` when they are UTF-8, `base64`
  otherwise, with the pact's type in `content-type`. A replay sends them as they stand.
- **A pact that names no `Content-Type` header** gets a `content-type` member in that part's `headers`
  shape: `optional`, of a `regex` that admits what the pact's check of the reply's type admits. This
  applies to JSON bodies too, so a converted JSON interaction with no `Content-Type` header now has a
  `headers` shape, and a variant space of two.
- **A number or boolean rule on text** (a path, header, query or form value, an XML attribute or text)
  converts to the `regex` v1–v4 read the text by, where it converted to `integer`, `number` or
  `boolean`, which no string satisfies.
- **`eachValue` and `arrayContains`** convert with their own rules.
- **New finding codes**: `response-body-opened` (lossy), `body-type-on-header` (judgement) and
  `rule-as-pattern` (note). `body-not-parsed` is now raised only for a body that is not what its type says.
  `request-body-opened` and `rule-narrowed` are raised for the new body kinds too.

Two behaviours change outside `upgrade/pact`:

- **The built-in multipart component** decodes a slot whose declared type names no `boundary` at the
  boundary the slot's own label names.
- **The shape compiler quotes a member name a path cannot hold**, such as `a.b`, `''` or
  `@{urn:x}id`. Such a member used to be resolved as a different member, nested or missing, so a shape
  naming one matched the wrong value.

#### `upgrade/pact` can leave pending interactions out of the contract

A Janus contract has no pending interactions: whether a provider has yet to satisfy one is the broker's
to decide, from the provider's results, as it decides a pending pact (ADR 0051). `janus verify` keeps
honouring a v4 pact's `pending` flag, as today's verifier does.

- **`upgrade/pact` reads `options.exclude-pending`.** With `true`, each pending interaction in the pact
  is left out of the contract and reported as a new finding, `pending-excluded` (`lossy`), pointing at
  the interaction and naming it. Without it, nothing changes: each is converted without its flag, and
  reported as `pending-dropped`, whose message now says how to leave it out instead. An interaction
  left out is not counted again in `interaction-dropped`.
- **`janus upgrade --exclude-pending`** sends it, for a pact whose pending interactions are known to
  fail, whose upgraded contract would otherwise fail where the pact passes.
- **Every finding points at the interaction's position in the pact.** Upgrading used to count only the
  HTTP interactions, so a finding for one after a message or plugin interaction pointed at the wrong
  one, and the suffix `duplicate-description` gave a renamed interaction in such a pact was off by as
  many.

The protocol's `upgrade.schema.json` now lists every finding code the contract-file spec does, and the
generated bindings carry the new option.

### Fixes

#### `content-type` shapes match in live runs

A `content-type` shape, or a v1–v4 `contentType` matcher, failed every live verification and mock exchange
with "no content component is loaded": only the golden corpus runner gave the interpreter a detector. Both
paths now ask the session's content registry (ADR 0049).

- On a contract, a slot's type is the one the interaction declares for it, else the `Content-Type` it
  arrived with, and only a slot with neither is detected from its octets. A reply that labels JSON
  `text/plain` fails a `content-type` of `application/json`.
- A v1–v4 `contentType` matcher detects the type from the reply's octets, whatever they are labelled, as
  today's verifier does.
- Types are compared by essence: case and parameters aside, and `text/*` matches any text type.
- The built-in components detect JSON only. A `contentType` for a binary type such as `image/png` is not
  detected yet.

## 0.3.0 (2026-10-22)

### Breaking Changes

#### Providers and brokers are reached through one HTTP client, with TLS, proxy, timeout and HTTP/2 options

Every request to a provider or a Pact Broker now goes through one client, configured by one
document, the HTTP client options (component-interfaces spec §5.6, ADR 0035). It is today's
verifier's client (reqwest on rustls, the platform's trust store, proxies from the environment),
async and thread-free.

- **Breaking: a provider request times out after 5 s by default**, as today's verifier's does. It
  was 30 s, set by the kernel on every `transport/send`. Verification's sends now carry no
  `timeout-ms`; the HTTP transport's `client.timeouts.request-ms` bounds each request, and
  `janus verify --request-timeout` sets it.
- **Breaking: a variant that fails because the transport could not send** now carries the
  protocol's `component-failed` error (spec §10.2): `message` says why, and the component and its
  own error are under `details`. They were top-level `component` and `error` members, with no
  message.
- **The HTTP transport's drive options** are now specified: `base-url` (its path prefixes every
  request), `headers` (added to every request, replacing the interaction's own of that name in any
  case) and `client`. Options the transport cannot act on fail `verification/verify` with
  `component-failed` carrying `invalid-config` and the member's pointer.
- **The HTTP client options:** `tls.ca-certificates`, `tls.client-certificate` and `tls.insecure`;
  `proxy` (`environment`, the default, `none`, or `url` with `no-proxy`); `timeouts.request-ms` and
  `timeouts.connect-ms`; `http-version` (`1.1`, the default, `2` or `negotiate`); and `retries`, for
  a client that retries. Certificates and keys are PEM text, never paths.
- **The broker descriptor's `client` is read**, with the same document. With none, a broker is
  reached through the platform's trust store and the environment's proxies. It used to be reached
  through the HTTP library's bundled roots. `retries` replaces the fixed three attempts.
- **`janus verify` options:** today's `--base-path`, `--header`/`-H`, `--request-timeout` and
  `--disable-ssl-verification`, with today's environment variables; and new `--ca-cert`,
  `--client-cert`, `--client-key`, `--proxy`, `--no-proxy`, `--connect-timeout` and
  `--http-version`. `Documentation/verifier-migration.md` maps each of today's options.
- **What a provider sees:** no `Accept-Encoding: gzip` (as today's verifier), a `User-Agent` of
  `pact-janus/<version>` unless the interaction records one, and connections kept open between
  requests.
- **Never logged:** a client certificate's key, a `headers` value, and anything the HTTP stack
  writes at `trace`.

### Features

#### Form bodies verify, through a built-in form component

A v1–v4 pact whose body is `application/x-www-form-urlencoded` now verifies as today's verifier
compares it (ADR 0038). It used to fail before anything was sent, as `component-unavailable`.

- **A built-in `form` content component** decodes a form body into the query slot's model: an object
  whose members are arrays of strings, each name once, in the order it first appears. It encodes one
  back, writing a member that is one string as one value. Every embedding registers it after text,
  and `engine/hello` lists `content/form`. A Janus contract can declare a form body and shape it as it
  shapes a query.
- **A v1–v4 form body is compared as a query string**, as today's verifier compares one: each name's
  values in order, by the rule on the name or by equality, and a name the pact lacks is a mismatch in
  a response as well as a request. `explain` shows a form body's plan as it shows a query's.
- **A form body's request generators are applied**, each to one occurrence of its name, as
  `pact_models`' handler and pact-jvm apply them. Today's Rust verifier sends them unapplied, because
  it is built without that handler, so here Janus differs from it on purpose.
- `janus upgrade` still carries a form body across with a `body-not-parsed` finding.

#### An object's members keep the order they were written in

The engine used to sort every object's members, so a mock served a CSV body's columns, or a JSON
body's members, in alphabetical order whatever order the consumer declared them in. It now keeps
the order they arrived in (ADR 0037). Order is never compared: two objects with the same members are
equal in any order, so no verdict changes.

- **A mock serves members as the shape declares them**, and so does every document generated from a
  shape: the values a contract records, which a verifier sends as a request. A content component's
  `content/encode` is handed the author's order. A decoded body keeps the order the provider sent.
- **Plans, variant spaces and subsumption go by name, as before.** A plan does not change when a
  shape or a pact is written in another order. Dimension ids, the variants selected and every
  subsumption finding are unchanged.
- **`explain --executed` and mismatch messages show a captured value in the order it arrived**, not
  sorted.
- **A contract's bytes change only where its author's order was not alphabetical.** The canonical
  writer sorts the records and a shape node's own members by name as before. It keeps an `object`
  shape's `members`, examples, `any-of` options, recorded slot content and state parameter values
  as written (contract-file spec §2.4). A contract re-recorded with this engine is a new version at a
  broker only if one of those was out of name order. Provider shapes follow the same rule.
- **Engine output built from its own records** (`--output json`, a run summary, the `variants`
  result) is no longer sorted at every depth. It is just as deterministic.
- The v1–v4 compiler compiles a body as a document rather than as JSON, which is what lets form and
  XML bodies reach it later. It changes no plan.

#### Multipart bodies verify, through a built-in multipart component

A v1–v4 pact whose body is multipart now verifies as today's verifier compares it (ADR 0040). It used to
fail before anything was sent, as `component-unavailable`. No body kind is refused any more.

- **A built-in `multipart` content component** claims every `multipart/*` type. Every embedding registers
  it after form, and `engine/hello` lists `content/multipart`. It splits a body at the boundary its
  content type names, into a document keyed by part name: one member per name, each an array of the
  parts with that name, and unnamed parts under `""`. A part is an object of `headers`, `filename` and
  `body`. Each part's body is decoded by its own type through the built-in JSON, XML, text and form
  components; any other type is bytes. It encodes such a document back, choosing a boundary when the type
  names none, so a Janus contract can declare and serve a multipart body.
- **A v1–v4 multipart body is compared part by part**, as today's verifier compares one. Each of the
  pact's parts is found by name, or by position when it has none, and a missing part or a field where the
  pact has a file fails with today's messages. Then come the part's headers, and its body: a text field as
  text with the rules at `$.<part>`, and a file by its own type, with the rules under `$.<part>`. Parts the
  pact does not have are allowed.
- **Deliberate differences**, each where today's own behaviour is accidental: a field with a type of its own
  is compared as that type, not as text; a part with no `Content-Type` is text; unnamed parts are matched
  among the unnamed ones; and a part that is itself multipart is compared as octets.
- Plan grammar v1 gains `legacy:mime-part`.
- A multipart body's request generators are still not applied, as today. `janus upgrade` still carries a
  multipart body across with a `body-not-parsed` finding.

#### Text and binary bodies verify, through a built-in text component and bytes

A v1–v4 pact whose body is plain text or binary now verifies as today's verifier compares it
(ADR 0036). Both used to fail before anything was sent: text as `component-unavailable`, and a binary
body as `contract-invalid`.

- **A built-in `text` content component** decodes any `text/*` body as UTF-8 into a string, and
  encodes a string back. Every embedding registers it after JSON, `engine/hello` lists `content/text`,
  and a component a project declares for a `text/` type still comes first. It serves only a body whose
  shape admits a string: a `text/csv` body of rows with no CSV component declared is still refused as
  `component-unavailable` when the interaction arrives, now with `reason: compile-failed`.
- **A body no content component claims is now its octets**, the document model's bytes, wherever
  the engine decodes one. It used to be matched as absent. A Janus contract whose shape expected no
  body where the provider sends one of an unclaimed type now fails, as it should. A body that a
  component claims and cannot decode is still absent.
- **A v1–v4 body is read by the pact's content type**, as today's verifier picks a matcher: JSON as
  before, a binary type (`image/*`, `application/octet-stream`, and any type that is not text) as
  bytes, and other text as a string. Each is compared by the rules on `$`, or by equality. XML, form
  and multipart bodies are still `component-unavailable`.
- **A reply whose content type disagrees with the pact's fails**, with today's message (`Expected a
  body of 'text/plain' but the actual content type was 'text/html'`), when the pact names no
  `Content-Type` header. Plans show this as a new legacy action, `legacy:body-type`, so `explain`
  prints it for a JSON pact with no `Content-Type` header too. The reply's body is decoded as the
  pact's type whatever it is labelled.
- **A bytes mismatch names the first byte that differs.**
- **A corpus case may carry bytes**: `encoded` maps a path in `values` to `base64` (plan-grammar
  spec §6.1).

#### Verifying a v1–v4 pact applies its request generators, and a state-change reply's values reach them

A pact's request generators are now applied before the request is sent, as today's verifiers apply
them: the path, each header and query parameter, and every JSON body value a generator names. Before,
the recorded example was sent as it stood, so a pact whose path came from a `ProviderState`
generator asked the provider for the consumer test's id rather than the one the state handler had
just created (ADR 0034).

- **Values come from the built-in `pact` generator component**, through `generator/generate`, with
  `pact_models` producing them as today's Rust verifier does: `RandomInt`, `RandomDecimal`,
  `RandomHexadecimal`, `RandomString`, `Regex`, `Uuid`, `Date`, `Time`, `DateTime`, `RandomBoolean`
  and `ProviderState`. `MockServerURL` is the consumer side's and is left alone. Response generators
  are not applied, as today.
- **Provider-state values.** A `state-setup` hook result may carry `values`. A `pact-state-change`
  `http` hook reads a JSON object reply with no `outcome` member as those values, so a state endpoint
  that returns `{"id": 1234}` today feeds `${id}` unchanged. A `ProviderState` generator reads every
  state's `params`, then those values, a later one replacing an earlier one.
- **Changed for `pact-state-change` hooks:** a 2xx reply that is not JSON, or JSON that is not an
  object, is now `ok`, as today's verifiers read it. It was a `hook-result-invalid` failure.
- **A generator that produces nothing** (a `ProviderState` expression no state provided, say) leaves
  the recorded value, as today, and the run emits a `verification/warning` with code
  `generator-failed` naming the target, the generator and why. The verdict is unchanged.
- **New in the specifications:** the `values` member of a hook result and of the hook report
  (`report-data` only), the `example` and `provider-state` members of a generator's context, the
  component error codes `unknown-generator` and `generate-failed`, and the `verification/warning`
  event's row in the protocol's event table. All additive within v1.

A contract upgraded from such a pact still drops its generators (`generator-dropped`); that is
staged plan S1.19.

#### XML bodies verify, through a built-in XML component

A v1–v4 pact whose body is XML now verifies as today's verifier compares it (ADR 0039). It used to
fail before anything was sent, as `component-unavailable`.

- **A built-in `xml` content component** claims `application/xml`, `text/xml` and every `+xml` type.
  Every embedding registers it after JSON and before text, and `engine/hello` lists `content/xml`. It
  decodes an XML body into a document keyed by element name. The body is an object holding the root
  element under its local name. An element is an object of `#namespace` (when it has one), `@<name>`
  per attribute, `#text`, and an array of children per local name. It encodes such a document back, so
  a Janus contract can declare and serve an XML body.
- **A v1–v4 XML body is compared element by element**, as today's verifier compares one. Rules are found
  by pact's own XML paths (`$.a.b['@c']`, `['#text']`). Repeated children under a type rule are compared
  against the pact's first, a request has no attribute or child the pact lacks, and text is compared
  even when the pact has none. Every pact-specification case now runs, none skipped.
- **One deliberate difference:** an element the pact puts in no namespace matches its local name in any
  namespace, as the pact specification says. Today's verifier and pact-jvm compare the qualified name.
- **A type rule on text reads the text**, as today's verifier does. `integer`, `number`, `decimal` and
  `boolean` on a path, header, query or form value, or on XML attribute and text, accept text that parses
  as one. They used to fail it, because they compared the value's kind.
- **An XML body's request generators are applied**, to attributes and text, as today's verifier applies
  them.
- Plan grammar v1 gains `legacy:xml-children`, `legacy:xml-only-children` and `legacy:parses-as`. A
  path segment can be quoted, `['@{urn:names}name']`, for a member name a plain segment cannot hold.
- `janus upgrade` still carries an XML body across with a `body-not-parsed` finding.

### Fixes

#### Generated bindings list members in the order the schema writes them

`janus-sdk-gates bindings generate` now emits each type's members in the order its schema declares them,
however it was built. Before, the order depended on what was compiled alongside the generator. Built on its
own it sorted them alphabetically. Built in the release, next to the CLI, it inherited the kernel's
member-order setting (ADR 0037) and kept the schema's order. The bindings a release's gates generated
therefore differed from the ones checked in, so the release's bindings gate failed.

An SDK that checks its bindings against a release's gates regenerates them once. The change is member
order only: the JVM classes' `@JsonPropertyOrder` follows the schema, so they serialise members in that
order. No type, member or constant changes.

#### A bytes value in a plan shows its base64

`explain`, the executed form and mismatch messages printed a bytes value as `<8 byte(s)>`, which said
nothing about what the bytes were. They now print `BYTES(8, iVBORw0KGgo=)`: the length, then the
base64, as the forked engine did. The base64 is cut after 48 characters (the first 36 bytes) and ends
in `...` when it is longer (plan-grammar spec §3.1). A corpus that pinned the old form must accept the
new one.

#### A contract's executed plan shows the response it was scored on, not a failing request

When verifying a Janus contract, the `verification/executed-plan` event (and so
`janus verify --explain-failures`) rendered the whole interaction's plan. Its request half was
evaluated against nothing, because only the provider's reply is there to resolve against, so every
request check showed as an error, such as `ERROR(Expected <absent> to equal 'GET')`. The verdict was
never affected: only the response is scored.

The event's `text` is now the scored `response` subtree, the same as for a v1–v4 pact, so the plan
you read is the one that decided the verdict. Its `plan` member is unchanged.

#### Every HTTP request to a provider or a broker is logged in full at `debug`

At `--log-level debug` (or `RUST_LOG=debug janus-engine`), the HTTP library logs each request's
URL as `http://host:port/******`. It hides every path below `trace`, because it cannot know which
ones carry a secret. Now the HTTP transport and the broker client log each request themselves:
the method and the whole URL when it is sent, then the status when it is answered, or the error
when nothing answered. That makes debugging which requests ran possible without `trace`.

- **What is shown.** Provider paths come from the contract, and broker URLs are the broker's own
  links, so neither is hidden. User information in a URL (`http://user:password@…`) is removed,
  as everywhere else the engine logs a URL, and headers are not logged.
- **What does not change.** The HTTP library's own lines are still capped at `debug`, so its
  `trace` dump of request bytes, `Authorization` header included, is never logged.

#### A request the provider could not answer is reported once, on one line

`janus verify` printed a variant's `component-failed` error with its details as a JSON line under
it, and the engine logged the same failure again at `warn`, in Rust's debug format. Now the line
ends with the component's own code in brackets (`… frame with invalid size [transport-failed]`),
which is what the JSON added, and `--output json` still carries the whole error. The engine logs a
variant's failure, a transport failure or mismatches, at `debug`, since the run's result event
already reports it.

## 0.2.0 (2026-10-20)

### Breaking Changes

#### The `janus` command line is rebuilt: `--json` becomes `--output json`, and `upgrade --out` becomes `--write`

Two flags are renamed, so a script that runs `janus` must change to move to this version:

- `--json` is now `--output json` on `verify`, `check`, `upgrade`, `component push` and
  `component pull`. `--output text` is the default. This frees `--json <file>` for parity with
  `pact_verifier_cli`, where it writes a report to a file.
- `janus upgrade --out <file>` is now `--write <file>`, so it cannot be confused with `--output`.

The exit codes are unchanged: `0` done, `1` the subject failed, `2` the command could not run.
Everything else is new or fixed:

- **Stricter command lines.** A usage error now names the problem and shows the usage of the
  subcommand it is about, and exits 2. Mistakes that used to be ignored are now refused: a value
  that is not `warn` or `block`, an `--as-of` that is not a calendar date, an `--index` that is not
  a number, extra arguments, and flags that cannot be used together (`explain --spec` with `--index`
  or `--variant`; `explain --plan` with `--executed`; `upgrade --output json` with `--write` or
  `--quiet`). `component push --help` and `component pull --help` now exist.
- **`janus verify` no longer gives up on a slow run.** It used to fail with "the run stopped
  producing events" whenever 10 seconds passed with no events, such as a slow provider or a slow
  state hook. It now waits until the run ends.
- **A closed pipe is no longer a crash.** `janus verify ... | head` used to panic when `head`
  stopped reading. The command now runs to its end and exits with its own answer.
- **Help and usage errors name the program `janus`** on every platform; on Windows they said
  `janus.exe`. The same goes for `janus-engine`.
- **`janus check --output json`** now prints the "no provider shape supplied for ..." warning on
  stderr, as the text form always did.
- **New:** `--log-level <error|warn|info|debug|trace>` on every command (`RUST_LOG` still works when
  it is absent), `janus --version`, and `janus completions <shell>` for bash, zsh, fish, PowerShell
  and elvish. Logs are coloured only on a terminal, and never when `NO_COLOR` is set.
- **`janus-engine --version` and `--help`** now answer instead of waiting on stdin. SDKs start it
  with no arguments, as before. A frame whose `Content-Length` claims more bytes than the host sends
  is now an end-of-file error, not an allocation of the claimed size.

### Features

#### The engine protocol specifies how to verify from, and publish to, a Pact Broker

ADR 0032 decides that the broker client is the engine's, so no host speaks the broker's API. The
engine-protocol spec now specifies it, and the generated bindings carry the new types
(`BrokerDescriptor`, `BrokerAuth`, `ConsumerVersionSelector`, `PublishResults`, `VerifyOptions`,
`PublishContracts`):

- `verification/verify` takes a source of kind `broker` (the broker, the provider, consumer version
  selectors, the provider's branch and tags, pending and work-in-progress pacts), fetched inside the
  run, and an option `publish` that sends each pact's result to that pact's own link. A pending
  pact's failures do not fail the run.
- `broker/publish-contracts` publishes a consumer's contracts, reporting on a stream.
- The `broker` capability says whether an engine has a client. New event kinds:
  `verification/fetched`, `verification/published`, `broker/published`, `broker/finished`. New error
  code: `pacts-not-found`.

No engine has a broker client yet (staged plan S1.3 builds it), so every engine answers a `broker`
source, a `publish` option and `broker/publish-contracts` with `component-unavailable` naming
`broker`. A `broker` source used to be answered with `operation-unsupported`.

#### `janus explain` and `janus verify --explain-failures` colour the plan on a terminal

The plan's structure is in bold, its paths and values are coloured, and every `BOOL(false)` and
`ERROR(...)` is red, so the branch that failed is easy to find. Colour only wraps the text: with it
removed, the output is the plan-grammar spec's text form exactly (spec §3.3, new).

- **Where colour appears.** On a terminal, unless `NO_COLOR` is set or `TERM` is `dumb`. Anywhere
  when `CLICOLOR_FORCE` is set, which is how a CI log that shows colour asks for it. `NO_COLOR`
  wins over both. A pipe or a file gets plain text, as before.
- **What does not change.** The engine still renders the plan, and nothing in the protocol is
  styled: the `verification/explain` result and the `verification/executed-plan` event carry the
  same text as before. `explain --plan` prints JSON, never coloured.
- **For Rust hosts.** The kernel reads a rendered plan back as tokens (`plan::tokens`), so a host
  can colour it without parsing the text form itself. Every text form the corpora record reads
  back with nothing lost.

#### A run's summary counts its variants per source document

The summary `verification/finished` carries gains `pairs` (ADR 0033). It is positionally parallel
to `consumers` and `providers`, with one entry per source document: its `consumer` and
`provider`, its `status` (`verified`, `failed`, or `incomplete` when the run stopped before
finishing it), and its own `variants` counts. The run-level `variants` is their sum. A pending
document that failed is `failed` in its entry; whether that fails the run is still the run's
`status`.

`subsumption/decide`, and so `janus check`, now reports a pair's own variant counts for a run over
several contracts. When a run verified two documents between the same consumer and provider, for
example a consumer's `main` and deployed pacts fetched from a broker, their counts are added up.
Before, the counts were left out for any run over more than one pair. A summary from before `pairs`
is read as it was.

#### `janus publish` and `broker/publish-contracts` publish a consumer's contracts to a Pact Broker

`broker/publish-contracts` is built (engine-protocol spec §8.7, ADR 0032). It publishes Janus
contracts or v1–v4 pacts, all for one consumer, for a consumer version with its branch, tags and
build URL. Each is published as the broker's `pact` specification, so any Pact Broker since 2.86.0
accepts it.

- **The documents are checked first.** Every document is read as `verification/verify` reads an
  inline one, so the operation refuses a document it could not verify
  (`contract-version-unsupported` or `contract-invalid`, with pointers under `/contracts`) and
  documents from two consumers (`malformed-frame`) before it contacts the broker.
- **What is published is the document as sent,** including members this engine does not know,
  written as compact JSON with its members in name order.
- **The outcome arrives on a stream**, in a broker session: one `broker/published` per contract,
  then `broker/finished`, which carries the broker's URL (without credentials), the consumer and its
  version, every contract's outcome, and the broker's notices as text. The broker accepts or refuses
  a publication whole. A refusal, or a broker that cannot be reached, is `failed`, with
  `component-failed` naming `broker` and carrying the client's own error.
- **Compression only where the broker reads it.** The client gzips a publication only when the
  broker sends `Accept-Encoding: gzip` (RFC 7694), and resends it uncompressed after a 415. Today's
  Pact Broker sends no such header and refuses a gzipped body, so publications go uncompressed.

`janus publish <contract-or-pact>...` drives it, with the options of today's `pact-broker publish`:
`--consumer-app-version` (`-a`), `--branch`, `--tag` (`-t`), `--build-url`,
`--broker-base-url` (`-b`), `--broker-username` (`-u`), `--broker-password` (`-p`) and
`--broker-token` (`-k`). The broker URL and credentials can also come from `PACT_BROKER_BASE_URL`,
`PACT_BROKER_USERNAME`, `PACT_BROKER_PASSWORD` and `PACT_BROKER_TOKEN`. Documents from several
consumers are published one consumer at a time. The exit status is 0 when everything was published,
and 2 otherwise. Not carried over: `--auto-detect-version-properties`, `--tag-with-git-branch` and
`--merge`.

#### A release carries the gates an SDK's CI runs, and the specification they run against

Two kinds of file join every release:

- `janus-sdk-gates-<triple>` (`.exe` on Windows) for the same five targets as `janus-engine`: one
  statically linked binary holding the conformance check (`janus-sdk-gates conformance check
  <report.json>`), the thinness audit (`janus-sdk-gates thinness check`, which reads the
  `thinness.json` at the SDK's root) and the binding generator (`janus-sdk-gates bindings generate
  --only <language> --sdk <dir>`).
- `janus-spec.tar.gz`: the conformance suite, the corpora, the behavioural specification, and the
  schemas the bindings are generated from, at the paths they have in the repository. Point the gates
  at it with `--spec <dir>`, and an SDK's conformance driver with `JANUS_SPEC`.

An SDK that pins this version can run its conformance, thinness and bindings gates from the release
alone, with no checkout of this repository and no Rust toolchain. Both are listed in `SHA256SUMS`
and attested like every other file.

#### `janus verify` and `janus-engine` verify from a Pact Broker, and publish results back

Both native binaries now have a Pact Broker client (ADR 0032), and `engine/hello` declares the
`broker` capability. The offline WASM artifact still has none, and still answers every
broker-facing request with `component-unavailable` naming `broker`.

- **`verification/verify` takes a source of kind `broker`.** The run fetches the pacts the
  consumer version selectors choose, with pending and work-in-progress status, and reports them as
  a `verification/fetched` event before verifying anything. A fetch that fails ends the run with
  `component-failed` naming `broker`, carrying the client's own error (`broker-unreachable`,
  `broker-refused`, `broker-response-invalid`). Selectors that select nothing end it with
  `pacts-not-found`, unless the source says `require-pacts: false`.
- **Pending pacts and pending interactions don't fail the run.** This covers a pending pact, a
  work-in-progress pact, and a v4 interaction marked `pending` from any source, including a file.
  Their results say `pending: true` and are still counted in `variants`. A broker run's summary
  says per source whether it was `pending`.
- **`options.publish` publishes each pact's result to that pact's own link** when the run ends,
  keyed by the broker's interaction ids, which now ride in each interaction reference as `id`. It
  records the provider version's branch and tags first. Each publication is a
  `verification/published` event, and the summary lists them as `publication`. A publish that
  fails is reported and does not change the verdict. `publish` with an `inline` source is refused
  as `malformed-frame`.
- **Every link is fetched from the broker you named**, as today's verifier does. Of a link the broker
  writes, only the path and query are kept, and the broker URL's context path is added when the link
  lacks it. So a broker behind a proxy that writes links with an internal host name still works.
- **The client retries only what waiting can fix.** A broker that does not answer, or answers
  with a 5xx, is tried three times in all, with a short backoff. A 429 is retried only when it carries
  `Retry-After`, after that delay. PactFlow answers 429 for a plan's limit, and no retry gets past
  that. A `Retry-After` longer than 10 seconds is reported at once, with the delay, rather than waited
  for. Everything else the broker refuses is reported at once as `broker-refused`.
- **`janus verify` has today's broker options**, spelled as `pact_verifier_cli` spells them:
  `--broker-url`, `--provider-name`, `--consumer-version-selectors`, `--enable-pending`,
  `--include-wip-pacts-since`, `--provider-version`, `--provider-branch`, `--provider-tags`,
  `--publish`, `--build-url`, and `--user`/`--password`/`--token`, which also answer to
  `--broker-user`/`--broker-password`/`--broker-token`. The credentials can also come
  from `PACT_BROKER_USERNAME`, `PACT_BROKER_PASSWORD` and `PACT_BROKER_TOKEN`. As with today's
  verifier, pending and work-in-progress pacts are asked for only when selectors are given.
- **Short flags, and help for a bare command.** Every command's `--log-level` is also `-l`, as
  today's verifier spells it, and `janus verify`'s `--provider-url` is also `-u` and its `--output`
  `-o`. Today's verifier reads `-u` as a pact URL (`--url`), so a moved script that passes one gets
  a usage error. Every command that takes arguments (`verify`, `check`, `explain`, `upgrade`,
  `publish`, `component push`, `component pull`, `completions`) prints its help when given none,
  and exits 2 as before.
- **No credential is logged, at any level.** At `trace`, `janus` and `janus-engine` log every frame
  with each broker descriptor's `auth` reduced to its `kind`, and each `url` without user
  information. The HTTP library's own logging stops at `debug`, where it redacts headers, because
  its `trace` output dumps raw request bytes, `Authorization` included. Below `debug` it follows
  the level asked for, like everything else.

What the ledger says: every broker fixture now agrees with today's verifier, including what each run
publishes, and so does the compatibility suite's pending v4 interaction. The disagreements left are
plain-text, XML, form, multipart and binary bodies (S1.6), a repeated response header (S3.3), and a
provider-state call that is not retried (S1.9).

### Fixes

#### A verification run that crashes still ends its event stream

If a bug stopped a verification run part way, its stream never sent a terminal event, so a host
polling `events/poll` until `last: true` waited forever. The run now ends with
`verification/finished`, carrying `status: failed` and an `error` member: an `EngineError` with code
`internal` and the panic's message. The tallies died with the run, so `variants` and `failures` may
be absent from that summary (engine-protocol spec §9.6).

A host can rely on this: poll until the terminal event, with no timeout of its own. An empty poll
means nothing happened within `wait-ms`, never that the run is gone.

#### A regex in a v1–v4 pact matches the whole value, as it does in today's verifiers

Janus matched a pact's regex anywhere in the value, so `\d{4}-\d{2}-\d{2}` accepted
`2026-07-30 or later`, and `PENDING|SHIPPED` accepted `NOT SHIPPED`. Today's verifiers match it
against the whole value (pact-reference and pact-jvm both do), and now Janus does too:

- **Verifying a v1–v4 pact**, the pattern is compiled anchored, `^(?:…)$`, and `explain` and the
  executed plan print it that way. A provider that passed only because a pattern matched part of a
  value now fails, as it does under today's verifier.
- **`janus upgrade`** writes the anchored pattern into the contract, so a converted pact keeps its
  verdicts. A contract upgraded by an earlier release still has the unanchored pattern, which a
  shape matches anywhere; upgrade it again to get the pact's behaviour.

A regex in a shape (`"shape": "regex"`) is unchanged: it matches anywhere, and an author anchors
it with `^` and `$` (shape-language spec §4.2).

#### The verifier fixture corpus gains broker fixtures

`corpora/verifier/` in `janus-spec.tar.gz` now holds fixtures verified from a Pact Broker. Their
pacts are published to a real broker first, and both verifiers fetch them from it and publish their
results back. They cover the sample provider's own pact, pending and not-pending pacts, a
work-in-progress pact, a pact the consumer version selectors do not fetch, and the
pact-compatibility-suite's four broker scenarios, which were skipped until now. A fixture's `broker`
member says how its pacts are published, what was verified before the run, and the options the
verifiers get. `disagreements.json` gains keys `<pact>/published` for what a run published, and a
verdict `absent`.

Running them takes the Pact Broker pinned in `tools/verifier-diff/pact-broker.json`, which is not
in the archive: anything reading the corpus from the archive alone should skip a fixture with a
`broker` member. Nothing an SDK runs today reads the corpus.

What the ledger says about this release: `janus verify` cannot fetch from a broker or publish
results yet (S1.3). Every broker fixture is a disagreement until it can.

#### `janus-spec.tar.gz` carries the verifier fixture corpus

The specification archive gains `corpora/verifier/`: pacts with the providers that answer them, and
`disagreements.json`, every verdict on which `janus verify` and today's `pact_verifier_cli` (1.3.5)
are known to disagree, each with the plan item that accounts for it. The fixtures include the
pact-compatibility-suite's HTTP provider scenarios, V1 to V4 (Apache-2.0). Nothing an SDK runs today
reads it. A provider integration (staged plan S1.15) will be held to it.

What the ledger says about this release, for anyone deciding whether `janus verify` can replace
today's verifier yet: plain-text, XML, form, multipart and binary bodies are not verified (S1.6); a
pending interaction fails the run (S1.3); a repeated response header is judged as one list (S3.3);
and a failed provider-state call is not retried, where today's verifier retries it eight times
(S1.9).

## 0.1.0 (2026-10-12)

### Features

#### The first release

`janus-engine` and the `janus` CLI as statically linked binaries for five targets, and the engine as a WASM component for offline operations (`janus-engine.wasm`), with a `SHA256SUMS` file and build provenance attestations.

The engine speaks protocol version 1 and reads plan grammars `v1` and `v0`.
