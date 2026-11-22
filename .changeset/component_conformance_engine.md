---
engine: minor
---

# `janus component check` runs a component's conformance cases, and the built-in components have theirs

Staged plan S5.10, ADR 0105.

- **`janus component check <component.wasm> <cases>...`** loads a component as a run does, with no
  grants, and runs conformance cases against it through the WASM binding (`conformance-case.schema.json`).
  Exit 0 when every case passes, 1 when one fails or the component does not load, 2 when a case cannot be
  read. `--output json` prints one document.
- **The built-in components' conformance cases** are `corpora/components/<component>/`, for `json`,
  `text`, `form`, `xml`, `multipart` and the HTTP component's matcher, `http`. Each runs through the
  native binding and a serialising in-process one that reaches the component through frame bytes, as the
  WASM binding reaches an out-of-tree component. `janus-spec.tar.gz` carries them, and now carries the
  component-interfaces schemas too.
- **Fixed, as the corpus found them:** `decode-failed` and `unsupported-content-type` from the built-in
  content components now have category `document`, as component-interfaces spec §11.1 says (they had
  `component`). The HTTP component answers `matcher/apply` for an action it does not run with
  `unknown-action`, not `operation-unsupported`. No verdict changes: nothing dispatched on either.
- `conformance-case.schema.json`: an expected `error` needs only its `code`, and `serialising` is a known
  binding. Both loosen the schema.
