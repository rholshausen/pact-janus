# Reuse inventory — pact-reference, pact-jvm and friends

*Plan task 0.4 · Surveyed 2026-08-12 against local checkouts. Feeds the G1 "kernel starting point" ADR;
this report recommends, the ADR decides.*

Verdict key: **dependency** (consume as a released crate/artifact) · **fork** (copy into Janus and
adapt, recording provenance) · **data** (reuse test data/fixtures) · **reference** (read for semantics,
write new code) · **baseline** (used only to measure against).

## Summary table

| Asset | Where | License | Verdict |
|---|---|---|---|
| `pact_models` 1.3.15 | pact-reference/rust | MIT | **dependency** |
| v2 matching engine | pact-reference `pact_matching/src/engine` | MIT | **fork** (kernel starting point) |
| v1 matching code | pact-reference `pact_matching` | MIT | **reference** (semantics oracle for 3.5) |
| Spec test cases (803 JSON files, v1–v4) | `pact_matching/tests/spec_testcases` (mirrors pact-specification) | MIT | **data** (verdict-diffing corpus for 3.5) |
| Mock server | pact-core-mock-server | MIT | **reference** (HTTP transport component is written to the 2.6 interface) |
| `pact_verifier` | pact-reference/rust | MIT | **reference** (orchestration semantics; its callback design is what MkII replaces) |
| `pact_ffi` | pact-reference/rust | MIT | **baseline** (benchmark 1.7 only; never a dependency) |
| Compatibility suite | pact-compatibility-suite (+ rust & jvm harnesses) | Apache-2.0 | **data/fork** (seed for conformance suite 6.4) |
| Plugin driver + gRPC proto | pact-plugins `drivers/`, `proto/` | MIT | **reference**; proto possibly **fork** for escape hatch 8.3 |
| pact-jvm | pact-jvm | Apache-2.0 | **reference** (JVM SDK integration patterns; matchers/model not reused) |
| pact-js / pact-js-core | pact-js | MIT | **reference** (TS DSL surface; SDK written fresh from SDK spec 2.9) |

## Detail and justification

### `pact_models` — dependency

Core pact data structures, v1–v4 pact file reading/writing, matching-rule and generator models,
`DocPath`, content types. Version 1.3.15; features `datetime`, `xml`, `form_urlencoded`; builds for
`wasm32-wasip2` (checked in pact-reference CI), which Janus's kernel requires. The v2 engine imports it
pervasively (`matchingrules`, `path_exp`, `v4::http_parts`, `bodies::OptionalBody`), so forking the
engine without taking `pact_models` would mean rewriting both at once — exactly the risk the plan
avoids. Janus adds its own v5 model beside it (task 3.1); `pact_models` stays the v1–v4 door.
Consume from crates.io; a git pin only if Janus needs unreleased fixes.

### v2 matching engine — fork as the kernel starting point

`pact_matching/src/engine`: ~6,000 lines of core (`mod.rs` 2,484 — plan node grammar and builders;
`interpreter.rs` 2,853; `context.rs`; `value_resolvers.rs` for HTTP request/response and message;
`xml.rs`) plus ~1,400 lines of body plan builders (`bodies/json.rs` 710, multipart, others) and an
`engine/tests` directory (total ≈13.5k lines with tests). This is the RFC's "already prototyped" plan
compiler/interpreter: containers, actions, values, resolvers, pipelines, pretty/executed rendering.

Fork rather than depend because Janus must change its spine: today plan construction is driven by
v1–v4 matching rules attached to parts; Janus compiles *shapes* (design 2.2) with variant dimensions
(2.3), makes the node grammar public/versioned surface (2.4), and moves body builders behind content
components (2.6). Those are structural changes to the crate's core, not additions around it.

Fork notes:
- It is wired into `pact_matching` via a `PACT_MATCHING_ENGINE=v2` toggle at four call sites in
  `lib.rs` and leans on crate-internal helpers (e.g. `crate::headers::parse_charset_parameters`);
  the fork must bring those helpers along or reimplement them.
