// The engine a `lifecycle` or `variants` case runs against: a frame pipe that records every
// operation the SDK sends and answers from the case's script (README §3). What these cases check
// is what the SDK sends and what it does with what comes back — not what an engine would decide.

import type { protocol } from "../../src/generated/index.js";
import type { FramePipe } from "../../src/index.js";
import type { ScriptedEngine } from "./cases.js";

export const CONSUMER = "web-app";
export const PROVIDER = "orders-api";

const DEFAULT_CONTRACT = {
  $format: "janus-contract/1",
  consumer: { name: CONSUMER },
  provider: { name: PROVIDER },
  interactions: [],
};

export class ScriptedPipe implements FramePipe {
  /** Every operation sent, in order. */
  readonly ops: string[] = [];
  /** Every request frame body, alongside `ops`. */
  readonly bodies: Record<string, unknown>[] = [];
  closed = false;
  readonly #script: ScriptedEngine;
  #handles = 0;

  constructor(script: ScriptedEngine = {}) {
    this.#script = script;
  }

  /** The first frame body sent for `op`. */
  bodyOf(op: string): Record<string, unknown> | undefined {
    const at = this.ops.indexOf(op);
    return at < 0 ? undefined : this.bodies[at];
  }

  async exchange(frame: protocol.RequestFrame): Promise<protocol.ResponseFrame> {
    this.ops.push(frame.op);
    this.bodies.push(frame.body);
    const scripted = this.#script.errors?.[frame.op];
    // `after` answers that many calls the default way first: the engine that dies mid-loop.
    const before = this.ops.filter((op) => op === frame.op).length - 1;
    if (scripted && before >= (scripted.after ?? 0)) {
      const { code, message, problems, details } = scripted as Record<string, unknown> & { code: string };
      return {
        type: "response",
        id: frame.id,
        error: {
          code,
          message: (message as string) ?? code,
          category: "request",
          details: { ...((details as Record<string, unknown>) ?? {}), ...(problems ? { problems } : {}) },
        } as protocol.EngineError,
      };
    }
    return { type: "response", id: frame.id, ok: this.#reply(frame.op) };
  }

  async close(): Promise<void> {
    this.closed = true;
  }

  #reply(op: string): Record<string, unknown> {
    switch (op) {
      case "engine/hello":
        return { "protocol-version": 1, engine: { name: "scripted", version: "0.0.0" }, capabilities: {} };
      case "consumer-session/create":
        return { session: "s-1" };
      case "consumer-session/add-interaction":
        return { handle: `i-${++this.#handles}` };
      case "consumer-session/start-transport":
        return { endpoint: { kind: "http", "base-url": "http://127.0.0.1:1" } };
      case "consumer-session/variants":
        return { variants: this.#script.variants ?? [{ id: "base", label: "base" }] };
      case "consumer-session/finalise":
        return {
          results: this.#script.results ?? this.#verifiedResults(),
          ...(this.#script["withhold-contract"] ? {} : { contract: this.#script.contract ?? DEFAULT_CONTRACT }),
        };
      default:
        return {};
    }
  }

  #verifiedResults(): Record<string, unknown>[] {
    return Array.from({ length: this.#handles }, (_, i) => ({ handle: `i-${i + 1}`, status: "verified" }));
  }
}
