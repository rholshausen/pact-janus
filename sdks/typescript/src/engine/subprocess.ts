// The subprocess embedding (ADR 0003): `janus-engine` speaking frames over stdio with
// `Content-Length` framing (engine-protocol spec §3.2, spike 1.3). Orphan prevention is stdin EOF —
// the engine exits when its stdin closes, so there is no kill and no signal choreography here.

import { spawn, type ChildProcessByStdio } from "node:child_process";
import type { Readable, Writable } from "node:stream";
import type { protocol } from "../generated/index.js";
import type { FramePipe } from "./pipe.js";

export interface SubprocessOptions {
  /** The `janus-engine` executable. Defaults to the `JANUS_ENGINE` environment variable. */
  command?: string;
  /**
   * Environment for the engine. `RUST_LOG` defaults to `warn`, so a request the mock could not
   * match is reported on stderr but the engine's start-up chatter is not.
   */
  env?: NodeJS.ProcessEnv;
}

const HEADER_END = Buffer.from("\r\n\r\n");

export class SubprocessPipe implements FramePipe {
  private readonly child: ChildProcessByStdio<Writable, Readable, null>;
  private buffer = Buffer.alloc(0);
  private readonly pending = new Map<
    string,
    { resolve: (frame: protocol.ResponseFrame) => void; reject: (error: Error) => void }
  >();
  private failure: Error | undefined;
  private readonly exited: Promise<void>;

  constructor(options: SubprocessOptions = {}) {
    const command = options.command ?? process.env.JANUS_ENGINE;
    if (!command) {
      throw new Error(
        "No janus-engine to run: set JANUS_ENGINE to the janus-engine executable " +
          "(`cargo build -p pact_janus_cli --bin janus-engine` builds target/debug/janus-engine), " +
          "or pass engine.command.",
      );
    }
    const env = { ...process.env, ...options.env };
    env.RUST_LOG ??= "warn";
    this.child = spawn(command, [], { stdio: ["pipe", "pipe", "inherit"], env });
    this.child.stdout.on("data", (chunk: Buffer) => this.onData(chunk));
    this.exited = new Promise((resolve) => {
      this.child.on("error", (error) => {
        this.fail(new Error(`janus-engine (${command}) could not be started: ${error.message}`));
        resolve();
      });
      this.child.on("exit", (code, signal) => {
        this.fail(new Error(`janus-engine exited (${signal ?? `code ${code}`}) with requests outstanding`));
        resolve();
      });
    });
  }

  private fail(error: Error): void {
    this.failure ??= error;
    for (const { reject } of this.pending.values()) {
      reject(error);
    }
    this.pending.clear();
  }

  private onData(chunk: Buffer): void {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    for (;;) {
      const headerEnd = this.buffer.indexOf(HEADER_END);
      if (headerEnd === -1) {
        return;
      }
      const header = this.buffer.subarray(0, headerEnd).toString("ascii");
      const length = /^Content-Length:\s*(\d+)\s*$/im.exec(header)?.[1];
      if (length === undefined) {
        // A byte-counted stream cannot resynchronise (engine-protocol spec §3.2).
        this.fail(new Error(`janus-engine framing: no Content-Length in '${header}'`));
        this.child.stdin.end();
        return;
      }
      const start = headerEnd + HEADER_END.length;
      const end = start + Number(length);
      if (this.buffer.length < end) {
        return;
      }
      const frame = JSON.parse(this.buffer.subarray(start, end).toString("utf8")) as protocol.ResponseFrame;
      this.buffer = this.buffer.subarray(end);
      const waiting = this.pending.get(frame.id);
      if (waiting) {
        this.pending.delete(frame.id);
        waiting.resolve(frame);
      }
      // A response nobody is waiting for — including a pipe-level one with an empty id (spec
      // §4.4) — has nowhere to go; the request it answers will fail by other means.
    }
  }

  exchange(frame: protocol.RequestFrame): Promise<protocol.ResponseFrame> {
    if (this.failure) {
      return Promise.reject(this.failure);
    }
    const body = Buffer.from(JSON.stringify(frame), "utf8");
    return new Promise((resolve, reject) => {
      this.pending.set(frame.id, { resolve, reject });
      this.child.stdin.write(Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, "ascii"), body]));
    });
  }

  async close(): Promise<void> {
    this.child.stdin.end();
    await this.exited;
  }
}