- Inline unit tests in the core files are sparse (tests concentrate in `engine/tests` and in running
  the spec suite with the env toggle) — bring the test directory across and grow corpus coverage
  (3.7) before restructuring.
- Rendering pulls in `ansi_term`; keep terminal colouring out of the kernel proper (or feature-gate)
  to protect the `wasm32-wasip2` build.
- MIT code entering an Apache-2.0 repo: fine, but preserve the MIT notice and record file-level
  provenance (source repo, commit SHA) in the forking commit, per `CLAUDE.md`.

### v1 matching code and the spec test cases — the oracle for task 3.5

`pact_matching`'s current (v1-engine) implementation is the behavioural oracle: task 3.5 compiles
v1–v4 matching rules to plans and diffs verdicts against it over the 803 spec test case JSON files
under `pact_matching/tests/spec_testcases` (v1, v1.1, v2, v3, v4 — mirroring the pact-specification
repo). Copy the test cases into Janus's corpora (recording the source SHA) or fetch them in CI from
pact-specification; do not hand-maintain a divergent copy.

### Mock server, verifier, FFI — reference and baseline only

- **pact-core-mock-server**: hyper-based; useful to crib request capture and TLS details, but Janus's
  HTTP transport is written against the component interface (4.2) — importing a server built around
  the old matching entry points would smuggle the old shape in.
- **pact_verifier**: read it for what verification actually has to handle (state-change teardown
  ordering, pending pacts, broker result publishing). Its request-filter callbacks and per-language
  hook wiring are precisely what hooks (2.7) replace, so no code moves.
- **pact_ffi**: benchmark baseline (1.7) and the negative example the protocol design (2.1) is tested
  against. Never linked into Janus.

### Compatibility suite — seed for the conformance suite

`pact-compatibility-suite` (Apache-2.0) holds Cucumber features for V1–V4 behaviour, with existing
harnesses in pact-reference (`compatibility-suite/`, Rust) and pact-jvm (`compatibility-suite/`).
Task 6.4 grows the Janus conformance suite from these features: the features fork cleanly (they are
implementation-neutral by design); the step harnesses are per-implementation and only worth reading.
Bonus: two existing harnesses demonstrate the suite is portable across implementations — the property
Janus's SDK conformance needs.

### pact-plugins — the escape hatch is already specified

`proto/` defines the gRPC plugin protocol that the RFC keeps as the out-of-process escape hatch
(spike 8.3); reusing that proto (MIT) as the starting point keeps existing plugin authors' mental
model. The Rust `drivers/` crate shows lifecycle/discovery handling but predates the component-first
design — reference only.

### pact-jvm — reference for the JVM SDK, not for the engine

Converging pact-jvm onto the engine is the point, so `core/matchers`/`core/model` are explicitly not
reused (they are the duplication MkII removes). What *is* valuable: `consumer/junit5` and
`provider/junit5` integration patterns (annotations, extension lifecycle) as the idiom target for the
JVM SDK (6.3), the Gradle build conventions, and its compatibility-suite harness as above. Apache-2.0,
same as Janus.

### pact-js / pact-js-core — reference for the TS SDK

The TS SDK (6.2) is written fresh from the SDK specification (2.9) — that is itself the experiment.
pact-js informs the DSL ergonomics users expect (and its FFI-wrapping pain in pact-js-core is the
"before" picture for B1 evidence). MIT.

## Risks and follow-ups

- The v2 engine is a moving target — it is actively developed in pact-reference. Fork at a recorded
  SHA at the start of Phase 3; cherry-pick upstream fixes consciously rather than tracking.
- `engine/xml.rs` and `bodies/` beyond JSON are thin; treat non-JSON content as unproven in the fork
  and scope Phase 3 to JSON first (as the plan already does).
- Sparse inline tests in the engine core mean the corpus (3.7) is the real safety net for the
  restructuring — build it before, not after, invasive changes.
- `pact_models` carries `anyhow`-based errors; the engine boundary needs structured errors (2.1), so
  expect a translation layer at the kernel edge rather than leaking `anyhow` through the protocol.
