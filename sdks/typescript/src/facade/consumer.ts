// pact-js's consumer API over the engine (ADR 0066): `PactV4` (`Pact`), its HTTP and GraphQL
// builders, and `PactV3`; its message builders are `message.ts`'s. Each builder records the `pact_ffi` calls pact-js's builder makes, in the
// order it makes them, and `executeTest` hands them to `consumer-session/add-pact-interaction`, which
// builds and converts the interaction as `janus upgrade` converts a pact. The test runs once, against
// the base variant, as it does today. Nothing here matches anything or decides a verdict.
//
// The builders follow pact-js (MIT, Copyright (c) 2014 DiUS and the pact-js contributors) at
// 8a8f27a9: src/v4/http/, src/v4/graphql/, src/v3/pact.ts, src/v3/ffi.ts and
// src/common/matchingRules.ts.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { protocolVocabulary, type protocol } from "../generated/index.js";
import { Engine } from "../engine/engine.js";
import type { FramePipe } from "../engine/pipe.js";
import { SubprocessPipe, type SubprocessOptions } from "../engine/subprocess.js";
import { causeText, ContractWithheldError } from "../errors.js";
import { isMatcher, isStatusCodeMatcher, reify, regex, type Matcher, type StatusCodeMatcher } from "./matchers.js";
import { UnconfiguredAsynchronousMessage, UnconfiguredSynchronousMessage } from "./message.js";

const Op = protocolVocabulary.RequestFrameOp;

export const SpecificationVersion = {
  SPECIFICATION_VERSION_V2: 3,
  SPECIFICATION_VERSION_V3: 4,
  SPECIFICATION_VERSION_V4: 5,
} as const;
export type SpecificationVersion = (typeof SpecificationVersion)[keyof typeof SpecificationVersion];

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error";

export interface PactV4Options {
  consumer: string;
  provider: string;
  /** Where the contract is written, as `<consumer>-<provider>.janus.json`. Default `./pacts`. */
  dir?: string;
  spec?: SpecificationVersion;
  logLevel?: LogLevel;
  /** The mock's address. Default: the loopback interface, at a port the system chooses. */
  host?: string;
  port?: number;
  /** Refused: the mock serves plain HTTP. */
  tls?: boolean;
  /** Accepted; no effect. The mock answers no CORS preflight. */
  cors?: boolean;
  /** How the engine is reached, as the Janus SDK's own `engine` option. */
  engine?: SubprocessOptions | (() => FramePipe);
}
export type PactV3Options = PactV4Options;

export interface V3MockServer {
  port: number;
  url: string;
  id: string;
}
export type V4MockServer = V3MockServer;
export type TestFunction<T> = (mockServer: V4MockServer) => Promise<T>;

export type JsonMap = { [key: string]: unknown };
export type TemplateHeaders = { [header: string]: string | Matcher<string> | (Matcher<string> | string)[] };
export type TemplateQuery = Record<string, string | Matcher<string | number | boolean> | (string | Matcher<string | number | boolean>)[]>;
export type Path = string | Matcher<string>;
export type Rule = { path?: string; rules: Matcher<unknown>[] };
export type Rules = Partial<Record<"path" | "body" | "header" | "query" | "status", Rule | Rule[]>>;

export interface V3ProviderState {
  description: string;
  parameters?: JsonMap;
}
export interface V3Request {
  method: string;
  path: Path;
  query?: TemplateQuery;
  headers?: TemplateHeaders;
  body?: unknown;
  contentType?: string;
}
export interface V3Response {
  status: number | StatusCodeMatcher<number>;
  headers?: TemplateHeaders;
  body?: unknown;
  contentType?: string;
}
export interface V3Interaction {
  states?: V3ProviderState[];
  uponReceiving: string;
  withRequest: V3Request;
  willRespondWith: V3Response;
}

export type Part = "request" | "response";

/** The calls that name an interaction's description. */
const DESCRIBING = new Set(["upon_receiving", "message_expects_to_receive", "new_async_message", "new_message", "new_sync_message_interaction"]);

/** One interaction's `pact_ffi` calls, in the order pact-js's builder made them. */
export class Calls {
  readonly steps: protocol.PactStep[] = [];
  description = "";
  /** A `PactV4`'s interactions added since its last `executeTest`, this one among them. */
  queue?: Calls[];

  /** Every interaction an `executeTest` of this one serves: this one's queue, as pact-js's mock serves its pact. */
  take(): Calls[] {
    const taken = this.queue?.splice(0) ?? [];
    return taken.includes(this) ? taken : [...taken, this];
  }

