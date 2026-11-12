# 0086 — Record a provider shape from the provider's own tests, by the operation each test names

- **Status**: accepted
- **Date**: 2026-11-12
- **Plan tasks**: S4.1 (amends engine-protocol spec §8.5, design 2.8 §2.2 and ADR 0025 decision 3;
  adds to component-interfaces spec §5)
- **Evidence**: [spike 7.3 findings](../../spikes/7.3-type-derived-shapes/FINDINGS.md) §2,
  [`samples/order-service/README.md`](../../samples/order-service/README.md) "Recording its shape",
  [ADR 0025](0025-a-provider-shape-entry-may-select-interactions-by-operation.md),
  [ADR 0065](0065-let-the-engines-host-write-and-merge-contract-files-in-identity-order.md),
  [ADR 0074](0074-redact-credentials-from-what-the-engine-records-and-reports.md)

## Context

The engine records a provider shape (task 7.2, engine-protocol spec §8.5), and nothing but
`engine/kernel/tests/provider_shape_record.rs` calls it. A provider's own test suite cannot, for three
reasons.

- **`observe` names an interaction by the consumer's words**: its description and state names. A
  provider's own tests do not know them. Task 7.2's demonstration used them because it was written to.
  Spike 7.3 §2 found that this identity makes a derived shape match nothing, and ADR 0025 gave a shape
  entry a `selector` instead. The recorder never writes one.
- **`observe` takes decoded values.** To record a JSON, XML or form body, an SDK would have to decode
  it itself, which the thin-SDK rule forbids. A recorded value also skips what an arrival goes through
  in verification: the transport reads it into parts, a header becomes field lines (component-interfaces
  §5.9), and the content type is labelled or detected (ADR 0049).
- **Nothing keeps an operation's outcomes apart.** A suite that calls `GET /orders/{id}` for an order
  that exists and for one that does not records a 200 with an order and a 404 with an error document as
  one interaction. The recorder unions them, so every member of the order becomes optional, and a
  consumer that expects the order gets a `weaker-presence` finding on each of its fields. That is the
  O(fields) noise spike 7.3 measured from generated OpenAPI, produced from evidence instead.

There is also no file. `finalise` answers the document, and a JavaScript suite's test files run in
separate workers, so each would answer a part of the shape.

Recording what the provider answered during `janus verify` (the `observed` provenance, design 2.8 §2.3)
would avoid the first problem, since a verification run knows each contract's descriptions. It sees only
what consumers asked for, though, and the RFC's promise is the provider's own tests. That route is S6.8
(decision 9).

## Decision

1. **Each observation names its operation, as a selector.** `observe` takes `operation`: shapes over
   the initiating part's slots, in the part → slot → shape form of a contract's `parts`, which become the
   recorded entry's `selector` (ADR 0025). The test writes it with the DSL it already uses, so in both
   SDKs an observation is `observe({ method: "GET", path: pathTemplate("/orders/{id}") }, response)`.
   `description` becomes optional, and an entry recorded without one is named by its selector as
   `explain` prints a shape, which is what a report shows for it.
2. **The HTTP component contributes `http:path`.** Its member `template` is a path template as OpenAPI
   and every router write one: `{name}` matches one non-empty segment, and every other character matches
   itself. It admits a path that matches the whole template. Both SDKs spell it `pathTemplate(t)`, which
   a consumer test may use too, and S4.2's importer writes it in place of a regex. `janus check` compiles
   a selector with the session's components, so a selector may use any operator a component runs.
3. **An observation is an arrival.** `observe` takes `parts` as the transport's own parts, each slot
   wrapped as contract spec §5.3 wraps one, a content slot carrying its octets. The session reads them
   as a verification reads what a provider answered: through the transport component for `kind`, a new
   member of `create` (`http` unless given), with each content slot decoded through the session's
   content registry by its labelled or detected type. A value that does not decode is recorded as its
   octets, and the `observe` result says so, as a warning.
4. **A transport declares the slots that tell an exchange's outcomes apart**, as `outcome-slots` in its
   `TransportContribution` (component-interfaces §5): part name → slot names. HTTP's are
   `{ "response": ["status"] }`. The recorder keeps one entry per operation, state names and outcome
   value, and writes each outcome slot's value into the entry's selector as `equality`. So a consumer
   interaction that recorded a 200 selects the 200 entry, the 404 entry is about the consumers that
   expect a 404, and neither widens the other. A transport that declares none records one entry per
   operation and state names.
