---
engine: patch
---

# A converted interaction's body is served as it was written

An interaction from `add-pact-interaction` whose body no generator fills is served as the octets the
library wrote, as today's mocks serve it, where it was re-encoded from its document: a JSON reply
written with spaces keeps them (ADR 0066 decision 4, as amended). A body a generator fills is still
decoded, so the generator fills it fresh. An HTTP mock started on an IPv6 address answers a
`base-url` with the address in brackets, where it had none.
