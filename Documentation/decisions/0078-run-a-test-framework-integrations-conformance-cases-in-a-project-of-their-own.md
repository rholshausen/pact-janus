# 0078 — Run a test-framework integration's conformance cases in a project of their own, through the framework's own runner

- **Status**: accepted; extends [ADR 0076](0076-write-a-facades-conformance-cases-as-tests-in-its-own-language.md)
  (decision 1's subjects, and decision 6, which deferred this)
- **Date**: 2026-11-09
- **Plan tasks**: S2.19
- **Evidence**: [conformance-suite report](../prototype/conformance-suite-report.md) §5 and §7.2, which
  left "that the *framework hook* runs `finalise`" to each SDK's own tests; S2.4a's
  `sdks/typescript/test/frameworks.test.ts` and S2.4b's `FrameworksTest`, which run Jest, Vitest, JUnit 5
  and Spock suites as a user's project does; the behavioural specification's `finalise`, which says the
  test-framework integration MUST run it after the suite's last `execute`, failed or not

## Context

Each SDK has its test-framework integrations: `useJanus` from `pact-janus/jest` and `pact-janus/vitest`,
`JanusExtension` and `JanusProviderExtension` for JUnit 5, and a Spock spelling that needs no integration
(the style guides, "Test-framework integration"). What they promise is in the behavioural specification:
`finalise` runs after a suite's last test, pass or fail; a failed variant fails the test that ran it; a
withheld contract fails the suite. The suite pins those against `finalise` called by the driver, not by a
framework's hook. That a hook calls it, and what the framework then reports, is each SDK's own test.

ADR 0076 made the suite able to hold a test in one language, a subject, which the driver runs in its own
process: a TypeScript module whose exported functions it calls, or a JUnit class it launches through the
JUnit Platform beside its own tests. Neither runs a framework as a user's project does. A Jest suite is
a file Jest finds, loads under its own module system and reports in its own words. Its after-all hook's
failure is the file's. Its contract goes to the working directory's `contracts`. A Spock specification
is Groovy, and has a shared field Spock cleans up. Called from the driver's process, a subject shares
the driver's working directory and its after-all hooks, and the integration is not what is under test.

## Decision

**1. A `framework` category, and `conformance/frameworks.json`.** The list names each test framework an
SDK integrates with: the language whose SDK it is, the file a subject for it is (its suffix), and the
style-guide sections that describe the integration. There are five frameworks:
`jest-commonjs` and `jest-esm` (a `.test.cjs` and a `.test.mjs` file, since the style guide promises
both module systems), `vitest` (`.test.ts`), `junit5` (a `.java` class) and `spock` (a `.groovy`
specification).

**2. A framework case's subjects are keyed by framework.** Each is a test file written as that
framework's users write one: a Jest or Vitest test file, a JUnit Jupiter class, or a Spock
specification. It is written against the SDK's public entry points, and it configures nothing a user's
test would not. A case is one scenario, such as "a withheld contract fails the suite", and has one subject
per framework that can spell it. A case written for one framework's own spelling names only that
framework: `JanusProviderExtension`'s test per interaction, or JUnit's test per variant.

**3. The driver runs each subject in a project of its own, through the framework's own runner, in a
process of its own.** The project directory is the run's working directory, so a contract goes where
the SDK writes one by default (`contracts`), and the case's `project` member writes files into it
first, such as a pact for a provider test.

- **TypeScript**: the SDK is built and installed into the project's `node_modules` as a user installs
  it (S2.4a's `buildPackage`). Jest or Vitest runs from its own command line, with its JSON reporter.
  Jest gets Node's `--experimental-vm-modules` for an ES module test file, as any user's run does.
- **JVM**: the subject is compiled against the test class path, by `javac` or by Groovy's compiler. A
  forked JVM then runs the JUnit Platform's launcher on it, in the project directory, as Gradle's test
  worker does. The Jupiter and Spock engines on that class path discover it.

**4. A framework case's expectations are a subject case's, and every subject is held to them.** These
are `tests` by name, `suite`, `contracts` and `output`, as README §4 gives them. So the subjects of one
case name their tests alike: a JUnit test by `@DisplayName`, as its users name a test in words. `suite`
is the test file's or class's own failure, beyond its tests: the file's error in Vitest's report, the
file's "Test suite failed to run" in Jest's, or the class's container in JUnit's and Spock's. When a
case's subjects write a contract, every subject of one language must write the same bytes, and the
report carries that file's digest. So `check` holds two languages' integrations to one contract (ADR
0063 decision 6), whichever framework recorded it.

**5. A framework case is run by every SDK it has a subject for.** An SDK runs every subject in its
language, and its report must account for each case it has one for. The lint fails a subject keyed by
a framework the list does not name, a file without its framework's suffix, and a framework that no case
names. A framework must not drop out of the suite because its only case was deleted. It also fails a
style-guide section the list names that the guide in the tree does not have.

**6. The SDKs' own framework tests keep only what one subject cannot show.** That is several test files
run at once, merging into one contract (S2.11, ADR 0065), and pact-js's own verifier under Jest. That
second one is a facade's spelling run by a framework, not the Janus DSL's integration.

## Alternatives considered

- **Run a framework's subject inside the driver's process**, as ADR 0076 runs facade subjects.
  Rejected: an integration's whole job is to hook the framework's after-all, report through the
  framework, and write where a user's project writes. In the driver's process, the driver's working
  directory and hooks stand in for the framework's. The S2.4 harnesses already run frameworks out of
  process because of this.
- **One case per framework**, each with every scenario as a test in one file. Rejected: Jest and Vitest
  make a file the suite, and "a withheld contract fails the suite" needs a suite whose other tests pass.
  Scenarios would also stop lining up across frameworks, which is what lets one expectation hold four
  integrations to the same behaviour.
- **Require a subject for every framework in every case.** Rejected: JUnit's dynamic tests have no
  Spock or Jest spelling, and the provider extension has no counterpart in a framework that needs none.
  Instead, the lint holds every framework to at least one case, and review holds each case to every
  framework that can spell it.
- **Run the JVM subjects through a Gradle build of their own.** This is closest to a user's project.
  Rejected: each run would start Gradle and resolve dependencies, which costs tens of seconds per subject
  and needs the network or a primed cache. The JUnit Platform launcher in a forked JVM is what Gradle's
  test worker itself runs.

## Consequences

Easier: "conformant" now covers the integrations. A Jest suite whose session is never finalised, or a
withheld contract that fails nothing, fails a case that names the framework. Four integrations are held
to one expectation and one contract. A third SDK's integrations join by adding its frameworks to the list
and subjects to the cases.

Harder, and committed to:

- The TypeScript driver builds the package and runs Jest and Vitest as child processes. Jest is a
  development dependency of the SDK, as it was for S2.4a. A release's `sdk-gates` job runs these cases
  from the SDK's own checkout, which has both.
- The JVM driver forks a JVM per subject, with the test class path in an argument file, since a
  Windows command line cannot hold it. It needs Groovy's compiler at test time, which Spock already
  brings.
- A subject runs in seconds, not milliseconds, so framework cases have a longer time limit than the
  suite's other cases.
- `frameworks.json` names style-guide sections under `sdks/`. When the SDKs graduate (S2.9), the list
  names the guides where they then live, and the lint checks them only when they are in the tree.

**Tripwire**: revisit decision 3 if a framework arrives whose runner cannot be started from the
driver, such as one that runs only inside its build tool. Revisit decision 4's
shared expectation if two frameworks of one language name the same test differently, and no user's
spelling can make them agree.
