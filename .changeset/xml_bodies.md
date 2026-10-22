---
default: minor
---

# XML bodies verify, through a built-in XML component

A v1–v4 pact whose body is XML now verifies as today's verifier compares it (ADR 0039). It used to
fail before anything was sent, as `component-unavailable`.

- **A built-in `xml` content component** claims `application/xml`, `text/xml` and every `+xml` type.
  Every embedding registers it after JSON and before text, and `engine/hello` lists `content/xml`. It
  decodes an XML body into a document keyed by element name. The body is an object holding the root
  element under its local name. An element is an object of `#namespace` (when it has one), `@<name>`
  per attribute, `#text`, and an array of children per local name. It encodes such a document back, so
  a Janus contract can declare and serve an XML body.
- **A v1–v4 XML body is compared element by element**, as today's verifier compares one. Rules are found
  by pact's own XML paths (`$.a.b['@c']`, `['#text']`). Repeated children under a type rule are compared
  against the pact's first, a request has no attribute or child the pact lacks, and text is compared
  even when the pact has none. Every pact-specification case now runs, none skipped.
- **One deliberate difference:** an element the pact puts in no namespace matches its local name in any
  namespace, as the pact specification says. Today's verifier and pact-jvm compare the qualified name.
- **A type rule on text reads the text**, as today's verifier does. `integer`, `number`, `decimal` and
  `boolean` on a path, header, query or form value, or on XML attribute and text, accept text that parses
  as one. They used to fail it, because they compared the value's kind.
- **An XML body's request generators are applied**, to attributes and text, as today's verifier applies
  them.
- Plan grammar v1 gains `legacy:xml-children`, `legacy:xml-only-children` and `legacy:parses-as`. A
  path segment can be quoted, `['@{urn:names}name']`, for a member name a plain segment cannot hold.
- `janus upgrade` still carries an XML body across with a `body-not-parsed` finding.
