# 0041 — Refuse a filter that names no recorded variant, and report a narrowed run as filtered

- **Status**: accepted
- **Date**: 2026-10-23
- **Plan tasks**: S1.17 (the build); S1.13 (the `--filter-*` options, which this decides for); S1.14
  (the provider side of the SDK specification, where a conformance case for it belongs)
- **Evidence**: variant-semantics spec §5.1; engine-protocol spec §8.3, §8.6, §9.6; pact-reference's
  `pact_verifier` (`verify_pact_internal`, `publish_to_broker`); pact-jvm's `ProviderVerifier`
  (`runVerificationForConsumer`) and `FilteredPact.isFiltered`; `pact_verifier_cli` 1.3.5, the version
  S1.1 pins, run with a `--filter-description` that matches nothing

## Context

Variant-semantics spec §5.1 lets a verifier narrow a run to some recorded variants, and says it "MUST
report a filtered run as filtered — a partial run is not a pass". `janus verify --variant <id>` did
report the run as filtered, but its status was `verified` whenever nothing failed. So an id that no
document records replayed nothing and reported `VERIFIED: 0 verified, 0 failed, 0
state-unavailable, 2 skipped` with exit 0. A typo looked exactly like a narrowed run that passed.

S1.3 made this worse. A run from a broker can publish its result, and the result published for a
pact was `success: true` whenever none of its results failed. A filter that matched nothing left no
results, so it published a success for a pact nobody had verified, and `can-i-deploy` would trust it.

Today's verifiers disagree with each other on both points:

- `pact_verifier_cli` (1.3.5) verifies only the interactions its `--filter-*` options match. When
  they match nothing, it prints the pact's header, reports nothing else, and exits 0. It publishes
  the results it has, so a pact the filter emptied is published as a success.
- pact-jvm judges a pact filtered when the filter excluded at least one of its interactions. It does
  not publish a filtered pact's result, and says so ("Skipping publishing of verification results as
  the interactions have been filtered"). A filter that matches nothing reports the pact as having
  no interactions, and passes.

Neither refuses a filter that matches nothing, and both exit 0 for a filtered run that passed.

## Decision

1. **A filter naming a variant that no document in the run records is refused before anything
   runs**, as `variant-not-found`, naming the id and the ids the documents record. For an inline
   source, `verification/verify` answers with the error. A broker source's documents are fetched in
   the run, so its run ends right after `verification/fetched`, with `status: failed` and the error,
   as a fetch that finds no pacts ends. No hook runs, no request is sent, and nothing is published.
   A filter that matches nothing is always a mistake, and this is where the author can fix it, as
   §6.5 rejects a binding that names no dimension. A filter with an empty list of ids is
   `malformed-frame`. A broker run that fetched no documents, and was told that is fine
   (`require-pacts: false`), is not refused for its filter: it has nothing to narrow.
2. **A run is filtered when its filter excluded at least one recorded variant**, not merely when it
   was given one. This is pact-jvm's rule. `--variant base` on a v1–v4 pact, whose every interaction
   records only `base`, replays everything, and is a whole run.
3. **A filtered run that nothing failed has the status `filtered`, not `verified`.** The run's
   `status` is `failed` when anything not pending failed or a hook aborted it, then `filtered` when it
   was narrowed, then `verified`. Each `pairs` entry follows the same rule for its own document:
   `failed`, then `incomplete`, then `filtered`, then `verified`. `filtered` is in the summary and
   in `verification/started`, as it was, with decision 2's meaning.
4. **A pact the filter narrowed is never published.** Its `verification/published` event and its
   `publication` entry have the outcome `skipped`, with `reason: "filtered"`. A pact in the same run
   that the filter did not narrow is published as usual. This is pact-jvm's behaviour, per pact.
5. **`janus verify` exits 0 for a filtered run, and prints its status as `FILTERED`.** The exit code
   says the command did what it was asked. A filter that matches nothing does not get this far:
   from files it exits 2, as any refused request does.
6. **`subsumption/decide` reads a pair's `filtered` status per pair.** Its `verification-filtered`
   reason, a `warn`, is raised for a pair whose entry says `filtered`. For a summary from before
   `pairs`, it is raised from the run's `filtered`, as before.

## Alternatives considered

- **Exit 1 for a filtered run that passed.** It follows "a partial run is not a pass" to the exit
  code, and a `--variant` left in a CI script could never pass a build. But a narrowed run did what
  it was asked, and both of today's verifiers exit 0 for it, so the `--filter-*` options S1.13 adds
  would break scripts that use them. The cases that matter are covered elsewhere: decision 1 refuses
  a filter that matches nothing, decision 4 keeps a narrowed result off the broker, and the summary
  `decide` reads says `filtered`.
- **Refuse `publish` with a filter, before the run.** It is simpler, and an inline `publish` is
  already refused this way. But it also blocks a pact the filter did not narrow, which has a whole
  result to publish.
- **Publish a narrowed pact's result as a failure.** It is never a false pass. But it tells the
  broker a pact failed when nothing failed, and a provider's `can-i-deploy` would then block on a
  debugging run.
- **"Filtered" whenever a filter is given.** It is simpler to compute. But `--variant base` over v1–v4
  pacts would skip publishing for runs that verified everything.

## Consequences

- A mistyped `--variant` fails at once and names the ids the documents do record. It can no longer
  read as a pass, or be published as one.
- A host that reads a run's `status` must read `filtered` as "not verified, not failed". The value
  is new in the run's and in `pairs`' status, and an engine pin moving to it needs to know.
  `publication` outcomes gain `skipped`.
- `janus verify` differs from today's verifiers in two places, both deliberate: a filter that matches
  nothing is an error, not a pass, and a narrowed pact is not published, which `pact_verifier_cli`
  does and pact-jvm does not. S1.13's rows for `--filter-description`, `--filter-state`,
  `--filter-no-state` and `--filter-consumer` carry this ADR as their reason, and S1.1's harness
  cannot compare the two until `janus verify` takes those options.
- A broker run that ends with `variant-not-found` exits 1, as a broker run that finds no pacts or
  cannot fetch does, while the same refusal for files exits 2. Every engine error that ends a broker
  run is read the same way, and the CLI output review after stage 1a decides whether that is right.
- **Tripwire**: if pilot providers run narrowed verifications in CI and want them to fail the build,
  decision 5 becomes an option, not a change of default.
