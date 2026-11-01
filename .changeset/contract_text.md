---
engine: minor
---

# A contract crosses the engine boundary as its text

A host that declares the `contract-text` capability in its hello gets `consumer-session/finalise`'s
and `upgrade/pact`'s contract as `contract-text`: the contract's canonical bytes as a string, its
trailing LF included, in place of the `contract` document (ADR 0063, engine-protocol spec §8.9). A
host that does not declare it gets the document, as before. Wherever a request carries a contract or
a v1–v4 pact (`verification/verify`'s inline `contracts`, `upgrade/pact`'s `pact`,
`subsumption/check`'s `contract`, `broker/publish-contracts`'s `contracts`), it may now carry the
file's text, which the engine reads as it reads a file. Contract-file spec §2.4 now says how a number
and a string are written. That is what the engine already wrote, pinned by `corpora/contracts/`.
