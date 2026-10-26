---
default: minor
---

# An error carries its causes as a chain, and `janus` shows what failed and why

Staged plan S1.33, ADR 0054.

- **`EngineError` may carry `chain`**: its layers, outermost first, each `{ code?, message }` in that
  layer's own words. The first layer is the error itself and the last is its root cause.
  - `message` keeps its meaning, the complete sentence, so a host that reads only it loses nothing.
  - A component's error document may carry the same member.
  - `component-failed` writes its own layer, then the component's chain.
  - The generated bindings gain `ErrorLayer`.
- **Errors that carry a chain:**
  - a request the HTTP transport could not send;
  - a broker that could not be reached, refused a request, or answered with something that is not JSON;
  - an `http` hook that got no answer or a non-2xx one;
  - an `exec` hook that could not start or exited non-zero.

  A failing `exec` hook's message now ends with the last line it wrote to stderr, which used to
  appear only in `details`.
- **`janus` prints an error as what failed and its root cause:** the innermost layer with a code,
  then a `caused by:` line. Use `--error-chain`, a global option, to print every layer.
- **Wording.** A broker's retry-after note now comes before the broker's own explanation. A
  configuration file that cannot be read no longer says `(os error 2)`.
