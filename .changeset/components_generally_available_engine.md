---
engine: minor
---

# The component interface is generally available for content and matcher components, and a component that declares what nothing calls is refused

Staged plan S5.1d, ADR 0104.

- **Generally available**: content and matcher components loaded through the WASM binding may depend on
  the WIT world, the `component/*`, `content/*` and `matcher/{apply,compare,prefix}` documents, the
  declaration, both OCI layouts, signing, and the record of what was loaded, all evolving only additively
  within component protocol v1. `Documentation/component-author-guide.md` is where an author starts.
- **A declared component whose handshake declares what no binding calls fails to load** as
  `component-invalid` (inside `component-unavailable`, `reason: load-failed`), naming what it declared:
  the `hook` interface, `transport` through the WASM binding, `generators`, or an operator's
  `variant-facet`. These loaded before and were silently never used. No component outside the
  repository declared one.