  call(call: string, args: Record<string, unknown> = {}): this {
    if (DESCRIBING.has(call)) {
      this.description = String(args.description);
    }
    this.steps.push({ call, ...args });
    return this;
  }

  given(state: string, parameters?: JsonMap): void {
    this.call(parameters ? "given_with_params" : "given", { description: state, ...(parameters ? { params: parameters } : {}) });
  }

  /** A header or query value per index, as pact-js's builders pass a list; one value at index 0. */
  values(call: string, extra: Record<string, unknown>, map: Record<string, unknown> | undefined, list: boolean): void {
    for (const [name, value] of Object.entries(map ?? {})) {
      const values = list && Array.isArray(value) ? value : [value];
      values.forEach((one: unknown, index) => this.call(call, { ...extra, name, index, value: one }));
    }
  }

  file(part: Part, contentType: string, file: string): void {
    this.call("with_binary_file", { part, content_type: contentType, body: read(file).toString("base64") });
  }

  multipart(part: Part, contentType: string, file: string, partName: string, boundary?: string): void {
    const contents = read(file).toString("base64");
    this.call("with_multipart_file_v2", { part, content_type: contentType, file, contents, part_name: partName, boundary });
  }

  rules(part: Part, rules: Rules): void {
    this.call("with_matching_rules", { part, rules: rulesForFfi(rules) });
  }

  status(status: number | StatusCodeMatcher<number>): void {
    this.call("response_status_v2", { status: JSON.stringify(reify<number>(status)) });
    if (isStatusCodeMatcher(status)) {
      this.call("with_matching_rules", {
        part: "response",
        rules: { status: { $: { matchers: [{ match: "statusCode", status: status.status }] } } },
      });
    }
  }
}

function read(file: string): Buffer {
  try {
    return readFileSync(file);
  } catch (e) {
    throw new Error(`unable to read file for binary payload : ${e instanceof Error ? e.message : String(e)}`, { cause: e });
  }
}

/** pact-js's `validateRules` and `convertRulesToFFI`. */
export function rulesForFfi(rules: Rules): Record<string, unknown> {
  if (!rules || typeof rules !== "object" || Array.isArray(rules)) {
    throw new Error("rules must be an object");
  }
  const converted: Record<string, Record<string, unknown>> = {};
  for (const [part, partRules] of Object.entries(rules)) {
    if (!["path", "body", "header", "query", "status"].includes(part)) {
      throw new Error(`Invalid part "${part}" in rules. Valid parts are: path, body, header, query, status`);
    }
    converted[part] = {};
    for (const [index, rule] of (Array.isArray(partRules) ? partRules : [partRules]).entries()) {
      if (!rule || typeof rule !== "object" || !Array.isArray(rule.rules)) {
        throw new Error(`Rule at ${part}[${index}] must have a "rule" property that is an array`);
      }
      if ("path" in rule && typeof rule.path !== "string") {
        throw new Error(`Rule at ${part}[${index}] has a "path" property that is not a string`);
      }
      if (rule.path) {
        converted[part][rule.path] = {
          matchers: rule.rules.map((matcher) => ({
            ...(matcher["pact:matcher:type"] ? { match: matcher["pact:matcher:type"] } : {}),
            ...Object.fromEntries(Object.entries(matcher).filter(([key]) => key !== "pact:matcher:type" && key !== "pact:generator:type")),
          })),
        };
      }
    }
  }
  return converted;
}

const SPECIFICATIONS: Record<number, string> = { 3: "V2", 4: "V3", 5: "V4" };

/** What a test runs against: the mock's endpoint, or the message the engine hands over (ADR 0067). */
export interface Served {
  endpoint?: Record<string, unknown>;
  parts?: protocol.ServeVariantResult["parts"];
}

/** One HTTP `executeTest`, against the mock. */
function execute<T>(opts: PactV4Options, specification: string, interactions: Calls[], testFn: TestFunction<T>): Promise<T | undefined> {
  return run(opts, specification, interactions, "http", ({ endpoint = {} }) =>
    testFn({ port: Number(endpoint.port), url: String(endpoint["base-url"]), id: "unknown" }),
  );
}

/**
 * One test: one engine session, every interaction added and its base variant armed, through
 * `transport` when it names one, as pact-js's mock serves every interaction it was given (ADR 0069
 * decision 5). A message is served with no transport and answered as its parts.
 */
