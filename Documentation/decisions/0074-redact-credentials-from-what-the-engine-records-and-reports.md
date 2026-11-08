# 0074 — Redact credentials from what the engine records and reports

- **Status**: accepted
- **Date**: 2026-11-08
- **Plan tasks**: S2.8; X.4 (secrets in recorded artifacts, cross-cutting security)
- **Evidence**: contract-file spec §5.4, which has a verifier replay a variant's recorded request;
  lifecycle-hooks spec §7.3, the redaction rules for a project's interpolated secrets; RFC 9110 §11.6.2
  and §11.7.2 (`Authorization`, `Proxy-Authorization`) and RFC 6265 §4 (`Cookie`, `Set-Cookie`);
  pact-reference, pact-js and pact-jvm, none of which redacts anything from a pact or a report

## Context

A contract records each variant's request as the test declared it, and a verifier sends that request
to the provider (contract-file spec §5.4). A consumer test that declares the token its client sends,
read from the environment or fetched from a test identity provider, records the token, and the
contract is then published to a broker that many people read. A token that changes every run also
gives the contract a new hash every run.

Reports leak the same way. An executed plan shows every value it resolved, a mismatch message quotes
the value it rejected, and a provider's `Set-Cookie` or a signed request's `Authorization` lands in CI
logs through `explain --executed`, `--explain-failures` and the result events.

Lifecycle-hooks spec §7.3 already says the engine never repeats a project's interpolated secrets. It
says nothing about secrets in the values an interaction carries. Today's Pact redacts nothing: pact
files record whatever the test declared, and pact's own advice is to declare a fake token and have the
provider's verification replace it with a request filter. Several providers instead accept one fixed
fake token in their test environment, and their verification depends on the pact replaying it.

## Decision

1. **A redacted member is named by a path**: a part, a slot, then member names, spelled as a plan path
   is without its `$.` (`request.headers.authorization`, `request.body.credentials.password`; a name
   with a dot or a bracket is quoted, `['a.b']`). A path of a part and a slot alone is the whole slot.
   Names are compared ignoring ASCII case, since HTTP header names are case-insensitive and redacting
   too much is the safe direction. A member step that meets an array applies to each element.
2. **The defaults are the transport's.** A transport declares which of its members carry credentials,
   as `credential-members` in its `TransportContribution` (part → slot → member names), as it declares
   its content slots. The kernel names none. The built-in HTTP transport declares the request's
   `authorization`, `proxy-authorization` and `cookie` headers and the response's `set-cookie`: the
   credentials HTTP itself defines. Anything else, an `x-api-key` or a body's `password`, is a session's
   to add.
3. **A consumer session adds and keeps.** `consumer-session/create`'s `config.redaction` is
   `{ add, keep }`, each a list of paths. `add` names more members. `keep` names a member to record as
   it is, which removes it from the transport's defaults and from `add`: a fixed fake token the
   provider's test environment accepts is kept, and is recorded and replayed as today. A path that does
   not parse fails the create. Each SDK's `janus` options take `redaction` and pass it as written.
4. **A contract the engine writes holds no redacted value.** Under a redacted member, every recorded
   value and every shape's `example` is replaced, keeping its kind and structure: a string becomes
   `"[redacted]"`, a number `0`, a boolean `false`, and objects and arrays keep their member names and
   lengths. An `equality` there becomes a `type`, so the contract says a value of that kind was sent
   and not what it was. Patterns, bounds and generators are kept. A path that reaches into a slot
   recorded as octets replaces the slot's whole value. The interaction lists the paths it redacted, as
   `redacted`. An interaction with no redacted member is written exactly as before, so its contract's
   hash does not move.
5. **`upgrade/pact` redacts with the transport's defaults**, and reports each member it redacted as
   `value-redacted`, a `note`. `add-pact-interaction`, which converts as `upgrade` does (ADR 0066),
   is redacted with its session's `redaction` when the contract is written. So a facade's contract and
   `janus upgrade` of the pact it would have written stay the same. The pact itself is not changed.
6. **A verifier replays what was recorded.** A redacted member is sent as its placeholder unless the
   run replaces it, with a `before-request` hook (an SDK's request filter) or the HTTP transport's
   `headers` (`--header`). When a variant fails, its result lists, as `redacted`, the request's
   redacted members that no `before-request` hook changed, and the report says how to supply them. The
   engine cannot see what a transport adds as it sends, so the list names what the hooks left alone.
7. **Nothing the engine reports repeats a redacted value.** The secrets of an exchange are the strings
   found under a redacted member: in the interaction it ran (its shapes' examples and its variant's
   recorded values), in the values it matched, and in the request it sent. The redacted members are
   the transport's defaults, a session's `add`, and a contract's `redacted`; `keep` decides what is
   recorded, not what is reported. Every occurrence of a secret is replaced by `[redacted]` in an
   executed plan's resolved values, literals and messages, in the static plan beside it, and in the
   mismatches built from it. That covers `verification/executed-plan`, a result's `mismatches`,
   `verification/explain`'s `executed` (`janus explain --executed`), and a consumer session's
   mismatches in `finalise` and in the mock's rejection.
8. **Lifecycle-hooks spec §7.3 extends to these**, and says so: a hook's own secrets were never
   reported, and now nor are the secrets an interaction carries.

## Alternatives considered

- **Redact nothing by default; let a session name what to redact.** Changes nothing for anyone who
  does not ask, and protects nobody who does not know to. The token that leaks is the one nobody
  thought about. Rejected.
- **Defaults that cannot be kept.** A credential header would never be recorded, so the rule is
  simpler. It breaks every provider that accepts a fixed fake token, the one pact workflow that relies
  on the recorded value, and the only fix would be a request filter they did not need before.
- **Leave a redacted member out of the replayed request.** The provider then sees no header, a cleaner
  `401` than a placeholder's. But a provider that does not check credentials in its tests would also
  see a request the consumer never sent, and today's verifier always sends what the pact holds.
- **Refuse to verify a redacted member nothing supplies.** Strict, but the engine cannot see the
  transport's headers, so it would refuse runs that `--header` makes work.
- **Scrub by name in the text forms.** Recognising a header in a plan's text means parsing the
  engine's own output, and a mismatch message quotes the value, not the member it came from. Replacing
  the values themselves, wherever they appear, needs neither.
- **Keep `equality` on a redacted member, with the placeholder as its value.** A response's redacted
  `set-cookie` would then fail against every real cookie. `type` keeps the kind and drops the value,
  which is all that survives redaction.

## Consequences

- A contract recorded with a token in `Authorization` now holds `"[redacted]"` there, and its hash
  stops changing with the token. A provider that relied on the recorded value supplies one with a
  request filter or `--header`, or its consumer keeps the header. The engine change is `major`.
- `janus upgrade` of a pact with credential headers writes a contract the pact does not match byte for
  byte; the findings say where. A provider verifying the upgraded contract needs the same remedy.
- A shape's example under a redacted member need not be admitted by its node, since nothing produces
  from it: contract-file spec §5.6 says so.
- A secret also appearing as ordinary text elsewhere in a report is replaced there too. A short secret
  can make a report harder to read; it cannot make one leak.
- Scripts and `exec` hooks that log a value have still logged it (§7.3's last rule). The engine cannot
  redact what it never sees.
- **Tripwire**: if pilot providers keep many headers by name, a per-interaction `keep` in the DSL is
  the next step, not a change to the defaults.
