# 0083 — Compare `Accept` and `Content-Type` as media types, and every other header's parameters as text

- **Status**: accepted
- **Date**: 2026-11-11
- **Plan tasks**: S3.4 (its remainder after S2.16)
- **Evidence**: `pact_matching::headers::match_header_value`, which reads parameters only in
  `pact_models::headers::PARAMETERISED_HEADERS` (`accept`, `content-type`), and compares a header with
  several values element by element; S1.1's fixtures `headers/parameters` and `headers/parameters-exact`,
  run against pact_verifier_cli 1.3.5; the pact-specification header cases
  (`matches content type with charset with different case`, which names `Accept` too, and
  `header value is different case`, whose `Accept` is `alligators`);
  [ADR 0071](0071-compare-a-content-type-as-a-media-type-with-the-http-components-http-media-type.md)
  and [ADR 0082](0082-compare-a-list-valued-header-as-its-elements-when-the-contract-says-it-is-a-list.md),
  which this extends and corrects

## Context

S3.4 was "`janus upgrade` without the media-type narrowing", and S2.16 did it for `Content-Type`
(ADR 0071). ADR 0082 decision 5 left a `rule-narrowed` judgement on any other header whose value has
parameters, on the premise that v1–v4 compare any such value's parameters as a set. The legacy
compiler's `legacy:header-value` did, for every header.

Today's verifier does not. It reads parameters in two headers only, `Accept` and `Content-Type`. Any
other header is text, compared exactly but for whitespace around a comma. And it compares `Accept`,
which `pact_models` splits into elements, element by element, each as a media type. S1.1's harness,
given two new fixtures, found three differences:

- `X-Params: a;b=1` against a reply's `a;b=1;c=2`: today's verifier fails it, and `janus verify` of the
  pact passed it.
- `Accept: application/json, text/plain` against `application/json;q=1, text/plain`: today's verifier
  passes it, and `janus verify` of the pact failed it, comparing the joined text as one media type.
- `Accept: application/json` against `application/json;q=0.9`, or a charset in another case: today's
  verifier passes it, and the upgraded contract, `equality`, failed it.

So the upgrade still narrowed `Accept`, and its judgement was given for headers it did not narrow.

## Decision

1. **The legacy compiler compares a header with no rule as today's verifier does.** `Accept` and
   `Content-Type` compile to `legacy:header-value`, which now compares each comma-separated element
   with the one at its index, as many on each side, each as a media type where either has parameters,
   and exactly otherwise. Every other header compiles to a new legacy action, `legacy:header-text`: its
   text, exactly, ignoring whitespace around a comma, a parameter compared as any other text.
2. **`janus upgrade` writes an `Accept` of one media type as `http:media-type`**, as it writes a
   `Content-Type` (ADR 0071), with the same `rule-narrowed` note, and the interaction requires `http`. An
   `Accept` that is not a media type (`alligators`) stays `equality`, as v1–v4 compare it exactly.
3. **The upgrade's parameters judgement is left only on an `Accept` of several elements**, which is
   `http:list` (ADR 0082) and compares each element exactly, where v1–v4 compare each as a media type,
   extra parameters allowed. It is given whether or not the pact's elements have parameters, since the
   reply's may. Another header with parameters is `equality`, or `http:list`, with no judgement: it is
   compared as v1–v4 compare it.

This amends ADR 0082 decision 5, whose premise was the legacy compiler's reading rather than today's
verifier's.

## Alternatives considered

- **Element shapes inside `http:list`, so an `Accept` list is a list of media types.** It removes the
  last judgement, but it is the alternative ADR 0082 declined: a component operator is a leaf, and a
  shape per element is a shape-language change, for one header whose elements rarely carry a quality
  in a pact. The judgement says where it differs, and ADR 0082's tripwire covers it.
- **An `array` of `http:media-type` entries for an `Accept` list.** It compares by index, as v1–v4 do,
  but over field lines: `a, b` on one line would fail it, which is the framing ADR 0082 removed.
- **Keep `legacy:header-value` for every header and tell it the header's name.** One action with a
  mode argument changes every plan the corpus pins; a second action changes only the plans with a
  header other than these two, and names the comparison each makes.
- **Compare the media type's type and subtype case-sensitively, as today's verifier does.** The legacy
  compiler and `http:media-type` both ignore their case, which RFC 6838 makes insensitive. ADR 0071
  accepted that difference for `Content-Type`, and it is the same for `Accept`.

## Consequences

Easier: `janus verify` of a pact agrees with today's verifier on parameters in any header, and an
upgraded contract's `Accept` passes the qualities and charsets its pact passes. A judgement now
means the contract compares something more narrowly.

Harder: a pact that passed `janus verify` with a parameter its custom header did not name now fails,
as it does with today's verifier. An upgraded pact with an `Accept` requires the `http` component.

Committed to: `legacy:header-text` in the plan grammar's legacy-only actions (plan-grammar spec §4.4).

Tripwire: ADR 0082's. If consumers need a shape on a list header's elements, an `Accept` list of media
types is the first case for it.
