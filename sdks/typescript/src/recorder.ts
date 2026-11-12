// The recording primitives (behavioural spec 'shape-recorder', 'observe', 'finish-recording';
// ADR 0086). A recorder is one provider-shape session: each answer the test hands it goes to the
// engine as the HTTP transport presents one, and the engine's host writes the shape. Nothing here
// decodes a body or decides what the shape is.

import { relative, resolve, sep } from "node:path";
import { protocolVocabulary, type protocol } from "./generated/index.js";
import { Engine } from "./engine/engine.js";
import type { FramePipe } from "./engine/pipe.js";
import { SubprocessPipe, type SubprocessOptions } from "./engine/subprocess.js";
import { compile, Shape, type Template } from "./shapes.js";

const Op = protocolVocabulary.RequestFrameOp;

export interface ShapeRecorderConfig {
  /** The provider the shape is about. */
  provider: string;
  /** Where the shape is written, as `<provider>.provider-shape.json`. Default: `shapes`. */
  shapeDir?: string;
  /** Where the recorder's counts are kept. Not a file to publish. Default: `.janus/evidence`. */
  evidenceDir?: string;
  /** Whose evidence this is, replaced when it records again. Default: the test file (`useShapeRecorder`, or Jest's global `expect`). */
  source?: string;
  /** The recorder's judgements, passed as given (engine-protocol spec §8.5). */
  policy?: protocol.RecordingPolicy;
  /** How the engine is reached, as for `Janus`. */
  engine?: SubprocessOptions | (() => FramePipe);
  /** How much the engine logs, as for `Janus`. */
  logLevel?: string;
}

/** The operation an answer was to: a request's slots, `method` and `path` the usual two. */
export type Operation = Record<string, Template>;

/** What the provider answered, when it is not a `fetch` `Response`. */
export interface RecordedResponse {
  status: number;
  /** Each header's value, or its field lines. */
  headers?: Record<string, string | readonly string[]>;
  body?: Uint8Array | string;
}

export interface ObserveOptions {
  /** What a report calls the entry. Default: the operation's method and path, `GET /orders/{id}`. */
  description?: string;
  /** The provider states the answer was given in, by name. */
  states?: readonly string[];
  /** The request the answer was to, which the engine checks the operation admits. */
  request?: { method: string; path: string };
}

type Slots = Record<string, protocol.ObservedSlotValue>;

export class ShapeRecorder {
  readonly #config: ShapeRecorderConfig;
  #started?: Promise<{ engine: Engine; session: string }>;
  #finished = false;

  constructor(config: ShapeRecorderConfig) {
    this.#config = config;
  }

  /**
   * Records one answer. A `fetch` `Response` is read from a clone, so hand it over before reading
   * its body. Resolves with the engine's answer, `warnings` included.
   */
  async observe(
    operation: Operation,
    response: RecordedResponse | Response,
    options: ObserveOptions = {},
  ): Promise<protocol.ObserveResponseResult> {
    const description = options.description ?? describe(operation);
    const answer = response instanceof Response ? await read(response) : response;
    const parts: Record<string, Slots> = { response: responseSlots(answer) };
    if (options.request) {
      parts.request = { method: { content: options.request.method }, path: { content: options.request.path } };
    }
    const { engine, session } = await this.#start();
    return engine.call(Op.ProviderShapeSessionObserve, {
      session,
      description,
      ...(options.states ? { states: [...options.states] } : {}),
      operation: { request: Object.fromEntries(Object.entries(operation).map(([slot, value]) => [slot, compile(value)])) },
      parts,
    });
  }

  /** Has the engine write the shape, merged with every other source's, and ends it. Resolves with the path. */
  async finish(): Promise<string | undefined> {
    const started = this.#started;
    if (this.#finished || !started) {
      this.#finished = true;
      return undefined;
    }
    this.#finished = true;
    const { engine, session } = await started;
    try {
      const { provider, shapeDir, evidenceDir } = this.#config;
      const source = this.#config.source ?? testFile();
      if (source === undefined) {
        throw new TypeError("finish: name the recorder's 'source', since no test file is running");
      }
      const write = {
        path: resolve(shapeDir ?? "shapes", `${provider}.provider-shape.json`),
        evidence: resolve(evidenceDir ?? ".janus/evidence", `${provider}.json`),
        source,
      };
      return (await engine.call(Op.ProviderShapeSessionFinalise, { session, write })).written;
    } finally {
      await engine.close();
    }
  }

  #start(): Promise<{ engine: Engine; session: string }> {
    this.#started ??= (async () => {
      const option = this.#config.engine;
      const engine = await Engine.start(
        typeof option === "function" ? option() : await SubprocessPipe.start(option, this.#config.logLevel),
      );
      const { provider, policy } = this.#config;
      const { session } = await engine.call(Op.ProviderShapeSessionCreate, {
        provider: { name: provider },
        ...(policy ? { policy } : {}),
      });
      return { engine, session };
    })();
    return this.#started;
  }
}

/** `GET /orders/{id}`: the operation's plain method and plain path or path template. */
function describe(operation: Operation): string {
  const { method, path } = operation;
  const template = path instanceof Shape ? (path.toJSON() as { shape?: string; template?: string }) : undefined;
  const pathText = typeof path === "string" ? path : template?.shape === "http:path" ? template.template : undefined;
  if (typeof method !== "string" || pathText === undefined) {
    throw new TypeError(
      "observe: name the entry with 'description', since the operation's method and path are not a plain value and a plain value or path template",
    );
  }
  return `${method} ${pathText}`;
}

async function read(response: Response): Promise<RecordedResponse> {
  if (response.bodyUsed) {
    throw new TypeError("observe: hand the Response over before reading its body");
  }
  const headers: Record<string, string[]> = {};
  response.headers.forEach((value, name) => {
    headers[name] = name === "set-cookie" ? response.headers.getSetCookie() : [value];
  });
  return { status: response.status, headers, body: new Uint8Array(await response.clone().arrayBuffer()) };
}

/** The answer as the HTTP transport presents a response (component-interfaces spec §5.9). */
function responseSlots({ status, headers, body }: RecordedResponse): Slots {
  const slots: Slots = { status: { content: status } };
  const lines: Record<string, string[]> = {};
  for (const [name, value] of Object.entries(headers ?? {})) {
    (lines[name.toLowerCase()] ??= []).push(...(typeof value === "string" ? [value] : value));
  }
  if (Object.keys(lines).length > 0) {
    slots.headers = { content: lines };
  }
  const octets = typeof body === "string" ? new TextEncoder().encode(body) : body;
  if (octets && octets.length > 0) {
    const contentType = lines["content-type"]?.[0];
    slots.body = {
      content: Buffer.from(octets).toString("base64"),
      encoded: "base64",
      ...(contentType === undefined ? {} : { "content-type": contentType }),
    };
  }
  return slots;
}

/** A test file as a recorder's source: relative to the working directory, written with `/`. */
export function testSource(path: string | undefined): string | undefined {
  return path === undefined ? undefined : relative(process.cwd(), path).split(sep).join("/");
}

/** The running test file, where the test framework puts `expect` on the global object (Jest's default). */
function testFile(): string | undefined {
  const expect = (globalThis as { expect?: { getState?: () => { testPath?: string } } }).expect;
  return testSource(expect?.getState?.().testPath);
}
