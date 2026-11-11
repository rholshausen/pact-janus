# 0085 — Name a generator in the DSL on the value it produces, and refuse one nothing provides when the interaction is added

- **Status**: accepted
- **Date**: 2026-11-12
- **Plan tasks**: S3.6
- **Evidence**: `conformance/cases/live/generated-served-fresh.json`, passing in both SDKs, the consumer
  conformance case for a served generated value that S1.19's exit named; `conformance/cases/translation/generated*.json`
  and `live/generated-engine-validates.json`; `engine/kernel/src/shape/fresh.rs`'s tests, and
  `engine/kernel/tests/consumer_flow.rs`'s `a_generator_no_component_provides_is_refused_when_the_interaction_is_added`

## Context

ADR 0048 put a `generator` on a shape node, `{ name, config }`, and made a mock fill it fresh in every
reply and a verifier in the request it sends, while the contract records the example. Only `janus
upgrade` and an interaction specification written by hand could name one: no DSL could, so a consumer
suite wrote no generated value. The SDK specification already refuses the other way of getting one, a
`like(v)` whose example a random function chooses, because a recorded value must never be random
(ADR 0008). Three things had to be settled before the DSL could spell it:

- **The spelling.** A wrapper around a shape, or an option on every value helper.
- **Where it may go.** A generator replaces the value where it is produced. On a node whose value the
  running variant chooses, an `any-of`'s option, an `optional`'s presence, an `each-like`'s length, it
  would replace the variant's evidence with its own, and the variant would never be served.
- **A name nothing provides.** The engine asked a generator only when it produced a value. A misspelt
  `pact:Uiud` was a warning in the engine's log on every exchange, and the recorded value was served in
  silence: the consumer's test passed against the example it thought it had made fresh.

And one gap in ADR 0048 decision 6's walk: it did not go into a `one-of`. No upgraded pact has one, but
the native DSL does, and a generator in an alternative was never filled.

## Decision

1. **`generated(of, generator, config?)` is the DSL's primitive.** It compiles `of` by the literal
   rules and writes the node it compiles to with `generator: { name, config }`, `config` only when
   given and as given. It is not a node of its own: it goes wherever the shape it wraps may, a body's
   member, a path, a status, a header or query value (inside the one-element list a value gets), a
   message's metadata. TypeScript spells it `generated(integer(1), "pact:RandomInt", { min: 1, max: 99 })`,
   Java `Shapes.generated(Shapes.integer(1), "pact:RandomInt", Map.of(...))`. The generator's name is
   its component's, `pact:` for the vocabulary the `pact` component provides (ADR 0034); the DSL has no
   helper per generator.
2. **`of` is one value: a plain scalar or a value helper's result.** `equality`, a kind predicate, a
   temporal or `regex`. Anything else is refused at the call: a map, a list or a structural helper, which
   one value would replace; `any-of`, `one-of`, `optional`, `nullable` and `each-like`, whose value the
   running variant chooses; `forbidden`, which has none; and a `generated` result, whose generator a
   second would replace. A value that may be absent is `optional(generated(...))`. A plain value stays
   `equality`: the generator changes what is produced, never what is admitted, which is what a request
   path a verifier fills from a provider state needs.
3. **The SDK runs no generator and checks nothing about one.** No recorded value is random (ADR 0008):
   the contract records the example, and the engine produces the fresh value.
4. **The engine asks every generator once when an interaction is added**, on the consumer's side, with
   its node's example and no provider-state values. A generator member that is not `{ name, config }`,
   a name no component provides (`unknown-generator`) or a configuration its component refuses
   (`invalid-config`) is `interaction-invalid`, its problem at the node's `generator`. Any other error,
   one that needs a provider state, say, stays the exchange's to report, as ADR 0034 decision 5 has it.
   This holds for every interaction `add-interaction` takes, a facade's converted one included; `janus
   upgrade` writes only generators the `pact` component provides (ADR 0048 decision 7), so nothing it
   wrote is refused.
5. **The walk goes into a `one-of`**, amending ADR 0048 decision 6: the alternative filled is the one
   whose discriminator literal the value binds (shape spec §5.4), through a `nullable` wrapping it. A
   value that binds none is left as it is.
6. **The behavioural specification gains a `shape-member` kind of `produces`**, for a primitive that
   writes a member on the node it is given rather than a node of its own. `generated` produces
   `generator`.

## Alternatives considered

- **A `generator` option on each value helper**, `integer(1, { generator: ... })`. It would put the
  same option on nine helpers in two languages, each a place for the spellings to drift, and on
  `datetime` and `regex` it would sit beside a format or a pattern that already takes the second
  argument.
- **Named helpers, `uuid()`, `randomInt(min, max)`, `fromProviderState(expression, example)`**, as
  pact-js and pact-jvm have. They are pact's vocabulary in the native DSL, and each new generator, a
  component's own included, would need a helper in every SDK. The facades keep them.
- **Accept a generator on any node**, as the shape language does. On a node a variant decides, the
  mock would serve the generator's value for every variant of the dimension, and the author would never
  learn why the variants looked alike.
- **Check the generator in the SDK**, against a list of `pact` generators. The list is the engine's: a
  declared component may provide others, and a second copy could only disagree with it.
- **Keep warning at serve time.** The warning is in the engine's log, which a passing test does not
  show; the consumer would ship a contract it thought was exercised with fresh values.

## Consequences

- A consumer suite can write a fresh value in both SDKs, and the contract it writes is the same bytes
  as one without: the example is the evidence. `live/generated-served-fresh` closes S1.19's exit.
- An interaction naming a generator nothing provides now fails at `execute`, where it passed before
  with the example served. That is the change file's `minor`, since the interaction never did what it
  said.
- Adding an interaction asks each of its generators once more than before. A `pact` generator is
  microseconds; a component's could be slower, and is asked once per interaction, not per exchange.
- A generated value is still not checked against its shape (shape spec §3.4's SHOULD, ADR 0048).
- **Tripwire**: an author who needs a generator on a value a variant chooses, a fresh value for every
  element of a list, say, writes it on the element, `eachLike(generated(...))`. If authors ask for one
  on the list itself, decision 2 is revisited with a shape-language answer for what a variant point
  means when a generator replaces it.
