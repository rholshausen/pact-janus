---
default: minor
---

# `upgrade/pact` can leave pending interactions out of the contract

A Janus contract has no pending interactions: whether a provider has yet to satisfy one is the broker's
to decide, from the provider's results, as it decides a pending pact (ADR 0051). `janus verify` keeps
honouring a v4 pact's `pending` flag, as today's verifier does.

- **`upgrade/pact` reads `options.exclude-pending`.** With `true`, each pending interaction in the pact
  is left out of the contract and reported as a new finding, `pending-excluded` (`lossy`), pointing at
  the interaction and naming it. Without it, nothing changes: each is converted without its flag, and
  reported as `pending-dropped`, whose message now says how to leave it out instead. An interaction
  left out is not counted again in `interaction-dropped`.
- **`janus upgrade --exclude-pending`** sends it, for a pact whose pending interactions are known to
  fail, whose upgraded contract would otherwise fail where the pact passes.
- **Every finding points at the interaction's position in the pact.** Upgrading used to count only the
  HTTP interactions, so a finding for one after a message or plugin interaction pointed at the wrong
  one, and the suffix `duplicate-description` gave a renamed interaction in such a pact was off by as
  many.

The protocol's `upgrade.schema.json` now lists every finding code the contract-file spec does, and the
generated bindings carry the new option.