export async function run<T>(
  opts: Pick<PactV4Options, "consumer" | "provider" | "dir" | "spec" | "logLevel" | "host" | "port" | "engine">,
  specification: string,
  interactions: Calls[],
  transport: string | undefined,
  test: (served: Served) => Promise<T>,
): Promise<T | undefined> {
  if (interactions.length === 0) {
    throw new Error("executeTest: no interaction was added to serve");
  }
  const engine = await Engine.start(typeof opts.engine === "function" ? opts.engine() : await SubprocessPipe.start(opts.engine, opts.logLevel));
  try {
    const config = { consumer: { name: opts.consumer }, provider: { name: opts.provider } };
    const { session } = await engine.call(Op.ConsumerSessionCreate, { config });
    const options = opts.host === undefined && opts.port === undefined ? {} : { options: { host: opts.host ?? "127.0.0.1", port: opts.port ?? 0 } };
    const endpoint = transport === undefined ? undefined : (await engine.call(Op.ConsumerSessionStartTransport, { session, transport, ...options })).endpoint;

    const served: { handle: string; variant: string; description: string }[] = [];
    let parts: Served["parts"];
    for (const calls of interactions) {
      const name = `'${calls.description}'`;
      const pact = { specification: SPECIFICATIONS[opts.spec ?? 0] ?? specification, steps: calls.steps };
      const { handle, findings } = await engine.call(Op.ConsumerSessionAddPactInteraction, { session, pact }, name);
      for (const finding of findings ?? []) {
        if (finding.kind === "lossy" && opts.logLevel !== "error") {
          console.warn(`pact-janus: ${name} ${finding.path}: ${finding.message}`);
        }
      }
      // A converted interaction's one variant is its example, `base`, as an upgraded pact's is.
      const { variants } = await engine.call(Op.ConsumerSessionVariants, { session, handle }, name);
      const variant = variants[0]?.id ?? "base";
      ({ parts } = await engine.call(Op.ConsumerSessionServeVariant, { session, handle, variant }));
      if (transport === undefined && parts === undefined) {
        throw new Error(`${name}: the engine handed over no message, so it is older than ADR 0067's`);
      }
      served.push({ handle, variant, description: calls.description });
    }

    let value: T | undefined;
    let failure: { error: unknown } | undefined;
    try {
      value = await test({ endpoint, parts });
    } catch (error) {
      failure = { error };
    }
    const verdict = failure ? { status: "failed", reason: causeText(failure.error) } : { status: "passed" };
    const results: { description: string; result: protocol.VariantResult }[] = [];
    for (const { handle, variant, description } of served) {
      const { result } = await engine.call(Op.ConsumerSessionReportVariant, { session, handle, variant, ...verdict });
      results.push({ description, result });
    }
    const write = { path: resolve(opts.dir ?? "./pacts", `${opts.consumer}-${opts.provider}.janus.json`), mode: "merge" };
    const finalised = await engine.call(Op.ConsumerSessionFinalise, { session, write });

    // What the mock found wanting: an interaction never called, or a request it did not match.
    const wanting = results.filter(({ result }) => result.status === "not-exercised" || (result.mismatches ?? []).length > 0);
    if (failure) {
      // As pact-js: the test's own error, with what the mock found written beside it.
      if (wanting.length > 0) {
        console.error(failureReport(wanting));
      }
      throw failure.error;
    }
    if (results.some(({ result }) => result.status !== "verified")) {
      throw new Error(failureReport(wanting));
    }
    if (finalised.written === undefined) {
      const named = new Map(served.map(({ handle, description }) => [handle, description]));
      throw new ContractWithheldError(finalised.results, named, finalised.refused ?? []);
    }
    return value;
  } finally {
    await engine.close();
  }
}

/** What pact-js says when the mock found the test wanting: each interaction's missing request or mismatches. */
function failureReport(wanting: { description: string; result: protocol.VariantResult }[]): string {
  const said = wanting.flatMap(({ description, result }) =>
    result.status === "not-exercised"
      ? [`Missing request: the interaction '${description}' was expected, but not received`]
      : (result.mismatches ?? []).map((m) => `'${description}': ${m.path === undefined ? "" : `${String(m.path)}: `}${String(m.message ?? JSON.stringify(m))}`),
  );
  return `Test failed for the following reasons:\n\n  Mock server failed with the following mismatches:\n\n${said.map((line, i) => `\t${i}) ${line}`).join("\n")}`;
}

