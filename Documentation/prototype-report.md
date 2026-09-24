# Pact Janus: what we built, and what we learned

*A short report for the Pact community. Plan task 9.3, 2026-09-24.*

The [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146) proposes rebuilding Pact
around one engine and thin SDKs. It keeps what makes Pact Pact: consumer-driven contracts, captured
and replayed. It changes almost everything underneath. An RFC like that is easy to agree with and hard
to believe, so we built a prototype to test its bets against real code before anyone commits to them.
That prototype is **Pact Janus**.

Janus is not "Pact 6", and it is not meant to ship. It is one Rust engine, a TypeScript SDK, a JVM SDK,
a CLI, a sample provider and a third-party plugin, built in nine phases. Every phase ran against real
sockets, real HTTP clients and real test frameworks. Its findings are the deliverable. The RFC has
already been revised with them, and this is the short version.

## The loop, in a minute

![The demo: a consumer test, verification, janus check, widening and fixing](../demo/demo.gif)

That is [`demo/run.sh`](../demo/README.md), and CI runs it on every change. It walks the loop the
RFC is built around:

**1. One consumer test, several variants.** A web app's test declares that an order's status is
`PENDING` or `SHIPPED`, with at least one item. The engine runs the test once per variant it selects,
four here, and records only the variants that ran. Declaring a variation is a promise to exercise it,
and the framework keeps the promise. You don't write four tests.

**2. The provider verifies it, with hooks.** `janus verify` replays each variant against the real
provider. The provider's config gets an OAuth2 token, sets up each variant's state through the v3
provider-state endpoint it already had, and adds a header with a few lines of JavaScript. The consumer
test binds each variant to a provider state (`whenVariant("status", "SHIPPED")`), so the provider
knows which order to set up.

**3. Can I deploy?** Verification passes, and yet:

```
✗ web-app is not compatible with order-service
  interaction 'a request for an order', response body $.items:
    provider may produce an empty list, or up to 3 items
    consumer has only tested at least one item
  interaction 'a request for an order', response body $.status:
    provider may produce: 'CANCELLED' | 'PENDING' | 'SHIPPED'
    consumer has only tested: 'PENDING' | 'SHIPPED'
```

Replay can only check what the consumer tested. The provider recorded the shape of what its own tests
saw it send, and `janus check` compares the two. This is the production incident Pact cannot catch
today, caught before deployment.

**4. The consumer widens its contract,** to include `CANCELLED` and an empty list. The engine now
selects six variants, and four of them fail the consumer's own code, each named with its cause:

```
VariantsFailedError: 4 of 6 variants failed for 'a request for an order':
  ✗ base
      TypeError: Cannot read properties of undefined (reading 'quantity')
  ✗ status=CANCELLED (response.body.status#value=CANCELLED)
      Error: unknown order status 'CANCELLED'
```

The SDK refuses to write a contract for code that cannot handle what it declared.

**5–6. The consumer fixes its code**, the variants pass, verification passes, and `janus check` says
`✓ web-app is compatible with order-service`.

## What held

All five of the RFC's bets held up.

- **One engine, thin SDKs.** Two SDKs in two languages run against one engine over one protocol. Both
  pass the same 47-case conformance suite. A complete SDK's hand-written code is **723 lines** of
  TypeScript or **1,802** of Java. The JVM SDK was written from the specification alone, by someone who
  never read the TypeScript one, and it records the same contract member for member. A CI audit fails
  the build if matching logic creeps back into either SDK.
- **Plans you can read.** Interactions compile to matching plans, and `janus explain` prints them. Your
  existing v1–v4 pacts compile to plans that agree with the Pact specification's own test cases on
  **583 of 583** in scope, and they verify unchanged.
- **Plugins through the front door.** Someone wrote a `text/csv` content plugin using only the
  published specs, without reading engine source. The same WASM file runs in a consumer test and in
  verification. It is distributed as an OCI artifact pinned by digest.
- **Optional fields, tested honestly.** The RFC's example order has 24 variants: three statuses, a
  shipping date present or absent, two payment types, and one item or several. One test covers them
  with 8, chosen deterministically, and two SDKs choose the same 8.
- **It's faster.** Janus beats today's `pact_ffi` on every like-for-like benchmark except very large
  request bodies. A mock round trip takes 33 µs against 139 µs.

## What changed

The prototype also changed parts of the RFC. The full list, with evidence, is in
[RFC feedback](rfc-feedback.md) §3.

- **The engine runs as a subprocess, not as WASM.** The RFC preferred compiling the engine to WASM so no
  SDK would ship a native binary. But a WASM engine cannot run a mock server or a verification: it
  has no sockets and no threads. So every SDK starts a small `janus-engine` process per test run. It
  exits when the test run ends, and cannot be left behind. The cost is that per-platform binaries come
  back, shipped the way tools like esbuild ship theirs. WASM stays useful for offline work such as
  `explain`, `upgrade` and `janus check`.
- **Provider shapes can be matched by operation.** A shape generated from an OpenAPI document has no
  way to know what a consumer team called its interaction. Before this change, it silently checked
  nothing.
- **The artifact isn't called "pact v5".** Janus writes its own contract format, so the Pact
  specification's next version stays the community's to define.

## What's hard

We went looking for evidence against the RFC, too.

- **Variant failures are only as clear as your test framework.** JUnit 5 and Vitest name the failing
  variant for free. A plain loop in Rust names nothing.
- **Generated provider shapes are noisy.** A shape derived from an ORM-generated OpenAPI document
  produced 4.5 times the findings of a hand-written one: every nullable column, all true, mostly
  useless. That is why `janus check` warns by default rather than blocking deployment.
- **Some field combinations cannot happen,** like a shipped order without a shipping date, and the SDKs
  cannot yet say "don't test that pair". The demo avoids it honestly, but real consumers will hit it
  (finding 35).
- **Non-JSON content strains a JSON-shaped core.** The CSV plugin exposed three gaps: column order is
  lost, an empty CSV loses its header row, and a plugin's own parse errors reach the user as generic
  mismatches.

## What it means for you

- **If you write consumer tests:** optional fields, enums and polymorphic payloads become testable in
  one test, and `explain` shows you exactly how matching works. Your existing pacts keep working, and
  `janus upgrade` converts them, telling you anything it had to narrow.
- **If you maintain a provider:** you verify with no code changes (your v3 state endpoint works as it
  is), and you can publish the shape of what you send, to learn what your consumers never tested.
- **If you maintain an SDK:** your job becomes a DSL and a test-framework integration of a few hundred
  lines over generated bindings, proven by a shared conformance suite. The matching, mocking and
  verification live in the engine.
- **If you write plugins:** you implement the same interfaces the built-ins do, from the published
  specs.

## What's next

The prototype has one task left: a staged plan for building the real thing. It will say what carries
over from Janus, what gets rewritten, and in what order. Some questions are not the prototype's to
answer: the name, versioning, governance and funding. Those belong to the community, and the plan will
set them out for that discussion.

Please join it on the [RFC pull request](https://github.com/pact-foundation/roadmap/pull/146).

---

**More detail:** [RFC feedback](rfc-feedback.md), every open question answered with evidence ·
[the demo](../demo/README.md) · [performance report](performance-report.md) ·
[decision records](decisions/README.md) · [project plan](project-plan.md)
