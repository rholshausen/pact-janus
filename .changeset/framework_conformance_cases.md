---
engine: major
---

# The conformance suite runs the test-framework integrations

The conformance suite has a `framework` category (staged plan S2.19, ADR 0078). A framework case is a
scenario an SDK's test-framework integration must pass, such as "a withheld contract fails the suite".
Its `subjects` are keyed by framework, each a test file as that framework's users write one.
`conformance/frameworks.json` names the frameworks: `jest-commonjs`, `jest-esm` and `vitest` for
TypeScript, and `junit5` and `spock` for the JVM. A case's `project` member names files the driver
writes into the subject's project before it runs.

A driver runs each subject in its language in a project of its own, through the framework's own
runner, in a process of its own, and holds every run to the case's one expectation. When the subjects
write a contract, the report carries its digest, as a live case's does. `janus-sdk-gates conformance
check` now expects every framework case with a subject in a report's language, so an SDK in either
language must run them to move to this release. An SDK in another language runs none of them. The lint
fails a framework no case names.
