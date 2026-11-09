# 0076 — Write a facade's conformance cases as tests in its own language, and read its verdicts from its migration note

- **Status**: accepted; extended by [ADR 0078](0078-run-a-test-framework-integrations-conformance-cases-in-a-project-of-their-own.md); amends [ADR 0017](0017-sdk-conformance-is-suite-passing-not-prose-matching.md)
  (what a case is, and which cases an SDK runs) and SDK spec §6.2–§6.3 (where a facade's verdicts live)
- **Date**: 2026-11-09
- **Plan tasks**: S2.6; S2.19 (the test-framework integrations, which this decision's subjects make possible)
- **Evidence**: [conformance-suite report](../prototype/conformance-suite-report.md) §5 and §7.2;
  [`consumer-test-migration.md`](../consumer-test-migration.md) and
  [`provider-test-migration.md`](../provider-test-migration.md), whose rows give 97 spellings the verdict
  `kept`; the behavioural specification's `facade` entries for `verifier`, `state-handler` and
  `message-producer`, which call four of those spellings `adapted`; contract-file spec §8.4, which raises
  `request-query-opened` for every HTTP request

## Context

S2.6's exit is that every facade primitive classified `kept` has a conformance case. The suite cannot
hold one today, for three reasons.

**A case is data that every language replays.** That is what keeps two SDKs from reading one scenario
two ways (conformance README §1). But a facade is one language's spelling of another library's API:
pact-js's builders exist only in TypeScript, and pact-jvm's annotations, lambdas and overloaded builders
only on the JVM. There is no second language to replay a facade case, and much of what the rows promise
cannot be written as data at all. `@PactIgnore`, "`@Pact` methods no test ran fail the class", and
`@ForProvider` on a parameter are facts about a JUnit class. The checker also requires every case of the
corpus in every report, so a facade case would fail every SDK that does not ship the facade.

**The verdicts are in two places, and they disagree.** SDK spec §6.3 put an FFI facade's verdicts in its
language's migration note. The provider side's are in the behavioural specification's `facade` arrays,
written in S1.14, before either provider facade existed. The provider note, written in S1.15 against the
facades as built, calls `stateHandlers`, `customProviderHeaders` with `disableSslVerification`,
`messageProviders` and `@State` `kept`. The arrays call all four `adapted`. Whichever S2.6 counts, the
other is wrong.

**"Kept" cannot mean what §6.3 says.** §6.3 calls a primitive `kept` when it "converts with no `lossy` or
`judgement` finding". But contract-file spec §8.4 raises `request-query-opened`, a `lossy` finding, for
every HTTP request, and `request-body-opened` for every object request body. Read interaction by
interaction, no HTTP row could ever be `kept`.

The suite has a gap of the same kind in each SDK's own types. `regex` refuses a pattern compiled with
flags the pattern text cannot carry (ADR 0019's context), but a flag is a `RegExp`'s or a `Pattern`'s,
and data cannot spell one. The report's §5 left that refusal to each SDK's own tests.

## Decision

**1. A case may name a subject: a test written in one language, beside the case.** `subjects` maps a
language to a file in the case's directory. The case's expectation stays data, and stays the corpus's:
how each test ended (`tests`), the contracts the run wrote (`contracts`), what it printed (`output`), what
the engine logged (`engine-log`), and, against a scripted engine, the frames it sent.

- A TypeScript subject is a module whose exported async functions are its tests. The driver calls each
  test the case names, in the case's order, with a context: the contract directory, and the scripted
  engine when the case has one. A facade that takes an engine (pact-js's `engine` option) is handed it as
  a user's test hands it one. The module imports the SDK by its package name (`pact-janus/pact`), which
  the driver resolves to this tree's sources.
- A JVM subject is a JUnit Jupiter test class. The driver compiles it against the test classpath, runs it
  through the JUnit Platform as a build does, and names the contract directory in `pact.rootDir`, which is
  where pact-jvm's users name it.

A subject is written as a user of the thing under test writes a test. A subject that does what no user
would, so that its case passes, is the "driver that helps" of README §2, and fails review.

**2. A facade's cases are run by the SDK that ships it.** `conformance/facades.json` names each facade,
the language whose SDK ships it, and the section of each migration note that classifies it. A case with a
`facade` member belongs to that facade, and has one subject, in that language. An SDK's report must
account for every case with no `facade`, and for every case of each facade its language ships. A facade
cannot be left out of a report by leaving it out of the report.

**3. A facade's verdicts are its migration note's rows, and nowhere else.** A facade case names the rows
it covers in `rows`, each by its first cell, verbatim, in the facade's section of a note. The lint reads
the notes. It fails on a `kept` row that no case names, and on a row a case names that the note does not
have. A case may also name `adapted` rows. So a note row is the specification a case is checked against,
and the note stays the one place a user reads verdicts.

The behavioural specification's `facade` arrays are removed. They were the second copy, written before
the facades existed. The `kept` they disagree on is the note's: SDK spec §6.1 calls a spelling `adapted`
when the facade must "supply a default the old call site never had to state". A handler that runs over a
loopback socket, or a header that becomes a transport member, supplies no such default. The schema keeps
`facade` as optional, so a future facade that maps onto primitives could use it. SDK spec §6.2 says a
facade's note is where its verdicts are written, whatever the facade maps onto.

**4. A case shows a row is `kept` by the content it recorded.** For a builder or a matcher, the case pins
the contract the facade recorded, by interaction content (ADR 0017's comparison), reviewed against the
row. A rule that converted with a `judgement` or a loss records something else: a narrower operator, an
inferred type, a missing generator. So the pin fails when a row stops being `kept`. §6.3's "no `lossy` or
`judgement` finding" is read per rule, as the findings at the rule's position. The conversion's findings
are not pinned. The JVM facade has no frame seam to observe them through, and the recorded content is
what a user relies on.

**5. Per-language refusals are cases with a subject in every language.** A case that is not a facade's
may name a subject per language, for a spelling that data cannot carry. The driver fails such a case when
it has no subject in the driver's language, so the case runs in every SDK, as ADR 0017 requires. The first
is `translation/regex-flags-refused`, under a new conformance id, `shape.regex.flags-refused`.

**6. The test-framework integrations are S2.19.** The Janus DSL's integrations (Jest and Vitest, JUnit 5
and Spock) get cases the same way, a subject per language and framework. Running Jest or Spock needs a
driver that runs the framework in a scratch project, which S2.4a's harness does today for one SDK's own
tests. That is its own work, and not this item's exit. pact-jvm's JUnit 5 extension is covered here,
because its subjects are JUnit classes.

## Alternatives considered

- **Facade chains as data, replayed by reflection.** `{ "call": "integerType", "args": ["id", 1] }`, with
  a driver that looks each method up by name. Rejected: an annotation or a lambda has no such form, and
  pact-jvm's overloads (`integerType(String, Integer)` beside `integerType(String, Long)`) would have the
  driver choose a Java type for every JSON number. That is the driver deciding what a case means.
- **Subjects in each SDK's own test tree, with only the expectations in the corpus.** Rejected: the
  corpus would no longer say what is tested. Once the SDKs graduate (S2.9), every facade case would be
  defined in another repository and only checked here.
- **A JSON classification beside each note**, which the lint reads instead of Markdown. Rejected: every
  row twice, in two files that drift exactly as the `facade` arrays drifted. Matching a row by its first
  cell is fragile only in the right direction: editing a row's spelling fails the lint until its case
  follows.
- **Pin the conversion's findings, through a recording seam in each facade.** Rejected: it adds an engine
  or facade feature for the suite's sake. And findings explain a conversion, where the recorded content
  is the conversion. Decision 4's content pin fails whenever a finding would have changed.
- **Leave facade tests in each SDK, and the verdicts as prose** (as before S2.6). Rejected: that is the
  gap the exit closes. A `kept` that no build checks is a claim, and §6.1's own rule is that a `kept` name
  never gains new behaviour, which only a build can hold anyone to.

## Consequences

Easier: each `kept` row is now a failing build when it stops being true, and names the row it broke. The
four disagreeing verdicts are settled, in one place. Each language's own refusals, and later its
framework integrations, sit in the one corpus.

Harder, and committed to:

- The lint reads `Documentation/consumer-test-migration.md` and `provider-test-migration.md`. Renaming
  either, renaming a facade's section, or rewording a row's first cell means changing the cases that name
  them, in the same commit.
- `janus-spec.tar.gz` carries the subjects and `facades.json`, and not the notes. `check`, which a release
  runs, reads only the corpus and the reports. `lint` runs in this repository.
- The JVM driver compiles Java while the tests run, so the JVM suite needs a JDK, which the Gradle
  toolchain already is. The TypeScript driver resolves `pact-janus/*` to `src/` through Vitest's alias.
  It runs a case that expects the engine's logs in a child Vitest, because the engine inherits the test
  process's standard error.
- A subject is a test that can be written to pass, where data cannot. Review holds it to its row.

**Tripwire**: revisit decision 1 if a facade arrives in a language whose tests cannot be run from its
driver: a compiled language with no in-process compiler, where a subject would need a build of its own.
Revisit decision 4 if a `kept` row's claim shows only in the findings and never in the recorded content.
That would argue for the recording seam rejected above. Revisit decision 2 if two SDKs ship one facade:
`facades.json` would then name a facade's languages, not its language.
