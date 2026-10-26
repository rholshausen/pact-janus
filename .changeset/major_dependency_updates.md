---
default: patch
---

# The engine moves to wasmtime 50 and rquickjs 0.14

Dependency updates with nothing for a host to change; what each one could change is below.

- **Scripted hooks run on QuickJS-ng 0.16.2** (rquickjs 0.14), up from 0.8.0. The script API, its
  interruption at a deadline and its memory and stack bounds are unchanged, and every hook test
  passes, but a script that leans on an engine quirk may behave differently.
- **Out-of-tree WASM components load under wasmtime 50**, up from 47. A component's `fs` grant
  allows what it did: `read-write` is every operation, anything else is read-only. Components built
  with an older wit-bindgen still load.
- The WASM engine is built with wit-bindgen 0.63, and the engine's own base64, sha2 and jsonschema
  move to their current versions. A component digest is still `sha256:` and 64 lowercase hex digits.
