---
jvm-sdk: patch
---

# The contract written is the engine's text, and pacts are sent as read

`finalise` writes the contract text the engine answers, where it copied the contract's bytes out of
the response frame, which only worked while frames were JSON. The `Verifier` sends each pact file's
text unparsed (ADR 0063). Both need an engine with the `contract-text` capability, which the SDK's
pinned engine has. An older engine named by `JANUS_ENGINE` fails `finalise`, saying so, rather than
reporting a withheld contract.
