// The frozen pipe (ADR 0002, ADR 0003): one frame in, one frame back. Everything above this file
// speaks frames; everything below it is one embedding's way of carrying them. The subprocess
// embedding is the only one today — the WASM embedding ADR 0003 names as Node's primary cannot
// host a consumer test's mock server (it needs sockets and a thread of its own; see
// Documentation/phase-9-findings.md §3) — and a WASM embedding drops in by implementing this.

import type { protocol } from "../generated/index.js";

export interface FramePipe {
  /** Sends one request frame and resolves with the response frame carrying the same `id`. */
  exchange(frame: protocol.RequestFrame): Promise<protocol.ResponseFrame>;
  /** Ends the embedding: engine-protocol spec §6 — for a subprocess, closing its stdin. */
  close(): Promise<void>;
}
