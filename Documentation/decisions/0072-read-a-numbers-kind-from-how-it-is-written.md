# 0072 — Read a number's kind from how it is written

- **Status**: accepted
- **Date**: 2026-11-05
- **Plan tasks**: S1.38, found by S2.5b's `OkHttpTest`
- **Evidence**: `pact_matching::json`'s `Integer` and `Decimal` rules (`is_i64() || is_u64()`, and
  `is_f64()`), and pact-jvm's `matchInteger` and `matchDecimal` (`MatcherExecutor.kt`, 4.7.8); the
  shape spec's kind predicates (§4.2); contract-file spec §2.4, which ADR 0063 pinned; S2.5b's
  `OkHttpTest`, which `sdks/jvm/facade/build.gradle.kts` excludes for it; the S1.1 fixture
  `matching-rules/number-kinds`

## Context

The engine decides whether a number is an integer or a decimal by its value. `match:integer` admits
a number with no fractional part, and `match:decimal` one with a fractional part, so `100.0` is an
integer and not a decimal. Subsumption's value membership and the provider-shape recorder decide the
same way.

Today's verifiers decide by how the number is written. pact-reference's JSON matcher calls a number an
integer when serde_json read it as one (`is_i64() || is_u64()`) and a decimal when it read it as a
double (`is_f64()`): `100.0` is a decimal and not an integer. pact-jvm agrees about `100.0`, reading
its `JsonValue.Decimal` by scale. So a provider that answers `100.0` to a pact's `decimal` rule passes
today and fails `janus verify`, and one that answers it to an `integer` rule does the reverse.

S2.5b found it at the mock. pact-jvm's `OkHttpTest` declares `decimalType("amount", 100.00)` and
sends `"amount":100.00`. pact-jvm's mock accepts it. The facade's mock refuses it, since the facade's
interactions are what `janus upgrade` writes, and the shape `decimal` is the value-based
`match:decimal`.

The plan's text for S1.38 proposed fixing the legacy compiler alone, with a check on the number as
written, and leaving the shape language's `integer` and `decimal` as they were. That reaches
`janus verify` of a pact, but not a contract upgraded from it, and not the facade's mock, which
matches that contract. Both would still disagree with today, and S1.38's exit asks for neither.

The shape spec already speaks of the written form. §4.2 says `decimal` "admits a number written with
one" fractional part, and that where a content component cannot tell lexical forms apart, `decimal`
degrades to `number`. The value-based interpreter is the deviation, not the spec. Equality is a
different question: §4.2 says `1` and `1.0` are the same value, and that stays.

The document model keeps the written form. The kernel's numbers are serde_json's, without
`arbitrary_precision`, which hold an integer that fits in 64 bits as an integer and any other number
as a double. Contract-file spec §2.4 (ADR 0063) writes them back the same way, `100.0` as `100.0`.
That classification is exactly the one pact-reference's rules read.

## Decision

1. **A number is an integer when it is written with no fraction or exponent and fits in a signed or
   unsigned 64-bit integer, and a decimal otherwise.** This is contract-file spec §2.4's own
   classification and serde_json's: `100` is an integer, and `100.0`, `1e2`, `-0` and
   `18446744073709551616` are decimals. One function in the kernel decides it, and every place that
   asks uses it.
2. **The shape language's `integer` and `decimal` read it.** `integer` admits an integer, `decimal` a
   decimal, and `number` either. They still partition `number`, so the kind lattice and subsumption's
   comparisons are unchanged. Subsumption's value membership and the provider-shape recorder use the
   same classification, so a recorded `100.0` is a decimal.
3. **Equality is unchanged.** `1` and `1.0` are one value to `equality`, `any-of`, `one-of` and every
   other comparison of values (shape spec §4.2). Only the kind predicates read the written form.
4. **The legacy compiler needs nothing new.** A v1–v4 `integer` or `decimal` rule on a document
   value compiles to `match:integer` or `match:decimal`, as it does now, and those now decide as
   today's verifier does. A rule on text still goes through `legacy:parses-as`, which reads the text
   as today's verifier does.
5. **pact-reference is the reference where pact-jvm differs.** pact-jvm also calls `0` a decimal, and
   accepts an integer whose text matches its decimal pattern. S1.1's harness measures against
   pact-reference, and the engine follows it. A facade test that depends on pact-jvm's difference is
   recorded as one.

## Alternatives considered

- **Fix the legacy compiler alone, with a `legacy:` action that reads the written form** (the plan's
  text). It cannot reach an upgraded contract or the facade's mock, which compile from shapes, so
  `OkHttpTest` still fails and the upgrade ledger gains an entry for every fixture that has the case.
- **Give the shape language a second, written-form decimal for `upgrade` to write.** That gives one
  meaning two operators, both of which an author must choose between, to keep a reading the spec does
  not ask for.
- **Keep the value reading and record the difference as deliberate.** It is today's behaviour, met by
  every Java provider whose JSON library writes a `double` with `.0`. Those providers would fail
  `janus verify` against pacts they pass today, and pact-jvm's users' consumer tests would fail on the
  facade.

## Consequences

- A provider answering `100.0` to a `decimal` rule passes, and to an `integer` rule fails, in
  `janus verify` of a pact, of its upgraded contract and at a mock, as it does today.
- A Janus contract's `decimal` no longer admits `100`, and its `integer` no longer admits `100.0`. A
  TypeScript author cannot write `100.0` (JavaScript writes it `100`), so a `decimal` whose example is
  a whole number is served as an integer, which is the limit pact-js has today.
- A content component whose document cannot tell `100` from `100.0` apart must say so, as §4.2 already
  requires: its `decimal` degrades to `number`.
- The engine is committed to keeping a number's integer-or-double form from the octets to the
  predicate. Turning on serde_json's `arbitrary_precision`, or any number model that loses the
  distinction, changes these verdicts, and the corpus cases pin them.
- **Tripwire**: a content type whose numbers have a written form this classification misreads (a
  format that writes `100.` or `1_000`), or pilots who need `decimal` to admit a whole number. Either
  is a superseding ADR.
