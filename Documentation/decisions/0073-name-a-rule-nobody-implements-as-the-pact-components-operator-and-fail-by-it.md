# 0073 — Name a rule nobody implements as the `pact` component's operator, and fail by it

- **Status**: accepted
- **Date**: 2026-11-08
- **Plan tasks**: S2.17, found by S2.5b
- **Evidence**: pact-jvm's `MatchingRules.kt` at 4.7.8 (`ignore-order`, `min-ignore-order`,
  `max-ignore-order`, `min-max-ignore-order`, the rules `unorderedArray` and its sized forms write);
  `pact_models`' `MatchingRule::create`, which reads a name it does not know as `MatchingRule::Plugin`;
  `pact_verifier_cli` 1.3.5, which fails such a rule (`No catalogue entry found for key
  'ignore-order'`); the S1.1 fixture `matching-rules/ignore-order`; shape spec §3.5 and §3.7;
  [ADR 0045](0045-refuse-an-interaction-that-needs-a-pact-plugin-by-name-and-verify-the-rest.md)

## Context

pact-jvm writes rules that pact-reference does not have. The one S2.5b met is `ignore-order`, which
pact-jvm's `unorderedArray`, `unorderedMinArray`, `unorderedMaxArray` and `unorderedMinMaxArray` write
on an array, with a `min`, a `max` or both. `pact_models` reads a rule name it does not know as a
plugin's (`MatchingRule::Plugin`, with the rule's other members as its configuration). When the pact
names a plugin, ADR 0045 refuses the interaction before anything runs. When it names none, as
pact-jvm's do not, the rule is nobody's. Today's Rust verifier then fails every value the rule reaches,
and the legacy compiler does the same with a `flow:error` node, so `janus verify` agrees with it.

`janus upgrade` did not. On an array it wrote an `each-like` of the first element, the template it
makes for a type rule, and reported nothing there. With no rule of their own, the elements were held to
the first one by equality. So a contract of `["red", "green", "blue"]` admitted `["red", "red"]` and
not its own example, and one whose elements' members had rules of their own admitted replies that
today fail. Contract-file spec §8.2 says a rule with no shape equivalent is `rule-unmapped`, never a
silent guess. On a scalar, `upgrade` did report it, and kept the example as `equality`. That contract
passes the example, which today fails.

The contract could keep the example, as a scalar's did, or fail as the pact does. Keeping the example
gives a contract that verifies, but it passes replies today's verifier fails. A provider moving from
today's verifier to a contract would see it pass what used to fail, which is the direction the
migration must not go.

## Decision

1. **A rule `pact_models` reads as a plugin's, in a pact that names no plugin, becomes the `pact`
   component's operator of the same name**: `{ "shape": "pact:ignore-order", "min": 1, "example": [...] }`.
   The `pact` namespace is the v1–v4 vocabulary's, which the built-in generator component already
   names its generators in (`pact:RandomInt`, ADR 0048). The rule's configuration becomes the node's
   members, as a component operator's do (shape spec §3.5), except any named `shape`, `example`,
   `encoded` or `generator`. The example is the position's whole value: an array's is the array.
2. **Nothing below it is walked.** The operator is the position's whole shape, as the rule is the
   whole of what the pact asks there. Rules the pact declares on the array's elements are not carried.
3. **No requirement is written.** A contract's requirements are checked before a run starts, and one
   nobody satisfies ends the run with nothing verified. With none, the run starts, and the node fails
   its interaction alone: `no component in scope contributes the action 'pact:ignore-order'`. The
   rest of the contract verifies, as the rest of the pact does. This is §3.7's named failure.
4. **It is `rule-unmapped`, `lossy`, reported once, where the rule lands.** The finding names the
   operator and says to rewrite the position as a shape that says what the rule asked for.
5. **As one alternative of an `OR`, such a rule is left out** when any other alternative is left.
   Today's verifier fails it wherever it is applied, so as an alternative it never matches and
   decides nothing, and nothing is reported. An `AND` that holds one, or an `OR` of nothing else,
   becomes the operator.
6. **The same holds at every position `upgrade` writes a rule for:** a body's values and arrays, and
   headers, query parameters, a path, and text, form, XML and multipart values. An XML element's own
   rules are unchanged: one that is nobody's is dropped and reported there, as before.
7. **A consumer test's mock follows from the contract.** The facade converts pact-jvm's interactions
   as `upgrade` converts a pact (ADR 0066), so a request that uses `unorderedArray` is refused by name
   at the mock, and a response that uses it is served as its example and fails at verification.

## Alternatives considered

- **Keep the example**, an exact `array` of the pact's elements, as a scalar's was kept. The contract
  passes the pact's own order, which today fails, and the facade's mock accepts it, so pact-jvm's
  `ArrayUnorderedTest` keeps passing. Rejected: it passes what today's verifier fails, and each such
  verdict would sit in S1.1's ledger for good.
- **Write a requirement for the rule**, as `pact-plugin/<name>` (ADR 0045's spelling) or `pact`. One
  nobody satisfies ends the whole run before anything is verified, so one rule would stop every
  interaction in the contract. `pact` is satisfied by nothing a run loads today either.
- **Implement `ignore-order`**, as pact-jvm does. It would be a new matcher with pact-jvm's semantics,
  which the shape language cannot say (`contains` needs a distinct element for each entry and bounds no
  length), for a rule today's Rust verifier does not have. It can still come later as the `pact`
  component's operator, and contracts written now would then verify by it unchanged.

## Consequences

- An upgraded contract with `ignore-order` fails where the pact fails, and S1.1's fixture
  `matching-rules/ignore-order` holds the two to the same verdicts.
- pact-jvm's `ArrayUnorderedTest`, which puts `unorderedArray` in a request, is refused by name at the
  facade's mock and no longer runs there. The migration note's row stays `dropped`, and says to
  rewrite with `eachLike` or `arrayContaining`.
- A contract can now name an operator nothing provides with no requirement for it. Only `upgrade`
  writes one, and only in the `pact` namespace.
- An engine that someday provides `pact:ignore-order` verifies these contracts by it, with no change
  to them.
- **Tripwire**: if pilot consumers on pact-jvm use `unorderedArray` widely, implementing it as
  `pact:ignore-order` is the step, not a different conversion.
