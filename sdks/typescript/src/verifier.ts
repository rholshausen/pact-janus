// The provider primitives (behavioural spec 'verifier', 'state-handler', 'request-filter',
// 'message-producer', 'verify'; ADR 0055, ADR 0056). A verification is one protocol run: this file reads the pact files the test names,
// writes the handlers as loopback `http` hook entries, sends one `verification/verify` and polls
// its stream. It decides no verdict: the run's status is the engine's, and a failed run fails here.

import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { protocolVocabulary, type hooks, type protocol } from "./generated/index.js";
import { Engine } from "./engine/engine.js";
import { Loopback } from "./loopback.js";
import type { FramePipe } from "./engine/pipe.js";
import { SubprocessPipe, type SubprocessOptions } from "./engine/subprocess.js";
import {
  StateUnsupportedError,
  VerificationFailedError,
  type VerificationResult,
  type VerificationResultEntry,
} from "./errors.js";
import { resolved, type ComponentDeclaration } from "./janus.js";

const Op = protocolVocabulary.RequestFrameOp;

/** How long one `events/poll` may wait. An empty answer only means nothing happened within it. */
const POLL_WAIT_MS = 10_000;

/** A source as the protocol spells it, without its `kind` (engine-protocol spec §8.3). */
export type SourceOptions = Omit<protocol.ContractSource, "kind">;

export interface VerifierConfig {
  /** Where the provider is: the HTTP transport's `base-url`. */
  providerUrl: string;
  /** Paths of Janus contracts or v1–v4 pacts, read and sent inline. One source only. */
  pacts?: readonly string[];
  /** A broker source: the pacts a Pact Broker selects for this provider. */
  broker?: SourceOptions;
  /** A urls source: pacts by URL. */
  urls?: SourceOptions;
  /**
   * The run's options, as `verification/verify` spells them: publish, filter, variants, … The run's
   * report is asked for unless they give `report` themselves (ADR 0059).
   */
  options?: protocol.VerifyOptions;
  /** The HTTP transport's drive options beside `base-url`: `headers`, `client`. Passed as given. */
  transport?: Record<string, unknown>;
  /** A project configuration file (`verifier.janus.yaml`), loaded through the engine. */
  config?: string;
  /** Declared components, as for `Janus`. */
  components?: readonly ComponentDeclaration[];
  /** How the engine is reached, as for `Janus`. */
  engine?: SubprocessOptions | (() => FramePipe);
}

/** A state's setup: receives its parameters, and may return provider-state values. */
export type StateSetup = (params: Record<string, unknown>) => unknown;

export interface StateHandler {
  setup: StateSetup;
  teardown?: (params: Record<string, unknown>) => unknown;
}

/**
 * Receives the request, one member per slot (`method`, `path`, `headers`, …), and returns it. A
 * filter that returns nothing changes nothing.
 */
export type RequestFilter = (request: Record<string, unknown>) => unknown;

/** A message a producer returns: bytes, a string or any value written as JSON, and what labels it. */
export interface Message {
  body: unknown;
  /** Labels the body. Absent: the engine reads the type from the metadata, else the octets. */
  contentType?: string;
  metadata?: Record<string, unknown>;
}

/** Receives today's request for the message: its description, provider states and any request. */
export type MessageProducer = (request: Record<string, unknown>) => Message | Promise<Message>;

export class Verifier {
  readonly #config: VerifierConfig;
  readonly #states: { state: string; handler: StateHandler }[] = [];
  readonly #filters: { filter: RequestFilter; changes: readonly string[] }[] = [];
  readonly #messages: { description: string; produce: MessageProducer }[] = [];

  constructor(config: VerifierConfig) {
    this.#config = config;
  }

  /** Declares the handler for one provider state. */
  state(state: string, handler: StateSetup | StateHandler): this {
    this.#states.push({ state, handler: typeof handler === "function" ? { setup: handler } : handler });
    return this;
  }

