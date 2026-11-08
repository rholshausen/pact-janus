---
engine: major
---

# Contracts and reports never repeat a credential

A contract the engine writes records the values of members that carry credentials as `"[redacted]"`
(a number as `0`, a boolean as `false`), and lists their paths in the interaction's new `redacted`
member (contract-file spec §5.6, ADR 0074, staged plan S2.8). Which members those are is the
transport's to declare, as `credential-members` in its contribution: the built-in HTTP transport
declares the request's `authorization`, `proxy-authorization` and `cookie` headers and the response's
`set-cookie`. Under a redacted member an `equality` becomes a `type`, so the contract says a value of
that kind was sent; patterns and bounds are kept. An interaction with no credential is written exactly
as before.

`consumer-session/create` takes `config.redaction`, `{ add, keep }`, each a list of paths
(`request.headers.x-api-key`): `add` redacts more members and `keep` records a fixed token as it is. A
path that does not parse fails the create with `malformed-frame`. `upgrade/pact` redacts the HTTP
transport's members and reports each as `value-redacted`; `add-pact-interaction` is redacted by its
session's choice.

A verifier sends what was recorded. A provider that checks credentials supplies its own with a
`before-request` hook or the HTTP transport's `headers`; a failed result names each redacted request
member no hook replaced, as `redacted`, and the report says how to supply it.

No report repeats a value found under a credential member, in the interaction, the request sent or
the reply: mismatches, `verification/executed-plan` and the plan beside it, `verification/explain`'s
`text`, `plan` and `executed`, and a consumer session's mismatches each show `[redacted]` instead.

This is `major`: a provider that relied on a contract replaying the token its consumer recorded must
now supply one, or its consumer keeps the header.
