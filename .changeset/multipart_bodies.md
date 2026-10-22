---
default: minor
---

# Multipart bodies verify, through a built-in multipart component

A v1–v4 pact whose body is multipart now verifies as today's verifier compares it (ADR 0040). It used to
fail before anything was sent, as `component-unavailable`. No body kind is refused any more.

- **A built-in `multipart` content component** claims every `multipart/*` type. Every embedding registers
  it after form, and `engine/hello` lists `content/multipart`. It splits a body at the boundary its
  content type names, into a document keyed by part name: one member per name, each an array of the
  parts with that name, and unnamed parts under `""`. A part is an object of `headers`, `filename` and
  `body`. Each part's body is decoded by its own type through the built-in JSON, XML, text and form
  components; any other type is bytes. It encodes such a document back, choosing a boundary when the type
  names none, so a Janus contract can declare and serve a multipart body.
- **A v1–v4 multipart body is compared part by part**, as today's verifier compares one. Each of the
  pact's parts is found by name, or by position when it has none, and a missing part or a field where the
  pact has a file fails with today's messages. Then come the part's headers, and its body: a text field as
  text with the rules at `$.<part>`, and a file by its own type, with the rules under `$.<part>`. Parts the
  pact does not have are allowed.
- **Deliberate differences**, each where today's own behaviour is accidental: a field with a type of its own
  is compared as that type, not as text; a part with no `Content-Type` is text; unnamed parts are matched
  among the unnamed ones; and a part that is itself multipart is compared as octets.
- Plan grammar v1 gains `legacy:mime-part`.
- A multipart body's request generators are still not applied, as today. `janus upgrade` still carries a
  multipart body across with a `body-not-parsed` finding.
