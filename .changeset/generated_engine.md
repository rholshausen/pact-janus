---
engine: major
---

# A generator nothing provides is refused when the interaction is added, and the conformance suite holds an SDK to `generated`

Staged plan S3.6, ADR 0085.

- **`consumer-session/add-interaction` asks each generator its shapes name once**, on the consumer's
  side, with its node's example. A `generator` member that is not `{ name, config }`, a name no
  component provides (`no generator 'pact:Uiud'`) or a configuration its component refuses (`'Regex'
  is missing a member it needs (regex)`) is `interaction-invalid`, its problem at the node's
  `generator`, such as `/parts/response/body/members/token/generator`. Before, the mock served the
  recorded value on every exchange and said so only in its log. `janus upgrade` writes only
  generators the `pact` component provides, so no upgraded pact, and no interaction a facade
  converts, is refused by this.
- **A mock and a verifier fill a generator inside a `one-of`**, in the alternative whose discriminator
  literal the value binds. Before, nothing below a `one-of` was filled.
- **The behavioural specification has a `generated` primitive**, and six cases cover it:
  `translation/generated`, `translation/generated-optional-refused`, `translation/generated-any-of-refused`,
  `translation/generated-twice-refused`, `live/generated-engine-validates` and `live/generated-served-fresh`.
  `janus-sdk-gates conformance check` expects them in every report, so an SDK must implement `generated`
  to move to this release. A live case's exchange may say what the mock served, `expect-body`, and a
  primitive's `produces` may be a `shape-member`, a member written on the node it is given.
