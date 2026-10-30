---
default: minor
---

# An SDK can verify a provider inside a test, and the engine loads a project's configuration for it

Staged plan S1.14, ADR 0055.

- **`config/load`** (engine-protocol spec §8.8): `{ path }` → `{ config }`, the project
  configuration file resolved by the loader `janus verify --config` runs. `${VAR}`s come from the
  engine process's environment, scripts are inlined and paths are made absolute. `path` must be
  absolute, or the call is `malformed-frame`.
  - It is behind a new `config-loading` capability, which `janus-engine` and `janus` declare.
  - The errors are the loader's: `hook-unresolved`, which names the `variable` or `path`, and
    `hook-config-invalid`.
  - An engine without a loader, such as the offline WASM artifact, answers `component-unavailable`
    naming `config-loader`.
  - The generated bindings gain `Load`, `LoadResult` and `ResolvedConfig`, and `config/load` joins
    the known operations. They also gain the lifecycle-hooks schemas as a new set, `hooks`
    (`HookContext`, `HookConfig`, `HookEntry`, …), which a provider SDK serves and writes.
- **The behavioural specification has a provider side:** `verifier`, `state-handler`,
  `request-filter` and `verify`, in a new `verification` category. Its schema adds a `hook-entry`
  kind to `produces`.
- **The conformance suite has a `provider` category**, and its scripted engine calls an SDK's hooks
  during a run. An SDK that implements the provider primitives must pass its 14 cases and the new
  live case.
