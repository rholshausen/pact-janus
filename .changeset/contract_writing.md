---
engine: minor
---

# The engine's host writes the contract file, merging test files into one

`consumer-session/finalise` takes `write: { path, mode? }` (ADR 0065). Behind the new
`contract-writing` capability, which `janus-engine` declares, the engine writes the contract to the
absolute `path` and answers `written` in place of `contract` or `contract-text`. With `mode: "merge"`,
the default, it joins the file's interactions and the session's by identity (a description with its
states) under an exclusive lock on the file, so test files run in separate workers or processes write
one contract. An interaction recorded differently, or a file holding another consumer's or provider's
contract, is the new `contract-conflict` error, naming each interaction, and the file is left as it
was. `mode: "overwrite"` replaces the file. A withheld contract writes nothing.

Every contract's interactions are now written in identity order: by description, then by states
(contract-file spec §2.4). A contract with several interactions, recorded or upgraded, changes its bytes
once, and `janus explain --index` counts in that order.
