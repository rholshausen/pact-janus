// The janus-engine this SDK runs when it is not told which (staged plan S1.36; ADR 0029 decision 3,
// ADR 0060 decision 6): sdks/engine-release.json, repeated as a module. Written with it by
// `cargo run -p pact_janus_engine_pin -- <version>`; never edited by hand.

import type { EnginePin } from "./release.js";

export const PIN: EnginePin = {
  "repository": "rholshausen/pact-janus",
  "version": "0.7.0",
  "sha256": {
    "janus-engine-aarch64-apple-darwin": "50361e0d76a9a8614448979e6929badf660ea624653a291f22689020ef8c79da",
    "janus-engine-aarch64-unknown-linux-musl": "5f367f6ac40b6ab3805be47c15446441b903fe554f118938645c2865c9145cca",
    "janus-engine-x86_64-apple-darwin": "7ab29d5402cf332a938005d66e02e33ead4bbb7ff8ac906f2a8f1b6bb8b45dc4",
    "janus-engine-x86_64-pc-windows-msvc.exe": "ef1b25bfb427d36e359a1dc1f8a29382c92d5d7c01e4985ad38ad73e51d1062a",
    "janus-engine-x86_64-unknown-linux-musl": "c40b1bfb2c879a65ec1513a4d54a2398c05b9795c9c23ac85e572b51d6de84ae"
  }
};
