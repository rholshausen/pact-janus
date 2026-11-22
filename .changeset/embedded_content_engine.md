---
engine: minor
---

# A content component can ask the engine to decode content of another type inside its own, and a multipart part reaches a declared component

Staged plan S5.6, ADR 0103, superseding ADR 0040 decision 3.

- **A multipart part's body is decoded and encoded through the session's content registry**, declared
  components first, where it went only to the built-in JSON, XML, text and form components. A project
  that declares a CSV component has a `text/csv` part decoded into rows a contract can shape. A mock
  serves that part as CSV, written from the part of the shape that describes it. With no component
  declared, every multipart document is what it was, and no verdict changes.
- **`content/decode` and `content/encode` results may carry `embedded`**, the slots or documents of
  another type a component needs first. The engine decodes or encodes each through the registry and
  repeats the request with `embedded` answered by id. A question carries `document: null` or
  `value: { "content": null }`, which the engine does not read. One round per request. Content more
  than eight levels deep is answered `embedding-too-deep`, and asking twice is `component-malformed`.
  An embedded document is encoded with the part of the request's `shape` at the pointer `at` gives.
- **`component/hello` now sends `capabilities: { "embedded-content": {} }`**: a component should ask
  only of an engine that sends it. Components that never ask are unchanged, and so is the WIT world.
- The native binding's `Decode`, `DecodeResult`, `Encode` and `EncodeResult` gain an `embedded` member,
  with `of` constructors. A Rust embedding that builds them by hand adds it.
