// The janus-engine this SDK runs when it is not told which (staged plan S1.36; ADR 0029 decision 3,
// ADR 0060 decision 6): sdks/engine-release.json, repeated as a module. Written with it by
// `cargo run -p pact_janus_engine_pin -- <version>`; never edited by hand.

import type { EnginePin } from "./release.js";

export const PIN: EnginePin = {
  "repository": "rholshausen/pact-janus",
  "version": "0.8.0",
  "sha256": {
    "janus-engine-aarch64-apple-darwin": "f38e8f080a514b6a1042df6c92f600c7f215c946c6556ec4bce856869d839664",
    "janus-engine-aarch64-unknown-linux-musl": "c943a68b928ac73d44b5963075840683469b9d1a575b7784ebc9dfbea3d58541",
    "janus-engine-x86_64-apple-darwin": "9e91b2aece7a134a9004eedac1b72f865ad99aad5f27e2224321f0d9a269a6c6",
    "janus-engine-x86_64-pc-windows-msvc.exe": "0d0b2de54c1c00a84741f9d330374b39a094ff5d04fc355d89beb6a1ebcba9c9",
    "janus-engine-x86_64-unknown-linux-musl": "1c99c00875fd51ed29e0287a78f50bca7a096bf5c0da946d85cb33a44bfb16fe"
  }
};
