// The janus-engine this SDK runs when it is not told which (staged plan S1.36; ADR 0029 decision 3,
// ADR 0060 decision 6): sdks/engine-release.json, repeated as a module. Written with it by
// `cargo run -p pact_janus_engine_pin -- <version>`; never edited by hand.

import type { EnginePin } from "./release.js";

export const PIN: EnginePin = {
  "repository": "rholshausen/pact-janus",
  "version": "0.4.2",
  "sha256": {
    "janus-engine-aarch64-apple-darwin": "224cd268eea210610c1e59c710cd3b5e0e2402b3d83a058fbd704b32a4f09a53",
    "janus-engine-aarch64-unknown-linux-musl": "ed839744fc9aa41314c8ca74d4aaccb1e4634f41a853506cf4d2dfe4aee6b1b2",
    "janus-engine-x86_64-apple-darwin": "b0f501c711b281336b827eb054f9a6567216dab919354a589eddf50ea1d366f4",
    "janus-engine-x86_64-pc-windows-msvc.exe": "95c13c38adb2ca57f7977f6151cd4da392397b588d0a926d294b96baa449f970",
    "janus-engine-x86_64-unknown-linux-musl": "c5402e4c9b888b3452c831138e39dc7dfd3574247bfb7b6d807bd9c5779b3c8f"
  }
};
