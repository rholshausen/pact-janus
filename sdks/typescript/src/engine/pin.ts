// The janus-engine this SDK runs when it is not told which (staged plan S1.36; ADR 0029 decision 3,
// ADR 0060 decision 6): sdks/engine-release.json, repeated as a module. Written with it by
// `cargo run -p pact_janus_engine_pin -- <version>`; never edited by hand.

import type { EnginePin } from "./release.js";

export const PIN: EnginePin = {
  "repository": "rholshausen/pact-janus",
  "version": "0.4.0",
  "sha256": {
    "janus-engine-aarch64-apple-darwin": "a597f3e519ff2c778d6da4f38701a98cafee4b25542808a2b78fe943aec7d8cc",
    "janus-engine-aarch64-unknown-linux-musl": "89cca6c74fb695574611ea88327b618f894cc2d8f564ba126efe3dd255298121",
    "janus-engine-x86_64-apple-darwin": "d782fc3c4ee734c330d94c71dc3627803fe8f1c68bee3f5231d0f4258ec48708",
    "janus-engine-x86_64-pc-windows-msvc.exe": "264953f84c92d8c5628ed4f2d914aa02907954fe542c48004969d4662871c228",
    "janus-engine-x86_64-unknown-linux-musl": "9b1f06d2c437c6497f3c34369f44100f3cdeb1071a8bf696d706cf8bbefe2a74"
  }
};
