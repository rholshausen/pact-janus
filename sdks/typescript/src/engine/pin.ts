// The janus-engine this SDK runs when it is not told which (staged plan S1.36; ADR 0029 decision 3,
// ADR 0060 decision 6): sdks/engine-release.json, repeated as a module. Written with it by
// `cargo run -p pact_janus_engine_pin -- <version>`; never edited by hand.

import type { EnginePin } from "./release.js";

export const PIN: EnginePin = {
  "repository": "rholshausen/pact-janus",
  "version": "0.4.5",
  "sha256": {
    "janus-engine-aarch64-apple-darwin": "3d8b0c3b898a4fc2703dd6793d59699f551320241d63402adf30a6084706b6cf",
    "janus-engine-aarch64-unknown-linux-musl": "6ce2c85bbff83723affcfcdc94bd242fd89a7a1b0d3a28927829f83b684de3e9",
    "janus-engine-x86_64-apple-darwin": "00d4fc97ad56274c34e012517b31ec5db081cb3ad3f0e6f2b9e1cc79700c22b5",
    "janus-engine-x86_64-pc-windows-msvc.exe": "96d904082c3f9154b2606f2204f2394f16fd20ea28af6c0cd8411519e3d32777",
    "janus-engine-x86_64-unknown-linux-musl": "0ac2ac2827e3ccae687e1b0e842cb8446820b51dd19129c4f432c214b94f5a9c"
  }
};
