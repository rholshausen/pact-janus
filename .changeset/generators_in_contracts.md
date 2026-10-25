---
default: minor
---

# Contracts carry generators, filled where a value is produced

A shape node's `generator` member is now `{ "name": "<component>:<generator>", "config": { … } }`, and the
engine fills fresh values from it where a value is produced (ADR 0048). The example stays what a contract
records, so recording the same interaction twice still writes the same contract.

- **`verification/verify` fills a contract's request.** After `state-setup` and before `before-request`,
  every position the request's shapes give a generator gets a fresh value, on the provider's side, from
  the same provider-state values a pact's generators read. A slot carried as octets is decoded by its
  declared type, or the type it is labelled with, and encoded back by the same one. A generator that
  produces nothing keeps the recorded value, with a `generator-failed` warning, as for a pact.
- **A mock fills its reply on every exchange**, on the consumer's side, with the variant's resolved
  state parameters as provider-state values.
- **`generator/generate`'s context has `side`**, `provider` or `consumer`. The built-in `pact`
  component keeps the example for a generator that does not run on that side: `pact:ProviderState` on
  the consumer's.
- **`upgrade/pact` writes a v1–v4 generator onto the shape at its target**, as `pact:<type>` with the
  pact's members as its `config`, where until now it dropped every one. A generator no component
  provides (`MockServerURL`), one in a text, binary or multipart body, or one whose path names no
  position is still `generator-dropped`.
- **`upgrade/pact` declares a JSON body's type** in `content-types`, with no `requires` entry, so a
  reply with no `Content-Type` is decoded as JSON, as the pact's verification decodes it. A converted
  body's evidence is the pact's octets, as text when they are UTF-8.
- **`eachKey` and `eachValue` convert as the legacy compiler compiles them.** `eachKey`'s keys follow
  the rule's own matchers and its values are left open, reported as `rule-unmapped`. `eachValue` on a
  list becomes `each-like` with `min: 0`.
- **A new finding, `pending-dropped`**, for a v4 interaction marked pending, which a contract has no form
  for.
- **The built-in form component writes a number or a boolean as its text**, as the XML component does.
