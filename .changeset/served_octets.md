---
engine: patch
---

# A served example's octets are octets

An example that is octets is served as those octets, where it was served as its base64 text
(staged plan S2.15). A slot whose shape's example is tagged `base64` is generated as octets, so
`serve-variant` answers a message's octets as a `base64` slot, an HTTP mock replies with them, and a
recorded variant records them as `base64`, which a verifier replays as the octets themselves. An
interaction from `add-pact-interaction` given a body no content component reads, recorded as
`base64` or as UTF-8 `text`, is served as those octets too. A generator's fresh values are still
filled everywhere else in the payload. Inside a document, such as a JSON body's member, a
`base64` example is still its base64 text, since JSON has no bytes.
