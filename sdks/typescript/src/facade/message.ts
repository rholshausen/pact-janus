// pact-js's message consumer API over the engine (ADR 0066, staged plan S2.5c): `PactV4`'s
// `addAsynchronousInteraction` and `addSynchronousInteraction`, `MessageConsumerPact`, and the body
// handlers. Each builder records the `pact_ffi` calls pact-js's builder makes, as the HTTP ones do,
// and a test is handed the message the engine answers for its base variant (ADR 0067 decision 5),
// in the form pact-js hands it. Nothing here matches anything or decides a verdict.
//
// The builders follow pact-js (MIT, Copyright (c) 2014 DiUS and the pact-js contributors) at
// 8a8f27a9: src/v4/message/, src/messageConsumerPact.ts and src/dsl/message.ts.

import { received, type ReceivedMessage } from "../message.js";
import { Calls, rulesForFfi, run, type JsonMap, type PactV4Options, type Part, type Rules, type Served } from "./consumer.js";

export type AnyJson = boolean | number | string | null | AnyJson[] | { [key: string]: AnyJson };
export type Metadata = Record<string, unknown>;

export interface ProviderState {
  name: string;
  params?: Record<string, unknown>;
}

export interface MessageContents {
  content: AnyJson | Buffer;
}

/** What a V4 asynchronous message test is handed. */
export interface AsynchronousMessage {
  contents: MessageContents;
  metadata: Metadata;
}

/** What a V4 synchronous message test is handed: each half's octets. */
export interface SynchronousMessage {
  Request: MessageContents;
  Response: MessageContents[];
}

/** What a `MessageConsumerPact` handler is handed. */
export interface ConcreteMessage {
  providerStates?: ProviderState[];
  description?: string;
  metadata?: Metadata;
  contents: AnyJson | Buffer;
}

export type MessageConsumer = (m: ConcreteMessage) => Promise<unknown>;
export type V4MessageConsumer = (m: AsynchronousMessage) => Promise<unknown>;

/** pact-js's message `PluginConfig`, whose use is refused. */
interface PluginConfig {
  plugin: string;
  version: string;
}

/** ramda's `isEmpty`, which pact-js's V4 builders check with. */
function isEmpty(value: unknown): boolean {
  return value === "" || (Array.isArray(value) && value.length === 0) || (isObject(value) && Object.keys(value).length === 0);
}

/** lodash's `isEmpty`, which `MessageConsumerPact` checks with: any number or boolean is empty too. */
function isBlank(value: unknown): boolean {
  return value === null || value === undefined || typeof value !== "object" ? typeof value !== "string" || value === "" : isEmpty(value);
}

function isObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value) && !Buffer.isBuffer(value);
}

function refusePlugin(config: PluginConfig): never {
  throw new Error(
    `usingPlugin: the pact plugin '${config.plugin}' is refused, since pact-janus cannot load pact plugins (ADR 0045, ADR 0066 decision 6)`,
  );
}

/** What pact-js's reified contents are: octets the test gave as octets, a JSON body as its document, any other as text. */
function content(message: ReceivedMessage, binary: boolean): AnyJson | Buffer {
  if (binary) {
    return Buffer.from(message.body);
  }
  return /^\s*application\/([^;]*\+)?json\s*(;|$)/i.test(message.contentType ?? "") ? (message.json() as AnyJson) : message.text();
}

function delivered(served: Served): ReceivedMessage {
  if (served.parts === undefined) {
    throw new Error("the engine handed over no message");
  }
  return received(served.parts);
}

/** A message's metadata, member by member, as pact-js passes each: its JSON text. */
function withMetadata(calls: Calls, metadata: Metadata): void {
  if (isEmpty(metadata)) {
    throw new Error("You must provide valid metadata for the Message, or none at all");
  }
  for (const [name, value] of Object.entries(metadata)) {
    calls.call("message_with_metadata_v2", { name, value: JSON.stringify(value) });
  }
}

function jsonContent(content: unknown): string {
  if (isEmpty(content)) {
    throw new Error("You must provide a valid JSON document or primitive for the Message.");
  }
  return JSON.stringify(content);
}

/** What the V4 message builders share before their contents: the states and the decorations. */
class UnconfiguredMessage {
  protected readonly opts: PactV4Options;
  protected readonly calls: Calls;
  readonly #given: string;

  constructor(opts: PactV4Options, calls: Calls, given: string) {
    this.opts = opts;
    this.calls = calls;
    this.#given = given;
  }

