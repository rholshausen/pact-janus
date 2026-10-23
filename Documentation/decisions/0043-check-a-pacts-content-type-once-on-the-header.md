# 0043 — Check a pact's Content-Type once, on the header

- **Status**: accepted
- **Date**: 2026-10-23
- **Plan tasks**: S1.26
- **Evidence**: pact-reference's `pact_matching::match_body` at the version `pact_verifier_cli` 1.3.5
  uses, and `pact_models`' `ContentType::is_equivalent_to` and its `Display`; pact-jvm's
  `Matching.matchBody`; S1.1's fixtures `bodies/content-type-rule-with-a-space` and
  `bodies/content-type-extra-parameter`; ADR 0036 decision 4

## Context

When a pact names a `Content-Type` header, today's Rust verifier checks the reply's type twice. First
it checks the header like any other: by the header's matching rule if it has one, otherwise as a MIME
type whose parameters are compared as a set, extra ones allowed. Then, before it compares the body,
`match_body` checks again. The two types agree when either is unknown, or when they are equivalent.
If the pact's type has no parameters, that means the same type and subtype. If it has some, the
reply must have exactly the same parameters. Only when they do not agree does it try the header's
rule, against both types as `pact_models` writes them: no space before a parameter, and the charset
in lower case (`application/json;charset=iso-8859-1`). A reply that fails this second check fails
with `Expected a body of '…' but the actual content type was '…'`.

So today fails replies that its own header check has just accepted:

- **a rule written with a space.** The rule `application/json; charset=.+`, the way a header is
  usually written, allows `application/json; charset=iso-8859-1` on the header. Against the pact's
  `charset=utf-8` the types are not equivalent, and the rule does not match the form without the
  space.
- **a reply that adds a parameter.** With no rule, `application/json; charset=UTF-8; version=2` passes
  the header check against the pact's `application/json; charset=UTF-8`. The parameters are not the
  same, and there is no rule to fall back on.

pact-jvm compares base types (type and subtype) before it compares a body, and has no second rule
check, so it passes both. Janus checks the pact's `Content-Type` once, on the header
(`legacy:header-value`, or the rule), and compares a body as the pact's type. It emits
`legacy:body-type` only when the pact names no `Content-Type` header (ADR 0036 decision 4). It passes
both too. S1.1's harness records both cases as disagreements.

## Decision

1. **Janus checks a pact's `Content-Type` once, on the header**, as ADR 0036 decision 4 has it. It
   does not copy today's second check: a reply whose header the pact's header check accepts is not
   then refused as a body of another type.
2. **The difference is deliberate**, and the ledger says so. `bodies/content-type-rule-with-a-space`
   and `bodies/content-type-extra-parameter` carry this ADR as their reason.

## Alternatives considered

- **Copy the second check into the legacy compiler.** It would remove two ledger entries. But it
  would make Janus refuse what its own header check accepts, on a difference of spelling (a space)
  or of an extra parameter nobody asserted. Today's two checks disagree there, and pact-jvm sides
  with the first.

## Consequences

- A provider moving from `pact_verifier_cli` sees these replies pass where they failed. A provider
  moving from pact-jvm sees no change. No reply that passes today fails under Janus because of this.
- When the pact names no `Content-Type` header, the type it gives its body is still checked as today
  checks it, parameters included (`legacy:body-type`, ADR 0036 decision 4).
- S1.1's ledger keeps four entries for the two fixtures, with this ADR as their reason.
- **Tripwire**: if a pilot provider relies on the second check, to catch a reply whose charset or
  parameters changed under a rule that allows them, the legacy compiler copies it after all.