  /** Declares a request filter, changing the `changes` slots (default: headers). */
  requestFilter(filter: RequestFilter, options: { changes?: readonly string[] } = {}): this {
    this.#filters.push({ filter, changes: options.changes ?? ["headers"] });
    return this;
  }

  /** Declares the producer of the message interaction with this description. */
  message(description: string, produce: MessageProducer): this {
    this.#messages.push({ description, produce });
    return this;
  }

  /** Runs the verification. Throws `VerificationFailedError` when the run failed. */
  async verify(): Promise<VerificationResult> {
    const source = await this.#source();
    const option = this.#config.engine;
    const engine = await Engine.start(typeof option === "function" ? option() : await SubprocessPipe.start(option));
    const handlers = this.#states.length + this.#filters.length + this.#messages.length;
    const loopback = handlers > 0 ? new Loopback() : undefined;
    try {
      const config = this.#config.config
        ? (await engine.call(Op.ConfigLoad, { path: resolve(this.#config.config) })).config
        : undefined;
      await loopback?.start();
      const components = [...(config?.components ?? []), ...(this.#config.components ?? []).map(resolved)];
      const target: protocol.VerificationTarget = {
        transports: [{ transport: "http", options: { ...this.#config.transport, "base-url": this.#config.providerUrl } }],
        ...(components.length > 0 ? { components } : {}),
        ...(config || loopback ? { hooks: this.#hooks(config, loopback) } : {}),
      };
      const { stream } = await engine.call(Op.VerificationVerify, {
        source,
        target,
        options: { report: {}, ...this.#config.options },
      });
      const result = await poll(engine, stream);
      if (result.status === "failed") {
        throw new VerificationFailedError(result);
      }
      return result;
    } finally {
      await loopback?.stop();
      await engine.close();
    }
  }

  async #source(): Promise<protocol.ContractSource> {
    const { pacts, broker, urls } = this.#config;
    if ([pacts, broker, urls].filter((given) => given !== undefined).length !== 1) {
      throw new TypeError("a Verifier needs exactly one of 'pacts', 'broker' and 'urls'");
    }
    if (broker) {
      return { kind: "broker", ...broker };
    }
    if (urls) {
      return { kind: "urls", ...urls };
    }
    const contracts = await Promise.all(
      (pacts ?? []).map(async (path) => JSON.parse(await readFile(path, "utf8")) as Record<string, unknown>),
    );
    return { kind: "inline", contracts };
  }

  /** The configuration's entries at each point, then the handlers', each served by `loopback`. */
  #hooks(config: protocol.ResolvedConfig | undefined, loopback: Loopback | undefined): hooks.HookConfig {
    const points = structuredClone((config?.hooks ?? {}) as Record<string, hooks.HookEntry[]>);
    const add = (point: string, entry: hooks.HookEntry, route: (context: Context) => Promise<Answer>): void => {
      (points[point] ??= []).push(entry);
      loopback?.route(`/${point}/${entry.name}`, route);
    };
    const run = (point: string, name: string): hooks.Run => ({
      kind: "http",
      url: loopback?.url(`/${point}/${name}`) ?? "",
      headers: { authorization: loopback?.authorization ?? "" },
    });
    this.#states.forEach(({ state, handler }, i) => {
      const name = `state-${i + 1}`;
      add("state-setup", { name, when: { state }, run: run("state-setup", name) }, (context) =>
        answerState(() => handler.setup(paramsOf(context))),
      );
      const teardown = handler.teardown;
      if (teardown) {
        add("state-teardown", { name, when: { state }, run: run("state-teardown", name) }, (context) =>
          answerState(() => teardown(paramsOf(context))),
        );
      }
    });
    this.#filters.forEach(({ filter, changes }, i) => {
      const name = `request-filter-${i + 1}`;
      const declared = changes.map((slot) => `parts.request.${slot}`);
      add("before-request", { name, changes: declared, run: run("before-request", name) }, (context) =>
        answerFilter(filter, changes, context),
      );
    });
    this.#messages.forEach(({ description, produce }, i) => {
      const name = `message-${i + 1}`;
      const entry = { name, when: { interaction: description }, changes: ["parts"], run: run("produce-message", name) };
      add("produce-message", entry, (context) => answerMessage(produce, context));
    });
    return { version: (config?.version as number | undefined) ?? 1, hooks: points };
  }
}

type Context = Record<string, unknown>;
type Answer = Record<string, unknown>;

/** Polls the run's stream to its terminal event, collecting every interaction result. */
async function poll(engine: Engine, stream: string): Promise<VerificationResult> {
  const results: VerificationResultEntry[] = [];
  const warnings: Array<Record<string, unknown>> = [];
  let reported = false;
  for (;;) {
    const { events } = await engine.call(Op.EventsPoll, { streams: [stream], "wait-ms": POLL_WAIT_MS });
    for (const event of events) {
      if (event.kind === "verification/interaction-result") {
        results.push(event.payload as VerificationResultEntry);
      }
      if (event.kind === "verification/warning") {
        warnings.push(event.payload);
      }
      if (event.kind === "verification/report") {
        process.stdout.write(`${String(event.payload.text ?? "")}\n`);
        reported = true;
      }
      if (event.last) {
        if (reported) {
          process.stdout.write("\n"); // sets the report apart from what the test runner prints next
        }
        return { status: String(event.payload.status), summary: event.payload, results, warnings };
      }
    }
  }
}

function paramsOf(context: Context): Record<string, unknown> {
  const state = context.state as hooks.StateContext | undefined;
  return state?.params ?? {};
}

async function answerState(handler: () => unknown): Promise<Answer> {
  try {
    const values = await handler();
    return isObject(values) ? { outcome: "ok", values } : { outcome: "ok" };
  } catch (error) {
    if (error instanceof StateUnsupportedError) {
      return { outcome: "unsupported", error: { code: "state-unsupported", message: error.message } };
    }
    return failed(error);
  }
}

async function answerFilter(filter: RequestFilter, changes: readonly string[], context: Context): Promise<Answer> {
  const parts = (context.parts ?? {}) as Record<string, Record<string, { content?: unknown }>>;
  const request = Object.fromEntries(Object.entries(parts.request ?? {}).map(([slot, value]) => [slot, value.content]));
  try {
    const returned = await filter(request);
    if (!isObject(returned)) {
      return { outcome: "ok" };
    }
    const answered = changes.filter((slot) => slot in returned);
    return {
      outcome: "ok",
      changes: Object.fromEntries(answered.map((slot) => [`parts.request.${slot}`, { content: returned[slot] }])),
    };
  } catch (error) {
    return failed(error);
  }
}

async function answerMessage(produce: MessageProducer, context: Context): Promise<Answer> {
  const interaction = (context.interaction ?? {}) as { "pact-message"?: { request?: Record<string, unknown> } };
  try {
    const { body, contentType, metadata } = await produce(interaction["pact-message"]?.request ?? {});
    const octets =
      body instanceof Uint8Array ? body : new TextEncoder().encode(typeof body === "string" ? body : JSON.stringify(body));
    const slot: Answer = { content: Buffer.from(octets).toString("base64"), encoded: "base64" };
    if (contentType !== undefined) {
      slot["content-type"] = contentType;
    }
    const meta = { ...(contentType === undefined ? {} : { contentType }), ...metadata };
    return { outcome: "ok", changes: { parts: { message: { metadata: { content: meta }, body: slot } } } };
  } catch (error) {
    return failed(error);
  }
}

function failed(error: unknown): Answer {
  return { outcome: "failed", error: { code: "handler-failed", message: error instanceof Error ? error.message : String(error) } };
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
