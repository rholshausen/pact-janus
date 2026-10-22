# 0038 — Read form bodies as a query string, through a built-in form component

- **Status**: accepted
- **Date**: 2026-10-22
- **Plan tasks**: S1.21 (the build); S1.25 (`janus upgrade`, which this leaves as it was)
- **Evidence**: pact-reference's matcher (`pact_matching::form_urlencoded::match_form_urlencoded`,
  which hands both bodies to `query::match_query_maps`); `pact_models`' `FormUrlEncodedHandler`, which
  `pact_matching` generates into a form body with, and which `pact_verifier` is built without
  ([pact-reference#588](https://github.com/pact-foundation/pact-reference/issues/588)); pact-jvm's
  `FormUrlEncodedContentTypeHandler`; S1.1's first run
  (`compatibility-suite/v1/23-…`); [ADR 0036](0036-read-text-bodies-through-a-text-component-and-unclaimed-bodies-as-bytes.md)
  decision 3 and its tripwire; [ADR 0037](0037-carry-member-order-through-the-engine-and-decide-nothing-by-it.md)

## Context

A v1–v4 pact with an `application/x-www-form-urlencoded` body is refused as `component-unavailable`,
and a body generator on a form body is sent as recorded with a `generator-failed` warning. Today's
verifier passes the first. What it does with the second depends on which verifier: see below.

Today's verifier compares a form body exactly as it compares a query string. It parses both bodies
into name–value pairs, groups the values by name, and calls the query matcher on the two maps, with
the body's matching rules. So a name the pact lacks is a mismatch, in a request and in a response
alike, which is unlike a JSON response body. A name's values are compared in order, by the rule at
`$.<name>` or `$.<name>[<index>]` or by equality.

Generating is where today's verifiers differ. `pact_matching` parses the body into pairs, applies each
generator to one occurrence of its name, and writes the pairs back out with `serde_urlencoded`, but
only with its `form_urlencoded` feature, which `pact_verifier` does not turn on. So
`pact_verifier_cli` sends a form body's generators unapplied, and has for a long time
([pact-reference#588](https://github.com/pact-foundation/pact-reference/issues/588)). pact-jvm applies
them.

The query slot already has a model: an object whose members are arrays of strings (the HTTP
transport's, which a Janus contract's query shape addresses). Since ADR 0037 the object keeps the order
its members arrived in.

ADR 0036's tripwire says that if a body kind cannot route through its classification without a
special case per kind in the legacy compiler, the classification belongs in the content components.
A form body needs one: it is compiled as a query string, not walked as a document.

## Decision

1. **Form bodies are a built-in content component, `form`**, declaring
   `application/x-www-form-urlencoded`. Every embedding registers it after JSON and text, and
   `engine/hello` lists `content/form`. It contributes no plan fragments and detects nothing.
2. **A form body decodes to the query slot's model**: an object whose members are arrays of strings,
   one member per name in the order the name first appears, and its values in order. Percent-encoding
   and `+` are decoded as `serde_urlencoded` decodes them, which is how today's verifier reads the
   body. `encode` writes each member's values in order, a string member as one value. Repeated names
   that were interleaved (`a=1&b=2&a=3`) are written grouped (`a=1&a=3&b=2`). That loses nothing a
   verdict reads, since today's verifier groups them too.
3. **The legacy layer reads a v1–v4 form body into that document, and the legacy compiler compiles
   it as today's verifier compares one: as a query string.** The compiler's query logic is reused
   whole, with the body's rules, the body's path, and every name required whichever part it is in.
   A reply's form body is decoded by the component, as the pact's type (ADR 0036 decision 4).
4. **This answers ADR 0036's tripwire, and does not move the classification.** The compiler gains a
   case for form, but no new semantics: form is the query rules on another slot. What the tripwire
   guards against is the legacy compiler owning each content type's reading. Here reading stays with
   the component on the reply side, and with `serde_urlencoded` on the pact side, as JSON's does with
   `serde_json`. Compiling v1–v4 rules is the legacy compiler's alone (plan-grammar spec §1), and no
   component could do it without importing those semantics. XML (S1.22) asks the tripwire's question
   again, more sharply.
5. **A form body's generators are applied**, with `pact_models`' `FormUrlEncodedHandler`, as
   `pact_matching` applies them when it is built to, and as pact-jvm does. Each value comes from the
   built-in `pact` generator component (ADR 0034), and a generator that produces nothing keeps the
   recorded value and is reported, as in a JSON body. Against `pact_verifier_cli` this is a
   deliberate difference, which S1.1's ledger records with pact-reference#588.

## Alternatives considered

- **Compile a form document with the JSON walk.** It needs no new compiler case, but it gets every
  verdict the walk decides differently wrong. A response would ignore names the pact lacks, and a rule
  at `$.<name>` would be applied to the array of values, not to each value.
- **A form component that contributes a plan fragment for v1–v4 rules.** The tripwire's literal
  answer. It would make a content component interpret `pact_models` matching rules, which only the
  legacy compiler may do, and it would put v1–v4 precedence in a second place.
- **Model a form body as an ordered list of pairs**, which keeps interleaving. The query slot and
  today's verifier both group by name, and a contract author would have a second model for the same
  data.
- **Leave form to a declared, out-of-tree component**, as CSV is. A v1–v4 pact names no components,
  so a pact with a form body would still fail until its user declared one. Today's verifier needs
  nothing declared.

## Consequences

- A v1–v4 pact with a form body verifies as it does today, and its request generators are applied,
  which today's Rust verifier does not do.
- A Janus contract can declare a form body. Its shape is an object of string arrays, written as the
  query slot's is.
- `janus upgrade` still carries a form body across with `body-not-parsed`. S1.25 now converts form
  bodies too.
- Plan text: a form body's plan shows `expect:count`, a match per value and `legacy:only-entries`
  under `body`, as a query's does under `query`.
- **Tripwire**: if a second body kind needs a compiler case that is not an existing slot's rules
  reused, as XML's attribute and text paths may, the reading of that kind's v1–v4 paths belongs with
  its component, and this decision is revisited for XML.