  given(state: string, parameters?: JsonMap): this {
    this.calls.call(parameters ? "given_with_params" : this.#given, { description: state, ...(parameters ? { params: parameters } : {}) });
    return this;
  }

  /** Accepted; no effect. A contract has no pending interactions: the broker decides (ADR 0051). */
  pending(pending = true): this {
    void pending;
    return this;
  }

  /** Accepted; no effect. A contract records no comments, test names or references. */
  comment(comment: string | { key: string; value: string }): this {
    void comment;
    return this;
  }

  testName(name: string): this {
    void name;
    return this;
  }

  reference(group: string, name: string, value: string): this {
    void [group, name, value];
    return this;
  }

  usingPlugin(config: PluginConfig): never {
    return refusePlugin(config);
  }
}

// ---------------------------------------------------------------------------------------------
// Asynchronous messages

export class UnconfiguredAsynchronousMessage extends UnconfiguredMessage {
  constructor(opts: PactV4Options, calls: Calls) {
    super(opts, calls, "message_given");
  }

  expectsToReceive(description: string, builder: (builder: AsynchronousMessageBuilder) => void): AsynchronousMessageWithContent {
    this.calls.call("message_expects_to_receive", { description });
    const contents = new AsynchronousMessageBuilder(this.calls);
    builder(contents);
    return new AsynchronousMessageWithContent(this.opts, this.calls, contents.binary);
  }
}

export class AsynchronousMessageBuilder {
  readonly #calls: Calls;
  /** Whether the contents were given as octets, which the test is handed back as a `Buffer`. */
  binary = false;

  constructor(calls: Calls) {
    this.#calls = calls;
  }

  withMetadata(metadata: Metadata): this {
    withMetadata(this.#calls, metadata);
    return this;
  }

  withContent(contentType: string, body: Buffer): this {
    this.#calls.call("message_with_contents", { content_type: contentType, contents: body.toString("base64") });
    this.binary = true;
    return this;
  }

  withJSONContent(content: unknown): this {
    this.#calls.call("message_with_contents", { content_type: "application/json", body: jsonContent(content) });
    return this;
  }

  withMatchingRules(rules: Rules): this {
    this.#calls.call("with_matching_rules", { part: "request", rules: rulesForFfi(rules) });
    return this;
  }
}

export class AsynchronousMessageWithContent {
  readonly #opts: PactV4Options;
  readonly #calls: Calls;
  readonly #binary: boolean;

  constructor(opts: PactV4Options, calls: Calls, binary: boolean) {
    this.#opts = opts;
    this.#calls = calls;
    this.#binary = binary;
  }

