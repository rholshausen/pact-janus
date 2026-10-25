# 0051 — Leave pending out of the contract, and let upgrade exclude pending interactions

- **Status**: accepted
- **Date**: 2026-10-25
- **Plan tasks**: S1.28; S1.3 (the broker's pending pacts, which this leaves pending to); S1.19 (which
  found it, through S1.1's upgraded column)
- **Evidence**: [pact-specification#73](https://github.com/pact-foundation/pact-specification/issues/73),
  where the v4 flag was proposed and argued; pact-reference's `pact_verifier` (`verify_pact_internal`,
  which keeps a pending interaction's failure out of the run's result, and `publish_to_broker`,
  which publishes `success: false` when any interaction failed, pending or not);
  `corpora/verifier/compatibility-suite/v4/01-verifying-a-pending-http-interaction`;
  `engine/kernel/src/protocol/verification.rs`, where a source's broker pending status applies to a
  contract as to a pact

## Context

A v4 interaction may be marked `pending`. Today's verifier reports its failure and does not fail the
run, as it does for a pact the broker says is pending. `janus verify` does the same for a pact. A
contract has no such member, so `janus upgrade` converted a pending interaction without its flag and
said so (`pending-dropped`), and a failure of the upgraded interaction failed the run. S1.1's
upgraded column found the difference, and S1.28 had to decide whether pending belongs in the
contract.

The flag was proposed in pact-specification#73 so that the consumer team, who know what is still work
in progress, could say so in the pact rather than leave it to the broker. The cases argued there were
trunk-based development, where a consumer records interactions for a feature that is not switched on
yet; a client written and tested before the provider has built the endpoint; teams that share pact
files with no broker; and interactions meant only for stubs.

Three things bear on it:

- **Pending is a fact about two codebases at a moment.** It means "this provider has not yet
  satisfied this". It stops being true when the provider catches up, and nothing in the consumer's
  build learns that. A flag in the consumer's tests stays until someone remembers to take it out, and
  until then a regression in that interaction never fails the provider's build. The broker works
  pending out from the provider's results instead, and a pact stops being pending the first time the
  provider's main branch verifies it. A contract version's identity is its bytes (contract-file spec
  §2.3), so a flag in it cannot follow the provider either.
- **The flag only shields the provider's build.** `pact_verifier` publishes `success: false` for a
  pact any interaction failed in, pending or not, so a consumer's `can-i-deploy` stays blocked until
  the provider satisfies it. That is right for a feature toggle, which can be switched on without a
  deploy. The provider's build is already shielded by the broker: a new interaction is new pact
  content, pending until verified, and the provider's deployed and released selectors keep the
  interactions it already satisfies failing the build.
- **Janus already has the broker's pending for contracts.** A source the broker says is pending is
  pending whether it fetched a pact or a contract (S1.3).

## Decision

1. **A contract has no pending interactions.** Contract-file spec §4.1 says so, and no reader accepts
   a `pending` member as meaning anything. Pending for a contract is the broker's to decide, from the
   provider's results, and a verifier honours it as it does for a pact.
2. **The v1–v4 path keeps honouring the flag.** `janus verify` on a pact reports a pending
   interaction's failure without failing the run, as today's verifier does. That is what S1.1
   measures, and a provider switching verifiers keeps it.
3. **`upgrade/pact` converts a pending interaction without its flag by default**, as `pending-dropped`,
   whose message now says that pending is the broker's for a contract, and that the interaction can be
   left out instead.
4. **`upgrade/pact` takes `options: { "exclude-pending": true }`, and `janus upgrade` takes
   `--exclude-pending`**, to leave every pending interaction out of the contract. Each is a
   `pending-excluded` finding, `lossy`, pointing at the interaction in the pact and naming it. It is
   for a pact whose pending interactions are known to fail: without it, the contract fails where the
   pact passes. An interaction left out is not counted again as `interaction-dropped`, whatever its
   kind.
5. **The S1.1 upgrade ledger keeps the pending case**, with this ADR as its reason. The harness
   upgrades with the default, so the entry records the difference decision 3 accepts.

Upgrading also now points every finding at the interaction's position in the pact. It used to count
only the HTTP interactions, so a finding after a message or a plugin interaction pointed at the wrong
one. That changes the suffix `duplicate-description` gives a renamed interaction in such a pact.

## Alternatives considered

- **A `pending` member on a contract's interaction.** The placement rule (§3.2) would put it there,
  since it changes what a verifier does. Rejected for the first point of the context: it is written
  once, by the side that cannot see when it stops being true.
- **A `pending` member with an expiry date**, like a `janus check` exemption (ADR 0016). It would make
  the flag go stale loudly. But the date is a guess at when another team finishes, and the broker
  already knows when they have.
- **Exclude pending interactions by default.** The upgraded contract would always verify as the pact
  does. But it would say nothing about interactions the pact does describe, without being asked, and
  once the provider satisfies them nothing would add them back. Converting them is the honest default.
  Leaving them out is a choice the author makes, knowing they fail.
- **A verifier option naming the interactions to treat as pending**, for teams with no broker. It
  keeps the decision with the team whose build it affects, but no pilot has asked for it. It is the
  answer if one does, not a contract member.

## Consequences

- A contract describes interactions, not where two teams are in their work. A pending pact keeps its
  meaning, and a pending contract is decided where pending pacts are.
- A provider that upgrades a pact with pending interactions chooses between a contract that fails
  where the pact passes and one that leaves them out. Both are said in the findings. Upgrading is a
  bridge: the consumer's own suite, run under Janus, records the contract that replaces it.
- `upgrade/pact` reads its `options` for the first time. `pending-excluded` joins the finding
  vocabulary, which is open, so a reader that does not know it displays it.
- **Tripwire**: if pilot providers verify upgraded contracts without a broker and need a pending
  interaction to report without failing, the verifier option above is the change, not a contract
  member.
