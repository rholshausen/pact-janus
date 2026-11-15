---
engine: patch
---

# `janus-engine.wasm` decides shapes selected by operation, as `janus check` does

Staged plan S4.3a, ADR 0092. The engine's WASM component registered the content components and
nothing of the HTTP component, because it has no transport. A provider shape recorded or derived by
operation (ADR 0086, ADR 0088) selects with the HTTP component's `http:path`, so no entry matched,
and `subsumption/check` answered `not-checked` where `janus check` found findings. The component now
registers the HTTP component's matchers (`http:path`, `http:media-type`, `http:list`,
`http:status-class`) without the transport, so it answers every `subsumption/*` frame as the native
engine does.

- `pact_janus_component_http`'s transport is behind a default `transport` feature. Without it, the
  crate is the matcher half alone and builds for `wasm32-wasip2`.
- The release checks the component it ships against the native engine, frame for frame, under the
  `wasmtime` crate and under Ruby's `wasmtime` gem, as the Rust and Ruby Pact Brokers would host it.
  `engine/wasm/hosts/ruby/pipe.rb` is that Ruby host.
