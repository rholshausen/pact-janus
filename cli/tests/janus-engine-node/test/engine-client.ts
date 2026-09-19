// A minimal client for the engine protocol's Content-Length stdio framing (engine-protocol spec
// §3.2, §3.4) over the `janus-engine` subprocess (plan task 4.5) — deliberately not an SDK; the
// TypeScript SDK (sdks/typescript, plan task 6.2) has its own embedding. This is the "thin test client speaks the
// protocol directly" Phase 4's own goal names, ported to TypeScript.

import { spawn, type ChildProcessByStdio } from "node:child_process";
import type { Readable, Writable } from "node:stream";

export interface ResponseFrame {
  type: "response";
  id: string;
  ok?: unknown;
  error?: { code: string; category: string; message: string; details?: unknown };
}

/** Speaks one `janus-engine` process's stdio, matching responses back to requests by `id`. */
export class EngineClient {
  private readonly child: ChildProcessByStdio<Writable, Readable, null>;
  private buffer: Buffer = Buffer.alloc(0);
  private nextId = 0;
  private readonly pending = new Map<string, (frame: ResponseFrame) => void>();

  constructor(binaryPath: string) {
    this.child = spawn(binaryPath, [], { stdio: ["pipe", "pipe", "inherit"] });
    this.child.stdout.on("data", (chunk: Buffer) => this.onData(chunk));
  }

  private onData(chunk: Buffer): void {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    for (;;) {
      const headerEnd = this.buffer.indexOf("\r\n\r\n");
      if (headerEnd === -1) return;
      const header = this.buffer.subarray(0, headerEnd).toString("ascii");
      const match = /Content-Length:\s*(\d+)/i.exec(header);
      if (!match) {
        throw new Error(`janus-engine framing: no Content-Length header in '${header}'`);
      }
      const length = Number(match[1]);
      const bodyStart = headerEnd + 4;
      if (this.buffer.length < bodyStart + length) return; // more data still to arrive
      const body = this.buffer.subarray(bodyStart, bodyStart + length).toString("utf8");
      this.buffer = this.buffer.subarray(bodyStart + length);
      const frame = JSON.parse(body) as ResponseFrame;
      const resolve = this.pending.get(frame.id);
      if (resolve) {
        this.pending.delete(frame.id);
        resolve(frame);
      }
    }
  }

  send(op: string, body: unknown): Promise<ResponseFrame> {
    const id = `r-${++this.nextId}`;
    const payload = Buffer.from(JSON.stringify({ type: "request", id, op, body }), "utf8");
    const framed = Buffer.concat([Buffer.from(`Content-Length: ${payload.length}\r\n\r\n`, "ascii"), payload]);
    return new Promise((resolve) => {
      this.pending.set(id, resolve);
      this.child.stdin.write(framed);
    });
  }

  /** Stdin EOF (engine-protocol spec §6/§7.1's orphan-prevention mechanism) — no explicit
   * shutdown call needed or, today, dispatched. */
  close(): void {
    this.child.stdin.end();
  }
}
