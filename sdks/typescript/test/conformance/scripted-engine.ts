// The engine a `lifecycle` or `variants` case runs against: a frame pipe that records every
// operation the SDK sends and answers from the case's script (README §3). What these cases check
// is what the SDK sends and what it does with what comes back — not what an engine would decide.

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import type { protocol } from "../../src/generated/index.js";
import type { FramePipe } from "../../src/index.js";
import type { Invoke, ScriptedEngine } from "./cases.js";

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
  /** What the SDK's endpoint answered each of the script's `invoke` calls (provider cases). */
  readonly invocations: { status: number; result?: unknown }[] = [];
  readonly #script: ScriptedEngine;
  #handles = 0;
  #polled = false;

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
    if (frame.op === "events/poll") {
      return { type: "response", id: frame.id, ok: await this.#poll() };
    }
    return { type: "response", id: frame.id, ok: this.#reply(frame.op, frame.body) };
  }

  /**
   * The run's stream, all at once: the script's hooks are called first, as the engine would call
   * them during the run, then its events and the terminal one (README §3).
   */
  async #poll(): Promise<Record<string, unknown>> {
    if (this.#polled) {
      return { events: [] };
    }
    this.#polled = true;
    for (const invoke of this.#script.invoke ?? []) {
      this.invocations.push(await this.#invoke(invoke));
    }
    const events = [
      ...(this.#script.events ?? [
        {
          kind: "verification/interaction-result",
          payload: {
            interaction: { contract: { consumer: CONSUMER, provider: PROVIDER }, index: 0, description: "get an order" },
            variant: "base",
            status: "verified",
          },
        },
      ]),
      { kind: "verification/finished", payload: this.#script.summary ?? { status: "verified", variants: { verified: 1, failed: 0 } } },
    ];
    return {
      events: events.map((event, i) => ({
        stream: "s-1",
        seq: i + 1,
        ...event,
        ...(i === events.length - 1 ? { last: true } : {}),
      })),
    };
  }

  /** POSTs the context to the hook entry the `verify` frame names, with its headers. */
  async #invoke(invoke: Invoke): Promise<{ status: number; result?: unknown }> {
    const target = this.bodyOf("verification/verify")?.target as { hooks?: { hooks?: Record<string, unknown[]> } } | undefined;
    const entries = (target?.hooks?.hooks?.[invoke.point] ?? []) as { name: string; run: { url: string; headers?: Record<string, string> } }[];
    const entry = entries.find((e) => e.name === invoke.hook);
    if (!entry) {
      throw new Error(`the verify frame has no '${invoke.hook}' hook at '${invoke.point}'`);
    }
    const headers: Record<string, string> = { "content-type": "application/json", ...(entry.run.headers ?? {}) };
    if (invoke.authorised === false) {
      delete headers.authorization;
    }
    const response = await fetch(entry.run.url, { method: "POST", headers, body: JSON.stringify(invoke.context) });
    const text = await response.text();
    return { status: response.status, ...(text === "" ? {} : { result: JSON.parse(text) as unknown }) };
  }

  async close(): Promise<void> {
    this.closed = true;
  }

  #reply(op: string, body: Record<string, unknown>): Record<string, unknown> {
    switch (op) {
      case "engine/hello":
        return {
          "protocol-version": 1,
          engine: { name: "scripted", version: "0.0.0" },
          capabilities: this.#script.capabilities ?? { "config-loading": {} },
        };
      case "config/load":
        return { config: this.#script.config ?? { version: 1, hooks: {} } };
      case "verification/verify":
        return { session: "vs-1", stream: "s-1" };
      case "consumer-session/create":
        return { session: "s-1" };
      case "consumer-session/add-interaction":
        return { handle: `i-${++this.#handles}` };
      case "consumer-session/start-transport":
        return { endpoint: { kind: "http", "base-url": "http://127.0.0.1:1" } };
      case "consumer-session/variants":
        return { variants: this.#script.variants ?? [{ id: "base", label: "base" }] };
      case "consumer-session/serve-variant": {
        const parts = this.#script.served?.[String(body.variant)];
        return parts === undefined ? {} : { parts };
      }
      case "consumer-session/report-variant": {
        // The case's result for this variant, else the report's own verdict (README §3).
        const variant = String(body.variant);
        const test = { status: body.status, ...(body.reason === undefined ? {} : { reason: body.reason }) };
        const result = this.#script.reports?.[variant] ?? {
          variant,
          status: body.status === "passed" ? "verified" : "failed",
          test,
        };
        return { result };
      }
      case "consumer-session/finalise":
        return {
          results: this.#script.results ?? this.#verifiedResults(),
          ...(this.#script.refused === undefined ? {} : { refused: this.#script.refused }),
          ...(this.#script["withhold-contract"] ? {} : this.#contract(body.write as protocol.ContractWrite | undefined)),
        };
      case "provider-shape-session/create":
        return { session: "r-1" };
      case "provider-shape-session/observe":
        return { observations: this.ops.filter((op) => op === "provider-shape-session/observe").length, interactions: 1 };
      case "provider-shape-session/finalise": {
        // As the engine's host would (ADR 0086 decision 7): the shape to the file the request names.
        const shape = { $format: "janus-provider-shape/1", provider: { name: PROVIDER }, provenance: "recorded", interactions: [] };
        const write = body.write as protocol.ShapeWrite | undefined;
        if (write !== undefined) {
          mkdirSync(dirname(write.path), { recursive: true });
          writeFileSync(write.path, `${JSON.stringify(shape)}\n`, "utf8");
        }
        return { "provider-shape": shape, ...(write === undefined ? {} : { written: write.path }) };
      }
      default:
        return {};
    }
  }

  /**
   * The contract, as a real engine answers it. Asked to write it (ADR 0065), it writes its canonical
   * text to the file, as the engine's host would, and answers the path; otherwise its text to a host
   * that declared 'contract-text' (ADR 0063), else the document. A case's 'contract-text' is that text
   * exactly; without one, it is the case's document written compactly.
   */
  #contract(write: protocol.ContractWrite | undefined): Record<string, unknown> {
    const contract = this.#script.contract ?? DEFAULT_CONTRACT;
    const text = this.#script["contract-text"] ?? `${JSON.stringify(contract)}\n`;
    if (write !== undefined) {
      mkdirSync(dirname(write.path), { recursive: true });
      writeFileSync(write.path, text, "utf8");
      return { written: write.path };
    }
    const capabilities = (this.bodyOf("engine/hello")?.capabilities ?? {}) as Record<string, unknown>;
    if (capabilities["contract-text"] === undefined) {
      return { contract };
    }
    return { "contract-text": text };
  }

  #verifiedResults(): Record<string, unknown>[] {
    return Array.from({ length: this.#handles }, (_, i) => ({ handle: `i-${i + 1}`, status: "verified" }));
  }
}
