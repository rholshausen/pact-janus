// The janus-engine this SDK runs when it is not told which (staged plan S1.36; ADR 0029 decision 3,
// ADR 0060 decision 6): sdks/engine-release.json, repeated as a module. Written with it by
// `cargo run -p pact_janus_engine_pin -- <version>`; never edited by hand.

import type { EnginePin } from "./release.js";

export const PIN: EnginePin = {
  "repository": "rholshausen/pact-janus",
  "version": "0.8.1",
  "sha256": {
    "janus-engine-aarch64-apple-darwin": "ed6ca1743867b636d8714b4d992f676867103b4d79ee96bd3ff365c35c33d035",
    "janus-engine-aarch64-unknown-linux-musl": "91a95d59539e3fdc18201a6e3751550662392979b704abf6eba735e13eae06a0",
    "janus-engine-x86_64-apple-darwin": "63a048d3be30287326135c6082733db1850774f80ae3d607a3bf0d0fc8b5881d",
    "janus-engine-x86_64-pc-windows-msvc.exe": "8743ce39afb25b8c467f7e8de8d4735b8cd2d800acbd4e46b01d1d8b7fa859ec",
    "janus-engine-x86_64-unknown-linux-musl": "7cb28dcb2a0bc9caf55a94bbe690024387ae58d102b7aa7b70d4ecc9d9724b9d"
  }
};
