// The janus-engine this SDK runs when it is not told which (staged plan S1.36; ADR 0029 decision 3,
// ADR 0060 decision 6): sdks/engine-release.json, repeated as a module. Written with it by
// `cargo run -p pact_janus_engine_pin -- <version>`; never edited by hand.

import type { EnginePin } from "./release.js";

export const PIN: EnginePin = {
  "repository": "rholshausen/pact-janus",
  "version": "0.6.0",
  "sha256": {
    "janus-engine-aarch64-apple-darwin": "5e98701c4a77439fd83797880f88e679115ceca699a8346ea6a1eeea443a61f2",
    "janus-engine-aarch64-unknown-linux-musl": "3b5039fbadb3231028c1b821901ead8039199dcce66e3d60e3f831ee016de600",
    "janus-engine-x86_64-apple-darwin": "53fa55c9184f151de05c3f2bb4888c834cc44c61c4396abc3753c6fdb5cfcbcb",
    "janus-engine-x86_64-pc-windows-msvc.exe": "06a4ac4bf97777f8fabf26c8f677c49620d401b8d8aa8065fe953f3e99840b2f",
    "janus-engine-x86_64-unknown-linux-musl": "c0286ea30931e0d8ddf30ba2fd5450f881c380a1ab769d4c7e0b0457faf33ac5"
  }
};
