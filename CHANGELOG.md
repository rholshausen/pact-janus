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
