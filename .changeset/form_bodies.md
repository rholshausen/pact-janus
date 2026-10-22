---
default: minor
---

# Form bodies verify, through a built-in form component

A v1–v4 pact whose body is `application/x-www-form-urlencoded` now verifies as today's verifier
compares it (ADR 0038). It used to fail before anything was sent, as `component-unavailable`.

- **A built-in `form` content component** decodes a form body into the query slot's model: an object
  whose members are arrays of strings, each name once, in the order it first appears. It encodes one
  back, writing a member that is one string as one value. Every embedding registers it after text,
  and `engine/hello` lists `content/form`. A Janus contract can declare a form body and shape it as it
  shapes a query.
- **A v1–v4 form body is compared as a query string**, as today's verifier compares one: each name's
  values in order, by the rule on the name or by equality, and a name the pact lacks is a mismatch in
  a response as well as a request. `explain` shows a form body's plan as it shows a query's.
- **A form body's request generators are applied**, each to one occurrence of its name, as
  `pact_models`' handler and pact-jvm apply them. Today's Rust verifier sends them unapplied, because
  it is built without that handler, so here Janus differs from it on purpose.
- `janus upgrade` still carries a form body across with a `body-not-parsed` finding.
