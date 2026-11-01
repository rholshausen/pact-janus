---
typescript-sdk: patch
---

# The contract written is the engine's text, and pacts are sent as read

`finalise` writes the contract text the engine answers, byte for byte, where it wrote
`JSON.stringify` of the document. A JavaScript parse had respelled numbers (`1.0` as `1`, `1e16` as
`10000000000000000`) and rounded an integer above 2^53, so a contract with one could be written
changed. The `Verifier` sends each pact file's text unparsed, so a provider is verified against what
the file says (ADR 0063). Both need an engine with the `contract-text` capability, which the SDK's
pinned engine has. An older engine named by `JANUS_ENGINE` fails `finalise`, saying so, rather than
reporting a withheld contract.
