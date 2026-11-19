---
engine: minor
---

# A component published with `wkg oci push` loads

Staged plan S5.1b, ADR 0098. The loader accepts the CNCF Wasm OCI artifact layout, which the Bytecode
Alliance's `wkg oci push` publishes, as well as Janus's own. Such an artifact is a Janus component's
when its config's `component.exports` names `pact:janus-component/pipe@1.0.0`. The config is read
before the layer is fetched, so an artifact of another component, or a WebAssembly module, is refused
as `not-a-component`, naming what it exports, without fetching it. Its config carries no name or
version, so the component's handshake alone says which it is.

- `janus component pull` names a component from its handshake, for either layout, so the declaration
  it prints for a `wkg` artifact names the component.
- `janus component push` still publishes Janus's layout, whose config names the component, and whose
  digest is the same however often the same bytes are pushed. `wkg` writes a timestamp into its
  config, so each `wkg` push is a new digest to pin.
- `pact_janus_component_host::oci::Pulled` has a `layout` (`Layout::Janus` or `Layout::Wasm`).
