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
cases/live/                               the real engine: derived dimensions, and contract content
```

The four groups are SDK spec §7's four categories, which ADR 0017 fixes as the minimum coverage:
DSL→interaction-spec translation, session lifecycle, variant iteration, and contract-output
equivalence.

Every case names, in `covers`, the conformance ids from
[`behavioural-spec.json`](../Documentation/specs/sdk-specification/behavioural-spec.json) it is the
scenario for, and in `why` the specification sentence it pins. The checker holds the corpus to both:
an id no case covers is a hole in what "conformant" guarantees, and fails the build here rather than
being discovered later by two SDKs quietly disagreeing.

## 2. Running it

Each SDK runs the corpus as part of its own test suite and writes a report; the checker reads the
reports.

```sh
cargo test -p pact_janus_conformance          # the corpus against its schema and the 53 ids
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
`description`, `given`, `request`, `response`, exactly the behavioural spec's primitives. A value
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
`withhold-contract` makes it return none, `results` replaces the default results, and `errors` maps
an operation to the error document it answers with instead. Everything unset is answered the default
way: `engine/hello` agrees protocol version 1, `create` returns session `s-1`, `add-interaction`
returns `i-1`, `i-2`, …, `start-transport` returns an HTTP endpoint, `variants` returns one variant
`base`, and `finalise` returns one verified result per interaction and a small contract. Every SDK
runs as consumer `web-app` and provider `orders-api`, so the contract file is always
`web-app-orders-api.janus.json`.

**Against the real engine** (`live`): no `engine` member; the driver starts the engine `JANUS_ENGINE`
names. A closure's `exchange` is the request the consumer sends to the mock, once per variant.

## 4. What a case can expect

Per step (`expect` inside a step):

| Member | Means |
|---|---|
| `outcome` | `"ok"`, or the failure: `execute-failed` (with the `variants` it must name), `contract-withheld` (with `interactions` and `engine-withheld`), `engine-error` (with its `code`) |
| `closure-calls` | the variant ids the closure ran on, in call order |
| `contract` | `written`, the file's exact `text`, or its `content` |

Per case (`expect` at the top level): `spec` (the whole built document) or `at` (JSON pointer →
value) for translation; `ops` (every operation sent, in order), `frames` (operation → members its
request body must carry), `engine-closed`, `dimensions` (dimension ids the engine's variants must
assign) and `variant-count`.

`outcome` is a vocabulary, not a type name: each language maps it to its own failure —
`execute-failed` is `VariantsFailedError` in TypeScript and `ExecuteFailedException` on the JVM. A
language whose failures are not exceptions maps it to whatever failing a test means there.

`contract.content` compares **interaction content only** (ADR 0017): each interaction by
description, on the members the case names — `states`, `parts`, `selection` (contract spec §5–§6).
`metadata` and the parties are never compared. `metadata.writer` names the SDK and version that
wrote the file, by design (contract spec §3.2), and a suite demanding byte-identical files would be
grading a fact this project never asked two SDKs to agree on.

## 5. The report

A run writes `{ "$format": "janus-conformance-report/1", "sdk": …, "cases": [ { "id", "status",
"covers", "detail"? } ] }`. `status` is `passed` or `failed`; a failed case carries the `detail` the
checker prints. The report is a build artifact (`target/conformance/`), not a checked-in file — what
is checked in is the corpus, and what CI asserts is that each language's run accounted for every
case in it.

## 6. Expected content, and how it was captured

A `live` case's expected contract content is the **engine's** record, not any SDK's output: it was
captured with the TypeScript driver's record mode (`JANUS_CONFORMANCE_RECORD=<dir> npm test`, which
writes each live case's content in the form `contract.content` expects), then read and reviewed
against the contract spec before being checked in. That keeps commitment 1 of ADR 0017 — no SDK is
the reference implementation for another — while still giving each SDK something fixed to be
compared against. A change to it is an engine or contract-format change, and is reviewed as one.

## 7. When a case and an SDK disagree

Three outcomes, and it matters which one is chosen:

1. **The SDK is wrong.** Fix the SDK. This is the ordinary case, and it is what the suite is for.
2. **The case is wrong.** Fix the case, in the same commit as whatever taught you so, and say in
   `why` what the specification actually requires.
3. **The specification does not say.** Then neither SDK is wrong, and a case that decides it would
   be making the suite normative beyond the document every implementer reads. Change
   `behavioural-spec.json` first (SDK spec §10's evolution rules), then write the case.
   [`Documentation/conformance-suite-report.md`](../Documentation/conformance-suite-report.md)
   lists what the suite deliberately does not decide today, and why.