function refuseTls(opts: PactV4Options): void {
  if (opts.tls) {
    throw new Error("tls: the pact-janus mock serves plain HTTP, so a TLS mock server is refused (ADR 0066 decision 6)");
  }
}

// ---------------------------------------------------------------------------------------------
// PactV4

export interface PluginConfig {
  plugin: string;
  version: string;
}
export type Comment = string;
export type CustomComment = { key: string; value: string };

export class PactV4 {
  private readonly opts: PactV4Options;

  constructor(opts: PactV4Options) {
    this.opts = opts;
    refuseTls(opts);
  }

  /** Every interaction added since the last `executeTest`, which the next one serves (ADR 0069 decision 5). */
  readonly #pending: Calls[] = [];

  #queued(): Calls {
    const calls = new Calls();
    calls.queue = this.#pending;
    this.#pending.push(calls);
    return calls;
  }

  addInteraction(): UnconfiguredInteraction {
    return new UnconfiguredInteraction(this.opts, this.#queued());
  }

  addGraphQLInteraction(): UnconfiguredGraphQLInteraction {
    return new UnconfiguredGraphQLInteraction(this.opts, this.#queued());
  }

  addSynchronousInteraction(description: string): UnconfiguredSynchronousMessage {
    return new UnconfiguredSynchronousMessage(this.opts, new Calls().call("new_sync_message_interaction", { description }));
  }

  addAsynchronousInteraction(): UnconfiguredAsynchronousMessage {
    return new UnconfiguredAsynchronousMessage(this.opts, new Calls().call("new_async_message", { description: "" }));
  }
}

export class UnconfiguredInteraction {
  protected readonly opts: PactV4Options;
  protected readonly calls: Calls;

  constructor(opts: PactV4Options, calls: Calls) {
    this.opts = opts;
    this.calls = calls;
  }

  uponReceiving(description: string): this {
    this.calls.call("upon_receiving", { description });
    return this;
  }

  given(state: string, parameters?: JsonMap): this {
    this.calls.given(state, parameters);
    return this;
  }

  /** Accepted; no effect. A contract has no pending interactions: the broker decides (ADR 0051). */
  pending(pending = true): this {
    void pending;
    return this;
  }

  /** Accepted; no effect. A contract records no comments, test names or references. */
  comment(comment: Comment | CustomComment): this {
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

  withCompleteRequest(request: V3Request): never {
    void request;
    throw new Error("withCompleteRequest is not implemented");
  }

  withRequest(method: string, path: Path, builder?: (builder: RequestBuilder) => void): InteractionWithRequest {
    this.calls.call("with_request", { method, path });
    builder?.(new RequestBuilder(this.calls));
    return new InteractionWithRequest(this.opts, this.calls);
  }

  usingPlugin(config: PluginConfig): never {
    throw new Error(
      `usingPlugin: the pact plugin '${config.plugin}' is refused, since pact-janus cannot load pact plugins (ADR 0045, ADR 0066 decision 6)`,
    );
  }
}

/** What a request and a response builder share: the bodies and rules of their part. */
class PartBuilder {
  protected readonly calls: Calls;
  readonly #part: Part;

  constructor(calls: Calls, part: Part) {
    this.calls = calls;
    this.#part = part;
  }

  jsonBody(body: unknown): this {
    this.calls.call("with_body", { part: this.#part, content_type: "application/json", body });
    return this;
  }

  xmlBody(body: unknown): this {
    this.calls.call("with_body", { part: this.#part, content_type: "application/xml", body });
    return this;
  }

  binaryFile(contentType: string, file: string): this {
    this.calls.file(this.#part, contentType, file);
    return this;
  }

  multipartBody(contentType: string, file: string, mimePartName: string, boundary?: string): this {
    this.calls.multipart(this.#part, contentType, file, mimePartName, boundary);
    return this;
  }

  matchingRules(rules: Rules): this {
    this.calls.rules(this.#part, rules);
    return this;
  }

  body(contentType: string, body: Buffer): this {
    this.calls.call("with_binary_file", { part: this.#part, content_type: contentType, body: body.toString("base64") });
    return this;
  }
}

export class RequestBuilder extends PartBuilder {
  constructor(calls: Calls) {
    super(calls, "request");
  }

  query(query: TemplateQuery): this {
    this.calls.values("with_query_parameter", {}, query, true);
    return this;
  }

  headers(headers: TemplateHeaders): this {
    this.calls.values("with_header_v2", { part: "request" }, headers, true);
    return this;
  }
}

export class ResponseBuilder extends PartBuilder {
  constructor(calls: Calls) {
    super(calls, "response");
  }

  /** As pact-js: each header's value at index 0, a list passed whole. */
  headers(headers: TemplateHeaders): this {
    this.calls.values("with_header_v2", { part: "response" }, headers, false);
    return this;
  }
}

export class InteractionWithRequest {
  private readonly opts: PactV4Options;
  private readonly calls: Calls;

  constructor(opts: PactV4Options, calls: Calls) {
    this.opts = opts;
    this.calls = calls;
  }

  willRespondWith(status: number | StatusCodeMatcher<number>, builder?: (builder: ResponseBuilder) => void): InteractionWithResponse {
    this.calls.status(status);
    builder?.(new ResponseBuilder(this.calls));
    return new InteractionWithResponse(this.opts, this.calls);
  }
}

export class InteractionWithResponse {
  private readonly opts: PactV4Options;
  private readonly calls: Calls;

  constructor(opts: PactV4Options, calls: Calls) {
    this.opts = opts;
    this.calls = calls;
  }

  executeTest<T>(testFn: TestFunction<T>): Promise<T | undefined> {
    return execute(this.opts, "V4", this.calls.take(), testFn);
  }
}

// ---------------------------------------------------------------------------------------------
// GraphQL, over the V4 HTTP builders

export interface GraphQLVariables {
  [name: string]: unknown;
}

/** pact-js's `escapeGraphQlQuery`: the query as a pattern, any run of whitespace optional. */
export const escapeGraphQlQuery = (s: string): string => s.replace(/[-[\]/{}()*+?.\\^$|]/g, "\\$&").replace(/\s+/g, "\\s*");

/** A query as pact-js validates one: a string, or a parsed document that carries its source. */
function querySource(query: unknown, type: string): string {
  if (!query) {
    throw new Error(`You must provide a GraphQL ${type}.`);
  }
  if (typeof query === "string") {
    return query;
  }
  const body = (query as { kind?: string; loc?: { source?: { body?: string } } }).loc?.source?.body;
  if ((query as { kind?: string }).kind === "Document" && typeof body === "string") {
    return body;
  }
  throw new Error("You must provide a either a string or parsed GraphQL.");
}

export class UnconfiguredGraphQLInteraction {
  #operation?: string;
  #variables?: GraphQLVariables;

  private readonly opts: PactV4Options;
  private readonly calls: Calls;

  constructor(opts: PactV4Options, calls: Calls) {
    this.opts = opts;
    this.calls = calls;
  }

  withOperation(operation: string): this {
    this.#operation = operation;
    return this;
  }

  withVariables(variables: GraphQLVariables): this {
    this.#variables = variables;
    return this;
  }

  uponReceiving(description: string): this {
    this.calls.call("upon_receiving", { description });
    return this;
  }

  given(state: string, parameters?: JsonMap): this {
    this.calls.given(state, parameters);
    return this;
  }

  withRequest(method: string, path: Path, builder?: (builder: RequestBuilder) => void): GraphQLInteractionWithRequest {
    this.calls.call("with_request", { method, path });
    builder?.(new RequestBuilder(this.calls));
    return new GraphQLInteractionWithRequest(this.opts, this.calls, this.#operation, this.#variables);
  }
}

export class GraphQLInteractionWithRequest {
  private readonly opts: PactV4Options;
  private readonly calls: Calls;
  private readonly operation?: string;
  private readonly variables?: GraphQLVariables;

  constructor(opts: PactV4Options, calls: Calls, operation?: string, variables?: GraphQLVariables) {
    this.opts = opts;
    this.calls = calls;
    this.operation = operation;
    this.variables = variables;
  }

  withQuery(query: unknown): InteractionWithRequest {
    return this.#details(querySource(query, "Query"));
  }

  withMutation(mutation: unknown): InteractionWithRequest {
    return this.#details(querySource(mutation, "Mutation"));
  }

  #details(query: string): InteractionWithRequest {
    const body = { operationName: this.operation, query: regex(escapeGraphQlQuery(query), query), variables: this.variables };
    this.calls.call("with_body", { part: "request", content_type: "application/json", body });
    return new InteractionWithRequest(this.opts, this.calls);
  }
}

// ---------------------------------------------------------------------------------------------
// PactV3

/** Whether a value holds an `undefined` that JSON would silently drop. */
function hasUndefined(value: unknown): boolean {
  return value === undefined || (value !== null && typeof value === "object" && Object.values(value).some(hasUndefined));
}

/** pact-js's `contentTypeFromHeaders`: the request's or response's own `Content-Type`, else the default. */
function contentTypeFromHeaders(headers: TemplateHeaders | undefined, fallback: string): string {
  let found = fallback;
  for (const [name, value] of Object.entries(headers ?? {})) {
    if (name.toLowerCase() === "content-type") {
      const first = Array.isArray(value) ? value[0] : value;
      const example = isMatcher(first) ? first.value : first;
      found = typeof example === "string" ? example : JSON.stringify(example);
    }
  }
  return found;
}

export class PactV3 {
  #states: V3ProviderState[] = [];
  #interactions: Calls[] = [];

  private readonly opts: PactV3Options;

  constructor(opts: PactV3Options) {
    this.opts = opts;
    refuseTls(opts);
  }

  addInteraction(interaction: V3Interaction): this {
    if (interaction.uponReceiving === "") {
      throw new Error("must provide a valid interaction description via 'uponReceiving'");
    }
    (interaction.states ?? []).forEach((s) => this.given(s.description, s.parameters));
    return this.uponReceiving(interaction.uponReceiving).withRequest(interaction.withRequest).willRespondWith(interaction.willRespondWith);
  }

  given(providerState: string, parameters?: JsonMap): this {
    if (parameters && JSON.stringify(parameters) === undefined) {
      throw new Error(`Invalid provider state parameter received. Parameters must not be undefined. Received: ${String(parameters)}`);
    }
    if (parameters && hasUndefined(parameters)) {
      throw new Error(`Invalid provider state parameter received. Parameters must not contain undefined values. Received: ${String(parameters)}`);
    }
    this.#states.push({ description: providerState, parameters });
    return this;
  }

  uponReceiving(description: string): this {
    const calls = new Calls().call("upon_receiving", { description });
    this.#states.forEach((s) => calls.given(s.description, s.parameters));
    this.#interactions.push(calls);
    return this;
  }

  get #current(): Calls {
    const current = this.#interactions.at(-1);
    if (!current) {
      throw new Error("call uponReceiving before describing the request or response");
    }
    return current;
  }

  #request(req: V3Request): void {
    this.#current.call("with_request", { method: req.method, path: req.path });
    this.#current.values("with_header_v2", { part: "request" }, req.headers, true);
    this.#current.values("with_query_parameter", {}, req.query, true);
  }

  #response(res: V3Response): void {
    this.#current.status(res.status);
    this.#current.values("with_header_v2", { part: "response" }, res.headers, true);
  }

  withRequest(req: V3Request): this {
    if (req.body) {
      const content_type = req.contentType || contentTypeFromHeaders(req.headers, "application/json");
      this.#current.call("with_body", { part: "request", content_type, body: req.body });
    }
    this.#request(req);
    return this;
  }

  withRequestBinaryFile(req: V3Request, contentType: string, file: string): this {
    this.#current.file("request", contentType, file);
    this.#request(req);
    return this;
  }

  withRequestMatchingRules(req: V3Request, rules: Rules): this {
    this.#current.rules("request", rules);
    this.#request(req);
    return this;
  }

  /** As pact-js: the response's rules, then the request's details. */
  withResponseMatchingRules(req: V3Request, rules: Rules): this {
    this.#current.rules("response", rules);
    this.#request(req);
    return this;
  }

  withRequestMultipartFileUpload(req: V3Request, contentType: string, file: string, mimePartName: string, boundary?: string): this {
    this.#current.multipart("request", contentType, file, mimePartName, boundary);
    this.#request(req);
    return this;
  }

  willRespondWith(res: V3Response): this {
    this.#response(res);
    if (res.body) {
      const content_type = res.contentType || contentTypeFromHeaders(res.headers, "application/json");
      this.#current.call("with_body", { part: "response", content_type, body: res.body });
    }
    this.#states = [];
    return this;
  }

  withResponseBinaryFile(res: V3Response, contentType: string, file: string): this {
    this.#current.file("response", contentType, file);
    this.#response(res);
    return this;
  }

  withResponseMultipartFileUpload(res: V3Response, contentType: string, file: string, mimePartName: string, boundary?: string): this {
    this.#current.multipart("response", contentType, file, mimePartName, boundary);
    this.#response(res);
    return this;
  }

  executeTest<T>(testFn: (mockServer: V3MockServer) => Promise<T>): Promise<T | undefined> {
    const interactions = this.#interactions;
    this.#interactions = [];
    this.#states = [];
    return execute(this.opts, "V3", interactions, testFn);
  }
}