  executeTest<T>(integrationTest: (m: AsynchronousMessage) => Promise<T>): Promise<T | undefined> {
    return run(this.#opts, "V4", [this.#calls], undefined, (served) => {
      const message = delivered(served);
      return integrationTest({ contents: { content: content(message, this.#binary) }, metadata: { ...message.metadata } });
    });
  }
}

// ---------------------------------------------------------------------------------------------
// Synchronous messages

export class UnconfiguredSynchronousMessage extends UnconfiguredMessage {
  constructor(opts: PactV4Options, calls: Calls) {
    super(opts, calls, "given");
  }

  withRequest(builder: (builder: SynchronousMessagePartBuilder) => void): SynchronousMessageWithRequest {
    builder(new SynchronousMessagePartBuilder(this.calls, "request"));
    return new SynchronousMessageWithRequest(this.opts, this.calls);
  }
}

/** A synchronous message's request or response builder. */
export class SynchronousMessagePartBuilder {
  readonly #calls: Calls;
  readonly #part: Part;

  constructor(calls: Calls, part: Part) {
    this.#calls = calls;
    this.#part = part;
  }

  /** Accepted as pact-js accepts it, for the response: `pact_ffi` writes no synchronous message's metadata. */
  withMetadata(metadata: Metadata): this {
    withMetadata(this.#calls, metadata);
    return this;
  }

  withContent(contentType: string, body: Buffer): this {
    this.#calls.call("with_binary_file", { part: this.#part, content_type: contentType, body: body.toString("base64") });
    return this;
  }

  withJSONContent(content: unknown): this {
    this.#calls.call("with_body", { part: this.#part, content_type: "application/json", body: jsonContent(content) });
    return this;
  }

  withMatchingRules(rules: Rules): this {
    this.#calls.call("with_matching_rules", { part: this.#part, rules: rulesForFfi(rules) });
    return this;
  }
}

export class SynchronousMessageWithRequest {
  readonly #opts: PactV4Options;
  readonly #calls: Calls;

  constructor(opts: PactV4Options, calls: Calls) {
    this.#opts = opts;
    this.#calls = calls;
  }

  withResponse(builder: (builder: SynchronousMessagePartBuilder) => void): SynchronousMessageWithResponse {
    builder(new SynchronousMessagePartBuilder(this.#calls, "response"));
    return new SynchronousMessageWithResponse(this.#opts, this.#calls);
  }
}

export class SynchronousMessageWithResponse {
  readonly #opts: PactV4Options;
  readonly #calls: Calls;

  constructor(opts: PactV4Options, calls: Calls) {
    this.#opts = opts;
    this.#calls = calls;
  }

  /** The test is handed each half's octets, its one response being the first, which a contract keeps (ADR 0067). */
  executeTest<T>(integrationTest: (m: SynchronousMessage) => Promise<T>): Promise<T | undefined> {
    return run(this.#opts, "V4", [this.#calls], undefined, (served) => {
      const message = delivered(served);
      return integrationTest({
        Request: { content: Buffer.from(message.request?.body ?? []) },
        Response: [{ content: Buffer.from(message.body) }],
      });
    });
  }
}

// ---------------------------------------------------------------------------------------------
// MessageConsumerPact, pact-js's V3 message API

export interface MessageConsumerOptions {
  consumer: string;
  provider: string;
  dir?: string;
  log?: string;
  logLevel?: PactV4Options["logLevel"];
  /** The pact specification, as a number: 2, 3 (the default) or 4. */
  spec?: number;
  /** Accepted; contracts from several tests are always merged (ADR 0065). */
  pactfileWriteMode?: "overwrite" | "update" | "merge" | "none";
  engine?: PactV4Options["engine"];
}

export class MessageConsumerPact {
  readonly #config: MessageConsumerOptions;
  #calls = new Calls().call("new_message", { description: "" });
  #states: ProviderState[] = [];
  #binary = false;

  constructor(config: MessageConsumerOptions) {
    this.#config = config;
  }

  given(state: string | ProviderState): this {
    const { name, params } = typeof state === "string" ? { name: state, params: undefined } : state;
    this.#calls.call(params ? "given_with_params" : "message_given", { description: name, ...(params ? { params } : {}) });
    this.#states.push(params ? { name, params } : { name });
    return this;
  }

  expectsToReceive(description: string): this {
    if (isBlank(description)) {
      throw new Error("You must provide a description for the Message.");
    }
    this.#calls.call("message_expects_to_receive", { description });
    return this;
  }

  withContent(content: unknown): this {
    if (isBlank(content)) {
      throw new Error("You must provide a valid JSON document or primitive for the Message.");
    }
    this.#calls.call("message_with_contents", { content_type: "application/json", body: JSON.stringify(content) });
    this.#binary = false;
    return this;
  }

  withTextContent(content: string, contentType: string): this {
    this.#calls.call("message_with_contents", { content_type: contentType, body: content });
    this.#binary = false;
    return this;
  }

  withBinaryContent(content: Buffer, contentType: string): this {
    this.#calls.call("message_with_contents", { content_type: contentType, contents: content.toString("base64") });
    this.#binary = true;
    return this;
  }

  withMetadata(metadata: Metadata): this {
    if (isBlank(metadata)) {
      throw new Error("You must provide valid metadata for the Message, or none at all");
    }
    withMetadata(this.#calls, metadata);
    return this;
  }

  /** The handler is handed the message; the contract is written when it resolves. */
  async verify(handler: MessageConsumer): Promise<unknown> {
    const { spec, ...opts } = this.#config;
    const calls = this.#calls;
    const states = this.#states;
    const binary = this.#binary;
    this.#calls = new Calls().call("new_message", { description: "" });
    this.#states = [];
    this.#binary = false;
    const specification = { 2: "V2", 3: "V3", 4: "V4" }[spec ?? 3] ?? "V3";
    return run(opts, specification, [calls], undefined, (served) => {
      const message = delivered(served);
      return handler({
        description: calls.description,
        ...(states.length > 0 ? { providerStates: states } : {}),
        contents: content(message, binary),
        metadata: { ...message.metadata },
      });
    });
  }
}

// ---------------------------------------------------------------------------------------------
// The body handlers: a handler of a message's contents, as a message test

export function synchronousBodyHandler<R>(handler: (body: AnyJson | Buffer) => R): MessageConsumer {
  return async (m) => handler(m.contents);
}

export function asynchronousBodyHandler<R>(handler: (body: AnyJson | Buffer) => Promise<R>): MessageConsumer {
  return (m) => handler(m.contents);
}

export function v4SynchronousBodyHandler<R>(handler: (body: AnyJson | Buffer) => R): V4MessageConsumer {
  return async (m) => handler(m.contents.content);
}

export function v4AsynchronousBodyHandler<R>(handler: (body: AnyJson | Buffer) => Promise<R>): V4MessageConsumer {
  return (m) => handler(m.contents.content);
}
