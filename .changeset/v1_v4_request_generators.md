---
default: minor
---

# Verifying a v1–v4 pact applies its request generators, and a state-change reply's values reach them

A pact's request generators are now applied before the request is sent, as today's verifiers apply
them: the path, each header and query parameter, and every JSON body value a generator names. Before,
the recorded example was sent as it stood, so a pact whose path came from a `ProviderState`
generator asked the provider for the consumer test's id rather than the one the state handler had
just created (ADR 0034).

- **Values come from the built-in `pact` generator component**, through `generator/generate`, with
  `pact_models` producing them as today's Rust verifier does: `RandomInt`, `RandomDecimal`,
  `RandomHexadecimal`, `RandomString`, `Regex`, `Uuid`, `Date`, `Time`, `DateTime`, `RandomBoolean`
  and `ProviderState`. `MockServerURL` is the consumer side's and is left alone. Response generators
  are not applied, as today.
- **Provider-state values.** A `state-setup` hook result may carry `values`. A `pact-state-change`
  `http` hook reads a JSON object reply with no `outcome` member as those values, so a state endpoint
  that returns `{"id": 1234}` today feeds `${id}` unchanged. A `ProviderState` generator reads every
  state's `params`, then those values, a later one replacing an earlier one.
- **Changed for `pact-state-change` hooks:** a 2xx reply that is not JSON, or JSON that is not an
  object, is now `ok`, as today's verifiers read it. It was a `hook-result-invalid` failure.
- **A generator that produces nothing** (a `ProviderState` expression no state provided, say) leaves
  the recorded value, as today, and the run emits a `verification/warning` with code
  `generator-failed` naming the target, the generator and why. The verdict is unchanged.
- **New in the specifications:** the `values` member of a hook result and of the hook report
  (`report-data` only), the `example` and `provider-state` members of a generator's context, the
  component error codes `unknown-generator` and `generate-failed`, and the `verification/warning`
  event's row in the protocol's event table. All additive within v1.

A contract upgraded from such a pact still drops its generators (`generator-dropped`); that is
staged plan S1.19.
