// The janus-engine this SDK runs when it is not told which (staged plan S1.36; ADR 0029 decision 3,
// ADR 0060 decision 6): sdks/engine-release.json, repeated as a module. Written with it by
// `cargo run -p pact_janus_engine_pin -- <version>`; never edited by hand.

import type { EnginePin } from "./release.js";

export const PIN: EnginePin = {
  "repository": "rholshausen/pact-janus",
  "version": "0.4.3",
  "sha256": {
    "janus-engine-aarch64-apple-darwin": "48d0e25eb9ac71f00b9604911512e4b8e96c9dcb05211cd2b3644dad5126adba",
    "janus-engine-aarch64-unknown-linux-musl": "03c19d49effd2a8f7f480d5735020b865678d5d7daf35e6f11c9d60a8c160c21",
    "janus-engine-x86_64-apple-darwin": "02999ea80cf977240cdb6604e2805dbc96505f0f090b972afe965d9984ea1ff8",
    "janus-engine-x86_64-pc-windows-msvc.exe": "b71ba22b15c75dbae59f648bf1c7a02a6fae449dadf71968bf89caf61ac36b09",
    "janus-engine-x86_64-unknown-linux-musl": "b3f4d750f57149d977a8ac62983f8ef6f892e326371e541f73184c737d045f0a"
  }
};
