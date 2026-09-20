# 0019 — An SDK refuses what it cannot spell, and stops when the engine does

- **Status**: accepted
- **Date**: 2026-09-20
- **Plan tasks**: 6.4 (amends the 2.9 behavioural specification; binds 6.2, 6.3, 6.5)
- **Evidence**: [conformance-suite report](../conformance-suite-report.md) §4.2 and §5,
  [JVM-SDK-from-spec report](../jvm-sdk-from-spec-report.md) §2.3 and §2.5, the conformance suite's
  own cases `translation/header-name-collision`, `translation/value-as-one-string`,
  `translation/unspellable-value`, `lifecycle/engine-error-aborts-the-loop`,
  `lifecycle/variant-budget-exceeded`

## Context

Task 6.4's suite found what 6.3 predicted it would: three places where the TypeScript and JVM SDKs
behave differently, pass every conformance id they name, and are both defensible, because the
behavioural specification never said. All three are one question wearing three hats — **what may an
idiomatic layer decide by itself?** SDK spec §2.1's thinness test answers it for matching ("does this
decide anything the engine does not already decide"), and these three sit just outside its reach:
they are about values and failures that never reach the engine at all, or that reach it as something
other than a verdict.

1. **Two header names that collide once lower-cased** (`Accept` and `accept`). TypeScript kept the
   last and dropped the first, silently; the JVM refused the second declaration. The engine cannot
   arbitrate, because it only ever sees the survivor.
2. **A header or query value that is none of the three declared forms** — `header("X-Count", 3)`.
   TypeScript built an `each-like` over `equality 3`, a shape the transport's string values can
   never match, which the engine accepts and which therefore fails silently; the JVM refused. Both
   are wrong for the author, who expects `X-Count: 3` on the wire.
3. **An engine error from `serve-variant` partway through the variant loop.** TypeScript let it
   abort `execute`; the JVM recorded it as that variant's failure and carried on, so a dead engine
   produced one "test failure" per remaining variant.

The specification's existing vocabulary bounds all three. `regex` already refuses, at the call, a
pattern flag the pattern text cannot carry, "because silently dropping an option would change what
the pattern means". `literal` already says "nothing else is inferred". `execute` already scopes its
every-variant-runs guarantee to "a closure that throws or rejects".

## Decision

**1. A header name that collides with another once lower-cased is refused at the call**, before any
document is built. The SDK made the collision by lower-casing, so only the SDK can report it; the
engine never sees the declaration that would be dropped. An author who means one header with two
values writes the list form, which the rule already carries.

**2. A value that is not a string is written as the one string that spells it, and only where every
language spells it the same way**: a whole number whose magnitude is at most 2^53 − 1 becomes its
shortest decimal form (no exponent, no trailing zeros, no leading `+` — so `3` and `3.0` are one
value, both written `"3"`), and a boolean becomes `true`/`false`. Every other value is refused at the
call: a number with a fractional part or a larger magnitude, a date or a time, `null`, a map, a list
holding any of these. The same rule applies to each element of a list.

The bound is where the languages stop agreeing, not an arbitrary limit: 2^53 − 1 is the largest
integer JavaScript represents exactly, and above it a `long` and a `number` no longer spell the same
value the same way.

This is not the coercion `method` and `path` forbid. Those are values the engine judges as the author
wrote them, so repairing one hides a mistake the engine would have reported. A header or query value
reaches the transport as a string and nothing else (shape spec §3.6) — which is why this rule already
writes a bare string as a one-element list — so writing `3` as `"3"` is completing a conversion the
rule was always performing, not overruling the engine.

**3. An engine error is not a closure's verdict.** When any operation `execute` sends answers with an
error, including a `serve-variant` inside the loop, `execute` fails at once with that error and the
variants after it do not run. The every-variant-runs guarantee is about what a *test* decided.

Spec text: [behavioural specification](../specs/sdk-specification/behavioural-spec.json), primitives
`request` (1, 2) and `execute` (3), with five new conformance ids and their cases.

## Alternatives considered

- **Merge colliding header names into one list** (`Accept: "a"` + `accept: "b"` → `["a", "b"]`).
  Rejected: it is HTTP-correct and lossless, but it repairs an ambiguity rather than reporting it,
  and the DSL already has an unambiguous spelling for a two-valued header. An author who wrote the
  same header twice more often has a bug than a list.
- **Last declaration wins** (TypeScript's behaviour, made normative). Rejected: it discards something
  the author wrote and tells nobody — the failure mode every honesty rule in this project exists to
  prevent (contract spec §2.2).
- **Stringify anything the language can** (`String(v)` / `String.valueOf(v)`). Rejected on one
  concrete fact: the languages disagree. `1.0` is `"1"` in JavaScript and `"1.0"` in Java, `1e21` is
  `"1e+21"` and `"1.0E21"`, a date is a locale-dependent sentence in one and ISO-8601 in the other,
  and an object is `"[object Object]"` in one and a field dump in the other. Two conformant SDKs
  would then send different bytes for the same test and record different bytes in the contract —
  reintroducing, in the ergonomic layer, exactly the divergence B1 exists to remove.
- **Specify an exact decimal algorithm so fractional numbers can convert too** (shortest
  round-tripping form, no exponent, within a stated range). Rejected for now, not on principle: it is
  implementable, but it is real spec surface for a case nobody has asked for, and refusal is the
  reversible direction — accepting more values later is additive, while a spelling that two SDKs have
  already written into contracts cannot be taken back.
- **Pass an unspellable value through and let the engine reject it** (SDK spec §2.1's "rejecting is
  the engine's call"). Rejected because the engine does *not* reject it: the document is well formed,
  so the shape is simply unmatchable and the variant never verifies, with nobody saying why. §2.1's
  rule assumes the engine will speak; where it cannot, the SDK must.
- **Treat a mid-loop engine error as that variant's failure** (the JVM's behaviour, made normative).
  Rejected: it reports N test failures for one machinery failure, and says the consumer's tests
  failed when the engine did. It also buys nothing — the closure has already not run on those
  variants either way.

## Consequences

Easier: a test author gets `X-Count: 3` from `header("X-Count", 3)`, which is what they meant, and an
error at the call for everything the SDK cannot spell for them — both failures arrive at the line
that caused them rather than as a variant that mysteriously never verifies. A dead engine now reports
itself once, in its own words.

Harder, and committed to: both SDKs change (TypeScript gains the refusals and the conversion and
widens its `MultiValue` type; the JVM gains the conversion and stops swallowing engine errors into
variant failures), and the conversion table is now a compatibility surface — widening it later is
additive, narrowing it is not. Every language added after this must implement the integral-number
spelling by hand where its default formatter disagrees.

**Tripwire** — revisit if an SDK maintainer reports that refusing fractional numbers in header and
query values is a real ergonomic problem in practice (argues for the exact-decimal-algorithm
alternative, additively); if a transport arrives whose values are not strings, so that "reaches the
transport as a string and nothing else" stops being true and the conversion belongs to the transport
rather than to `request` (design 2.6); or if a language appears in which refusing at the call is not
expressible as a test-time failure, which would make decision 1 unimplementable there and reopen the
merge alternative.