5. **States are the provider's own, and optional.** An observation may name provider states, from the
   vocabulary the provider's `state-setup` handlers answer to, which is the vocabulary consumers write.
   ADR 0025 decision 3 changes: when an entry that names the interaction's states and one that names
   none both select it, the one that names them is used. Two entries alike in that are still a tie.
6. **What a transport declares a credential is not recorded.** A member in its `credential-members`
   (ADR 0074) is recorded as present or absent, with no example and no values, so a published shape
   never carries a `Set-Cookie` a test was given.
7. **The engine's host writes the shape, and merges it.** `finalise` takes
   `write: { path, evidence, source }`, behind the `contract-writing` capability ADR 0065 added, and
   answers `written`. `evidence` is a file of the recorder's counts for each entry, keyed by `source`. A
   session replaces its own source's counts, and the shape is written from all of them, under the
   evidence file's lock, held as ADR 0065 decision 6 holds a contract's. So a test file run twice counts
   once, and test files that ran one after another or in separate workers write the shape one session
   recording them all would have. A single session's shape needs no merging, but is written the same
   way. The SDKs name the source after the test file or class, write the shape to
   `<shape directory>/<provider>.provider-shape.json`, and keep the evidence in the build directory
   (`target/janus/evidence/` or `build/janus/evidence/`, and `.janus/evidence/` beside a TypeScript
   project's `package.json`), so a shape directory holds only what is published. A test file that was
   deleted leaves its counts until the build directory is cleaned, as a contract keeps a deleted test's
   interactions.
8. **In each SDK, a recorder is part of the provider side.** TypeScript's `ProviderShapeRecorder` and
   the JVM's are opened on the engine, given a response as status, headers and body octets or as the
   standard client returns one (`fetch`'s `Response`; `java.net.http`'s `HttpResponse`), and finalised
   in an `afterAll`, or by the JUnit 5 extension when the test class's store closes. Reading a framework's
   own test response (Spring's `MvcResult`, supertest's) is an adapter, and S6.7's.
9. **Recording during verification is not built here.** It is additive: a `verification/verify` option
   that hands each answer the run scored to a recorder, with provenance `observed` and the contract's
   identity. It is S6.8, and waits until someone asks for it.

## Alternatives considered

- **The test names the operation as a string, `"GET /orders/{id}"`, and the transport makes the
  selector.** Simpler to type. It needs a transport operation that turns an operation's text into
  shapes, which is the transport-owned operation ADR 0025 declined, and the template still needs an
  operator to be matched with.
- **Read the template from the framework's routing** (Spring's best-matching pattern, Express's
  `req.route.path`). Nothing to type, and an adapter for every framework in every SDK's integration
  layer. An adapter can still fill in `operation`, and S6.7 tracks it.
- **Partition by state names alone.** Correct when a suite names its states, and a suite that names
  none is the common case, which is the case that unions a 200 with a 404.
- **Merge finished shapes.** A shape has lost the counts the recorder's two judgements need: two test
  files that each saw one timestamp would merge into a two-value enum. And adding counts on every run
  makes a shape depend on how often it was recorded.
- **One shape file per test file, merged by `janus check`.** Every reader of a shape would need the
  merge, the broker included, and two files' entries for one operation would tie.

## Consequences

Easier: a provider adds a recorder to tests it already has and publishes what they produced, matched to
every consumer by operation, with no consumer's words in it. Recording goes through the same transport
and content components a verification does, so a recorded `Content-Type` or `headers` is what the walk
compares against a consumer's.

Harder: a recorded shape now has `http:path` and HTTP status in its selectors, and a selector is only as
good as the template the test wrote. A test that names the wrong template records an entry nothing
selects, which the report shows as `not-published`. The recorder's counts live in a file of their own, in
the build directory, and cleaning it is how stale counts go.

Committed to: `observe`'s new form (decisions 1 and 3) is a change to §8.5, behind the
`provider-shape-recording` capability, whose engines all predate it, so the old form is not kept.
`outcome-slots` is a transport declaration like `content-slots` and `credential-members`, made in code
by an in-tree transport.

**Tripwire.** Revisit decision 4 if a transport's outcomes are not a slot's value (a message whose kind is
a header and a body member together), and decision 7 if the lock on the evidence file serialises a large
parallel suite enough to show in its time.
