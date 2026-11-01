---
engine: minor
---

# `engine/hello` declares `background`

The engine runs a mock's exchange loop, a verification run and a broker publication beside the frame
stream, on work its embedding provides (ADR 0062). `janus-engine` and `janus` provide threads, and
their hello now declares the `background` capability. The offline WASM engine provides none. It
declares no `background`, and answers `consumer-session/start-transport` and `verification/verify`
with `component-unavailable` naming `background`, where it named the transport before. Nothing an
SDK sends to `janus-engine` changes.
