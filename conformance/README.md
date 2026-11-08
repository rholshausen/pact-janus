# The SDK conformance suite

Executable-specification artifact #3 (plan task 6.4). One corpus of cases, run by every Janus SDK,
in that SDK's own language, against a pinned engine. [ADR 0017](../Documentation/decisions/0017-sdk-conformance-is-suite-passing-not-prose-matching.md)
makes this the definition: **an SDK is conformant, for a pinned engine version, when every
behavioural-specification primitive it implements passes every case its `conformance` list names.**
Not when its output matches another SDK's, and not when a maintainer reads the prose and agrees
with it — the suite is the only artifact that can fail a build.

Grown from [`pact-compatibility-suite`](https://github.com/pact-foundation/pact-compatibility-suite),
whose arrangement this keeps (one shared corpus, a thin per-language driver) and whose medium it
does not: a case here is **data**, not a Gherkin sentence each language re-reads. Prose steps put the
drift back where the suite is supposed to remove it — in the step definitions, one set per language,
each free to read "the header name is lower-cased" its own way. A case's expectation is the document
itself.

## 1. What is here

```
schemas/v1/conformance-case.schema.json   what a case is (the case's own schema)
cases/translation/                        what a DSL chain builds — no engine anywhere
cases/lifecycle/                          what the SDK sends, and what it does with the answers
cases/variants/                           which variants the closure runs on, and what a failure does
cases/live/                               the real engine: derived dimensions, contract content, and a provider run
cases/provider/                           a provider verification: what the SDK sends, and what its handlers answer
cases/facade/                             a compatibility facade's spellings, each a test in the facade's language
facades.json                              each facade, the language whose SDK ships it, and the notes that classify it
```

The first four groups are SDK spec §7's four categories, which ADR 0017 fixes as the minimum
coverage: DSL→interaction-spec translation, session lifecycle, variant iteration, and
contract-output equivalence. `provider` is the provider side (staged plan S1.14, ADR 0055): a
verifier, its state handlers and request filters, and one `verify`. `facade` is the compatibility
facades (staged plan S2.6, [ADR 0076](../Documentation/decisions/0076-write-a-facades-conformance-cases-as-tests-in-its-own-language.md)):
pact-js's API over the TypeScript SDK and pact-jvm's over the JVM SDK, one case per group of their
migration notes' rows.

Every case names, in `covers`, the conformance ids from
[`behavioural-spec.json`](../Documentation/specs/sdk-specification/behavioural-spec.json) it is the
scenario for, and in `why` the specification sentence it pins. The checker holds the corpus to both:
an id no case covers is a hole in what "conformant" guarantees, and fails the build here rather than
being discovered later by two SDKs quietly disagreeing.

## 2. Running it

Each SDK runs the corpus as part of its own test suite and writes a report; the checker reads the
reports.

```sh
cargo test -p pact_janus_conformance          # the corpus against its schema and the spec's ids
cargo run -p pact_janus_conformance -- lint   # the same checks, with a coverage summary

cd sdks/typescript && npm test                # writes target/conformance/typescript.json
cd sdks/jvm && ./gradlew test                 # writes target/conformance/jvm.json

cargo run -p pact_janus_conformance -- check target/conformance/*.json
```

`check` fails when a report is missing a case, when a case failed, or when a report names a case
this corpus does not have. CI runs all of it.

The drivers are deliberately small and live with each SDK's tests, not in its published surface:
[`sdks/typescript/test/conformance/`](../sdks/typescript/test/conformance) and
[`sdks/jvm/sdk/src/test/java/io/pact/janus/sdk/conformance/`](../sdks/jvm/sdk/src/test/java/io/pact/janus/sdk/conformance).
A driver knows only how its language *spells* a primitive — `each-like` is `eachLike` in TypeScript
and `Shapes.eachLike` on the JVM. It never decides what a primitive means; that is what the case is
for, and a driver that "helps" a case pass has broken the one property the suite has.

## 3. Writing a case

A case is one JSON document under `cases/<category>/<name>.json`, with `id` `"<category>/<name>"`.

**The DSL, as data.** `interaction` (and a step's `execute.interaction`) is the chain to replay:
`description`, `given`, `request`, `response`, exactly the behavioural spec's primitives. A chain
with `message` or `message-request` is a message chain instead (ADR 0068): `message` and
`message-request` are `message-contents` and `message-request`, each with `metadata` and `body`, and
the driver builds it with `message` in place of `interaction`. A value
where a shape is expected is a **template**: plain JSON is written as itself and compiles by the
`literal` rule, and an object carrying a `$` member is a shape-helper call instead —

```json
{ "$": "each-like", "args": [{ "$": "string", "args": ["SKU-1"] }], "options": { "min": 1 } }
```

`$` is the primitive's id in the behavioural specification, `args` its arguments in signature order,
`options` its options bag. `{ "$": "literal", "args": [ … ] }` escapes a plain object that would
otherwise be read as a call. A plain map carrying a `shape` member is still a plain map — that is
`translation/literal-rule`'s whole point, and it is why the escape hatch is `$`.

Two rules for a case author, both because a case must mean the same thing in every language:

- **Do not rely on a member name that a language may reorder.** Documents are compared by value:
  object members are a set, arrays are in order. A language cannot always preserve the order its
  author wrote (6.3's report §2.1), and ADR 0017 compares content, not order.
- **Do not write a case for something the specification does not say.** A case is a scenario for a
  sentence, and `why` quotes it. An id whose behaviour the spec leaves open is a spec change first
  (§7).

**Against a scripted engine** (`lifecycle`, `variants`): the case's `engine` member is the whole
engine. `variants` is what `consumer-session/variants` returns, `contract` what `finalise` returns,
`withhold-contract` makes it return none, `results` replaces the default results, `reports` maps a
variant id to the result `report-variant` answers with for it, `served` maps a variant id to the
`parts` `serve-variant` answers with for it (a message's, ADR 0067), and `errors` maps
an operation to the error document it answers with instead — with `after: n` on that error, the
operation answers `n` calls the default way first, which is how a case writes an engine that dies
partway through the variant loop. Everything unset is answered the default way: `engine/hello` agrees protocol version 1, `create` returns session `s-1`, `add-interaction`
returns `i-1`, `i-2`, …, `start-transport` returns an HTTP endpoint, `variants` returns one variant
`base`, `report-variant` answers `verified` for a report of `passed` and `failed` for any other, with
the report as `test`, and `finalise` returns one verified result per interaction and a small contract. Every SDK
runs as consumer `web-app` and provider `orders-api`, so the contract file is always
`web-app-orders-api.janus.json`.

**Against the real engine** (`live`): no `engine` member; the driver starts the engine `JANUS_ENGINE`
names. A closure's `exchange` is the request the consumer sends to the mock, once per variant,
except on the variants its `skip-on` names; a closure that `fail-on` names fails after its exchange.

**A provider verification** (`provider`, and a `live` case with a `verifier`): `verifier` is the
verifier as data. The driver writes each of its `pacts` to a file and names the files, writes
`config` to a file and names it by a path relative to the working directory, and declares each
state handler, request filter and message producer in order, recording every call. A state
handler's `setup` and `teardown` say what it does — `returns` an object, `throws` a failure with that
message, or says it cannot reach the state with an `unsupported` reason — and a request filter returns
the request it was given with the slots in `set` replaced, or `throws`. A message producer (ADR 0056)
`returns` a message, whose body is one of `json` (a value the language holds as its own), `text` (a
string) or `bytes` (the language's bytes, written in the case as base64), with a `content-type` and
`metadata` when given, or `throws`. `transport` is the verifier's option of that name, as given.
`verify: false` stops before `verify`.
Against the scripted engine, unset members are answered the default way: `engine/hello` declares
`config-loading`, `config/load` returns `{ "version": 1, "hooks": {} }`, `verification/verify` returns
session `vs-1` and stream `s-1`, and the first `events/poll` returns every event at once: one verified
result for `get an order`, then `verification/finished` with a verified summary. `events` and
`summary` replace those. Before it answers that poll, the engine calls each hook `invoke` lists, as a
real engine would during the run: it finds the entry by point and name in the `verify` frame's
`target.hooks` and POSTs the `context` to its url with its headers — with no `authorization` when
`authorised` is false. A live provider case has a `provider` instead, which the driver serves on the
loopback interface, and the verifier's `provider-url` is the driver's. Its `routes` may be empty: a case that verifies only
messages has no request for a provider to answer.

### 3.1 A case with a subject

Some spellings cannot be data: a compatibility facade's, which exist in one language only and lean on
its annotations, lambdas and overloads, and a language's own types, such as a pattern compiled with
flags. A case for one names a **subject**, a test written in that language, in a file beside the case
(`subjects`: language → file name; ADR 0076). Its expectation stays data, and stays the corpus's.

- A **TypeScript** subject is a module whose exported async functions are its tests. The driver calls
  each test the case names, in the case's order, with a context `{ dir, engine? }`: the directory the
  run's contracts go to, and, when the case has an `engine` script, that scripted engine, which a
  facade that takes an `engine` option is handed as a user's test hands it one. The module imports the
  SDK by its package name, `pact-janus`, `pact-janus/pact` or `pact-janus/verifier`, which Vitest
  resolves to this tree's sources. A case that expects `engine-log` runs in a child Vitest, whose
  standard error is the engine's too, with neither `RUST_LOG` nor `LOG_LEVEL` set.
- A **JVM** subject is a JUnit Jupiter test class in the default package, its file named after it. The
  driver compiles it against the test class path and runs it through the JUnit Platform, with
  `pact.rootDir` naming the run's contract directory, where pact-jvm's users name theirs.

A subject is written as a user of the thing under test writes a test, from the row or the sentence it
covers. A subject that does what no user would, so that its case passes, is the driver that helps of
§2, and fails review.

A **facade case** (`category: "facade"`) names its `facade`, a name `facades.json` gives, and has one
subject, in the language that ships it. Its `covers` is empty, and `rows` names the migration-note rows
it is the scenario for, each by its note and its first cell, verbatim. The lint reads the notes: a row
a note gives the verdict `kept` that no case names fails it, as an uncovered conformance id does, and
so does a row a case names that its note does not have. A case may name `adapted` and `dropped` rows
too. Rewording a row's first cell means changing the cases that name it, in the same commit.

What shows a row `kept` is the content the facade recorded (ADR 0076 decision 4): the case pins each
contract the run wrote, by interaction content, reviewed against the row, so a rule that converted with
a judgement or a loss fails it. A recording whose example the run makes afresh, a date from today, is
checked as `written` and no more, and the case says so in `note`.

Any other case with subjects has one per language that runs the corpus, and a driver fails it when its
language has none: `translation/regex-flags-refused` is one.

## 4. What a case can expect

Per step (`expect` inside a step):

| Member | Means |
|---|---|
| `outcome` | `"ok"`, or the failure: `execute-failed` (with the `variants` it must name), `contract-withheld` (with the `interactions` it must name), `engine-error` (with its `code`) |
| `closure-calls` | the variant ids the closure ran on, in call order |
| `received` | for a message, what the closure was handed on each call, in call order, on the members each entry names: `json` (the body read as JSON), `text` (read as UTF-8), `content-type` (`null` for none), `metadata`, and `request`, a synchronous message's, read the same way |
| `contract` | `written`, the file's exact `text`, or its `content` |

Per case (`expect` at the top level): `spec` (the whole built document), `at` (JSON pointer → value)
or `refused` (the chain never becomes a document, because the DSL refuses it at the call — ADR 0019)
for translation; `ops` (every operation sent, in order), `frames` (operation → members its request
body must carry), `engine-closed`, `dimensions` (dimension ids the engine's variants must assign)
and `variant-count`.

For a case with a subject, also: `tests`, every test the subject has, each passing or, with `fails`,
failing with an error whose message holds each of its strings (a JUnit test is named by its method, a
dynamic test by its display name, and a test the case does not name fails it); `suite`, the subject
failing beyond its tests, as a JUnit class does in an extension's `afterAll`; `contracts`, each file the
run wrote, by name, `written` and `content` as a step's `contract` compares them, a file the case does
not name failing it; `output`, strings the run's `stdout` and `stderr` hold, the subject's and the
SDK's; and `engine-log`, strings the engine's log holds, its standard error or the events the SDK hands
its language's logging, each written `<LEVEL> <message>`. Against a scripted engine, `ops`, `frames`,
`hooks` and `invocations` are a provider case's.

For a provider case, also: `outcome` (how `verify` ended: `"ok"`, `verification-failed` with the
results it names as `failed`, `engine-error` with its `code`, or `refused`, the SDK refusing the call
before any frame), `result` (its `status`, and its `results` by value), `hooks` (the `verify` frame's
`target.hooks`, or `null` for none), `config-loaded` (`config/load` carried the file's absolute
path), `invocations` (what the SDK's endpoint answered each `invoke`: the status, and the result by
value), `handler-calls` (each handler call, compared on the members the case names), and
`endpoint-closed` (the endpoint refuses connections once `verify` has ended). In `hooks`, a url on
the loopback interface is written `loopback:<path>` and an `authorization` header carrying a bearer
token of at least 32 hex digits is written `Bearer <token>`: the port and the token differ on every
run, and what a case pins is that they are there.

`refused` checks only that no document was produced. *How* a language refuses — a thrown error, a
failed build, a compile-time type error backed by a runtime check — is the style guide's business.

`outcome` is a vocabulary, not a type name: each language maps it to its own failure —
`execute-failed` is `VariantsFailedError` in TypeScript and `ExecuteFailedException` on the JVM. A
language whose failures are not exceptions maps it to whatever failing a test means there.

`contract.content` compares **interaction content only** (ADR 0017): each interaction by
description, on the members the case names — `states`, `parts`, `selection` (contract spec §5–§6).
`metadata` and the parties are never compared. Bytes are compared across languages instead: the
engine writes a contract and an SDK writes its text unchanged (ADR 0063), so a live case's contract
is the same file whichever SDK wrote it, and `check` fails when two reports' digests differ (§5).

## 5. The report

A run writes `{ "$format": "janus-conformance-report/1", "sdk": …, "cases": [ { "id", "status",
"covers", "detail"?, "contract-sha256"? } ] }`. `status` is `passed` or `failed`; a failed case carries
the `detail` the checker prints. A live case that wrote a contract carries the SHA-256 of its bytes,
in lower-case hex, and `check` fails when two SDKs' reports give one case different digests
(ADR 0063 decision 6). The report is a build artifact (`target/conformance/`), not a checked-in file — what
is checked in is the corpus, and what CI asserts is that each language's run accounted for every
case in it that is the language's: every case that is not a facade's, and every case of each facade
`facades.json` gives its `sdk.language` (ADR 0076 decision 2). A report that leaves one out, or runs
another language's facade case, fails.

## 6. Expected content, and how it was captured

A `live` case's expected contract content is the **engine's** record, not any SDK's output: it was
captured with the TypeScript driver's record mode (`JANUS_CONFORMANCE_RECORD=<dir> npm test`, which
writes each live case's content in the form `contract.content` expects), then read and reviewed
against the contract spec before being checked in. That keeps commitment 1 of ADR 0017 — no SDK is
the reference implementation for another — while still giving each SDK something fixed to be
compared against. A change to it is an engine or contract-format change, and is reviewed as one.

A facade case's `contracts` were captured the same way, by its own language's driver: the TypeScript
driver's record mode for pact-js's, and the JVM driver's (`JANUS_CONFORMANCE_RECORD=<dir> ./gradlew
:sdk:test --tests '*ConformanceTest*'`) for pact-jvm's, which write each file's content in the form
`contracts` expects. Each was read against the row it covers before it was checked in: that is what
makes the pin say the row is `kept` (ADR 0076 decision 4).

## 7. When a case and an SDK disagree

Three outcomes, and it matters which one is chosen:

1. **The SDK is wrong.** Fix the SDK. This is the ordinary case, and it is what the suite is for.
2. **The case is wrong.** Fix the case, in the same commit as whatever taught you so, and say in
   `why` what the specification actually requires.
3. **The specification does not say.** Then neither SDK is wrong, and a case that decides it would
   be making the suite normative beyond the document every implementer reads. Change
   `behavioural-spec.json` first (SDK spec §10's evolution rules), then write the case.
   [`Documentation/conformance-suite-report.md`](../Documentation/prototype/conformance-suite-report.md)
   lists what the suite deliberately does not decide today, and why.
