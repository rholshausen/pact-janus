---
default: patch
---

# A v4 `statusCode` matcher verifies, through the HTTP component's `http:status-class`

A v1–v4 pact whose response status has a `statusCode` rule failed `janus verify`, which refused the
rule. It now compiles to `http:status-class`, which the built-in HTTP component runs (ADR 0057): a
`class` (`information`, `success`, `redirect`, `client-error`, `server-error`, `non-error`, `error`)
or a list of `codes`, each as today's verifier checks it. A contract `janus upgrade` wrote with the
operator verifies too.

`janus upgrade` now writes a list of codes as `codes: [200, 204]`, where it wrote a comma-separated
`class` that nothing read.

For an SDK maintainer: `janus-engine` registers the matcher, so nothing changes for an SDK. An
embedding built on the kernel registers a matcher compiled into it with `Engine::register_matcher`;
the WASM engine has no HTTP component, and fails a plan that reaches `http:status-class` by name.
