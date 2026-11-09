// The janus-engine this SDK runs when it is not told which (staged plan S1.36; ADR 0029 decision 3,
// ADR 0060 decision 6): sdks/engine-release.json, repeated as a module. Written with it by
// `cargo run -p pact_janus_engine_pin -- <version>`; never edited by hand.

import type { EnginePin } from "./release.js";

export const PIN: EnginePin = {
  "repository": "rholshausen/pact-janus",
  "version": "0.5.0",
  "sha256": {
    "janus-engine-aarch64-apple-darwin": "0072e1ea6df262b13f9e93dcbbab8c58f34e93925dee8125199718818112e886",
    "janus-engine-aarch64-unknown-linux-musl": "9c064caa2b4161541ef4db95eec9b0fd6d574d9a7088fbba23d5911132db4c20",
    "janus-engine-x86_64-apple-darwin": "603478506c07125e6ac119e69a979158f877894ff24f5f94c61bad5edf16a7e1",
    "janus-engine-x86_64-pc-windows-msvc.exe": "4a5678a37cd970b13906ef9f4f2e072dc21114aa626177f44bc41e1021c79aee",
    "janus-engine-x86_64-unknown-linux-musl": "9e6903afb0f1abb167230a759d52a1755db1850ebe12503fdf8f26ec05f14bb9"
  }
};
