---
engine: patch
---

# The same interaction selects its variants in the same order every run

`janus-ipog-v1` took the targets it had not yet covered in the order of a hashed set, which differs
from run to run, so an interaction with several dimensions could select its covering variants in a
different order on a different run, or in a different SDK. A contract records variants in the order
they ran, so the same consumer test could record a contract with different bytes, and a broker
would see a new version. Selection now takes the targets in the order variant-semantics spec §3.4
states. A contract recorded before this may list its covering variants in another order, once.
