// The subprocess embedding (ADR 0003): `janus-engine` speaking frames over stdio with
// `Content-Length` framing (engine-protocol spec §3.2, spike 1.3). Orphan prevention is stdin EOF —
// the engine exits when its stdin closes, so there is no kill and no signal choreography here.

import { spawn, type ChildProcessByStdio } from "node:child_process";
import type { Readable, Writable } from "node:stream";
import type { protocol } from "../generated/index.js";
import type { FramePipe } from "./pipe.js";
import { engineEnvironment, engineNotFetched, engineNotStarted, resolveEngine, type EngineSource } from "./release.js";

export interface SubprocessOptions {
  /**
   * The `janus-engine` executable. Defaults to the `JANUS_ENGINE` environment variable, then to the
   * engine this SDK was released with, fetched on first use (ADR 0060 decision 6).
   */
  command?: string;
  /**
   * Environment for the engine. A `RUST_LOG` here or in the process's environment is passed as given;
   * otherwise the engine logs at the configured `logLevel` (ADR 0061).
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

  /**
   * Starts the engine `options` name, or the one this SDK was released with, logging at `logLevel`
   * (a test's `logLevel` option, else `LOG_LEVEL`, else `warn`) unless `RUST_LOG` is given.
   */
  static async start(options: SubprocessOptions = {}, logLevel?: string): Promise<SubprocessPipe> {
    const env = engineEnvironment({ ...process.env, ...options.env }, logLevel);
    const named = process.env.JANUS_ENGINE || undefined;
    if (options.command) {
      return new SubprocessPipe(options.command, "engine.command", env);
    }
    if (named) {
      return new SubprocessPipe(named, "JANUS_ENGINE", env);
    }
    const command = await resolveEngine().catch((error: unknown) => {
      throw engineNotFetched(error instanceof Error ? error.message : String(error));
    });
    return new SubprocessPipe(command, "release", env);
  }

  private constructor(command: string, source: EngineSource, env: NodeJS.ProcessEnv) {
    this.child = spawn(command, [], { stdio: ["pipe", "pipe", "inherit"], env });
    this.child.stdout.on("data", (chunk: Buffer) => this.onData(chunk));
    this.exited = new Promise((resolve) => {
      this.child.on("error", (error: NodeJS.ErrnoException) => {
        this.fail(engineNotStarted(command, source, error));
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
