// pact-js's provider API over the Janus `Verifier` (ADR 0056 decision 4; the behavioural spec's
// 'facade' entries for 'verifier', 'state-handler', 'request-filter', 'message-producer', 'verify').
// `import { Verifier } from "pact-janus/verifier"` in place of "@pact-foundation/pact": each option
// becomes the member of a protocol document S1.13 maps today's CLI option to, and each handler a
// handler of the Janus verifier. It decides nothing about a verdict, and imports no test framework,
// so it runs under Jest and Vitest alike.

import { Verifier as JanusVerifier, type Message, type SourceOptions, type VerifierConfig } from "../verifier.js";
import type { VerificationResult } from "../errors.js";
import type { protocol } from "../generated/index.js";

/** A consumer version selector, as the Pact Broker reads it. */
export type ConsumerVersionSelector = Record<string, unknown>;

export type StateFunc = (parameters?: Record<string, unknown>) => unknown;
export type StateHandler = StateFunc | { setup?: StateFunc; teardown?: StateFunc };

/** What a message provider receives: today's request for the message. */
export interface MessageDescriptor {
  description: string;
  providerStates?: { name: string; params?: Record<string, unknown> }[];
  metadata?: Record<string, unknown>;
  content?: unknown;
}

export type MessageProvider = (message: MessageDescriptor) => unknown;

/** The request an Express-style request filter receives: Express's names for its parts. */
export interface FilterRequest {
  method: string;
  path: string;
  url: string;
  headers: Record<string, string>;
  query: Record<string, unknown>;
  body: unknown;
}

/** pact-js's request filter: an Express middleware. It may change only `req.headers`. */
export type RequestFilter = (req: FilterRequest, res: unknown, next: (error?: unknown) => void) => unknown;

export interface VerifierOptions {
  providerBaseUrl: string;
  provider?: string;
  pactUrls?: string[];
  pactBrokerUrl?: string;
  pactBrokerUsername?: string;
  pactBrokerPassword?: string;
  pactBrokerToken?: string;
  consumerVersionTags?: string[];
  consumerVersionSelectors?: ConsumerVersionSelector[];
  providerVersionTags?: string[];
  providerVersionBranch?: string;
  /** @deprecated pact-js's own: use providerVersionBranch. */
  providerBranch?: string;
  enablePending?: boolean;
  includeWipPactsSince?: string;
  failIfNoPactsFound?: boolean;
  publishVerificationResult?: boolean;
  providerVersion?: string;
  buildUrl?: string;
  customProviderHeaders?: Record<string, string> | string[];
  disableSslVerification?: boolean;
  validateSSL?: boolean;
  consumerFilters?: string[];
  stateHandlers?: Record<string, StateHandler>;
  requestFilter?: RequestFilter;
  messageProviders?: Record<string, MessageProvider>;
  /** Accepted; no effect. They tuned pact-js's proxy or its logs, which Janus has no part like. */
  logLevel?: string;
  logFile?: string;
  timeout?: number;
  changeOrigin?: boolean;
  proxyHost?: string;
  /** Refused: no point runs before a state's setup (ADR 0056 decision 4). */
  beforeEach?: () => unknown;
  afterEach?: () => unknown;
  /** Refused: a hook in `config` (verifier.janus.yaml) instead. */
  providerStatesSetupUrl?: string;
  providerStatesSetupTeardown?: boolean;
  providerStatesSetupBody?: boolean;
  transports?: unknown[];
  /** Janus's own: a configuration file, declared components, and how the engine is reached. */
  config?: VerifierConfig["config"];
  components?: VerifierConfig["components"];
  engine?: VerifierConfig["engine"];
}

const REFUSED: Partial<Record<keyof VerifierOptions, string>> = {
  beforeEach: "a hook has no point before a state's setup; set up in a state handler",
  afterEach: "a hook has no point after a state's teardown; tear down in a state handler's teardown",
  providerStatesSetupUrl: "use stateHandlers, or a 'state-setup' hook in verifier.janus.yaml (config)",
  providerStatesSetupTeardown: "use stateHandlers, or a 'state-teardown' hook in verifier.janus.yaml (config)",
  providerStatesSetupBody: "use stateHandlers, or a 'state-setup' hook in verifier.janus.yaml (config)",
  transports: "a message transport is a 'produce-message' hook in verifier.janus.yaml (config)",
};

/** pact-js's `Verifier`: `new Verifier(options).verifyProvider()`. */
export class Verifier {
  readonly #options: VerifierOptions;

  constructor(options: VerifierOptions) {
    this.#options = options;
  }

  /** Verifies the provider. Rejects with `VerificationFailedError` when the run failed. */
  verifyProvider(): Promise<VerificationResult> {
    return Promise.resolve().then(() => verifier(this.#options).verify());
  }
}

/** pact-js's `MessageProviderPact`: messages only, so there is no provider URL to reach. */
export class MessageProviderPact {
  readonly #options: Omit<VerifierOptions, "providerBaseUrl"> & { providerBaseUrl?: string };

  constructor(options: Omit<VerifierOptions, "providerBaseUrl"> & { providerBaseUrl?: string }) {
    this.#options = options;
  }

  verify(): Promise<VerificationResult> {
    // pact-js points its verifier at a proxy that answers only messages; an HTTP interaction here
    // has no provider and fails to connect, as there.
    const providerBaseUrl = this.#options.providerBaseUrl ?? "http://127.0.0.1:1";
    return Promise.resolve().then(() => verifier({ ...this.#options, providerBaseUrl }).verify());
  }
}

const METADATA = Symbol("pact message metadata");

/** pact-js's `providerWithMetadata`: the provider's message, with this metadata. */
export function providerWithMetadata(provider: MessageProvider, metadata: Record<string, unknown>): MessageProvider {
  return async (descriptor) => {
    const produced = await provider(descriptor);
    const inner = isTagged(produced) ? produced : { [METADATA]: {}, message: produced };
    return { [METADATA]: { ...inner[METADATA], ...metadata }, message: inner.message };
  };
}

type Tagged = { [METADATA]: Record<string, unknown>; message: unknown };

function isTagged(value: unknown): value is Tagged {
  return typeof value === "object" && value !== null && METADATA in value;
}

function verifier(options: VerifierOptions): JanusVerifier {
  for (const [option, reason] of Object.entries(REFUSED)) {
    if (options[option as keyof VerifierOptions] !== undefined) {
      throw new TypeError(`pact-janus/verifier does not take '${option}': ${reason}`);
    }
  }
  const janus = new JanusVerifier({
    providerUrl: options.providerBaseUrl,
    ...source(options),
    ...runOptions(options),
    ...transport(options),
    ...(options.config ? { config: options.config } : {}),
    ...(options.components ? { components: options.components } : {}),
    ...(options.engine ? { engine: options.engine } : {}),
  });
  for (const [state, handler] of Object.entries(options.stateHandlers ?? {})) {
    const { setup, teardown } = typeof handler === "function" ? { setup: handler, teardown: undefined } : handler;
    janus.state(state, {
      setup: (params) => setup?.(params),
      ...(teardown ? { teardown: (params: Record<string, unknown>) => teardown(params) } : {}),
    });
  }
  const filter = options.requestFilter;
  if (filter) {
    janus.requestFilter((request) => middleware(filter, request));
  }
  for (const [description, provider] of Object.entries(options.messageProviders ?? {})) {
    janus.message(description, async (request) => message(await provider(descriptor(request))));
  }
  return janus;
}

/** The one source pact-js's options name: pact files, pacts by URL, or a broker's. */
function source(options: VerifierOptions): Pick<VerifierConfig, "pacts" | "urls" | "broker"> {
  const urls = (options.pactUrls ?? []).filter((url) => /^https?:\/\//i.test(url));
  const files = (options.pactUrls ?? []).filter((url) => !urls.includes(url));
  if (urls.length > 0 && files.length > 0) {
    throw new TypeError("pactUrls names both files and URLs: Janus verifies one kind of source per run");
  }
  const auth = brokerAuth(options);
  if (options.pactBrokerUrl && options.pactUrls?.length) {
    throw new TypeError("pactUrls and pactBrokerUrl: Janus verifies one kind of source per run");
  }
  if (urls.length > 0) {
    return { urls: { urls: urls as [string, ...string[]], ...(auth ? { auth } : {}) } };
  }
  if (!options.pactBrokerUrl) {
    return { pacts: files };
  }
  if (!options.provider) {
    throw new TypeError("pactBrokerUrl needs provider: the provider whose pacts to fetch");
  }
  // As S1.13 maps today's CLI: a tag is the latest pact for it, and pending and work-in-progress
  // pacts are asked for only with selectors, which is when today's verifier asks.
  const selectors = [
    ...(options.consumerVersionSelectors ?? []),
    ...(options.consumerVersionTags ?? []).map((tag) => ({ tag, latest: true })),
  ];
  const selecting = selectors.length > 0;
  const branch = options.providerVersionBranch ?? options.providerBranch;
  const broker: SourceOptions = {
    broker: { url: options.pactBrokerUrl, ...(auth ? { auth } : {}) },
    provider: options.provider,
    "consumer-version-selectors": selectors,
    "include-pending": Boolean(options.enablePending) && selecting,
    ...(branch ? { "provider-branch": branch } : {}),
    ...(options.providerVersionTags?.length ? { "provider-tags": options.providerVersionTags } : {}),
    ...(options.includeWipPactsSince && selecting ? { "include-wip-since": options.includeWipPactsSince } : {}),
    ...(options.failIfNoPactsFound === false ? { "require-pacts": false } : {}),
  };
  return { broker };
}

function brokerAuth(options: VerifierOptions): protocol.BrokerAuth | undefined {
  if (options.pactBrokerToken) {
    return { kind: "bearer", token: options.pactBrokerToken };
  }
  if (options.pactBrokerUsername) {
    return { kind: "basic", username: options.pactBrokerUsername, password: options.pactBrokerPassword ?? "" };
  }
  return undefined;
}

function runOptions(options: VerifierOptions): Pick<VerifierConfig, "options"> {
  const run: protocol.VerifyOptions = {};
  if (options.consumerFilters?.length) {
    run.filter = { consumers: options.consumerFilters as [string, ...string[]] };
  }
  if (options.publishVerificationResult) {
    if (!options.providerVersion) {
      throw new TypeError("publishVerificationResult needs providerVersion");
    }
    const branch = options.providerVersionBranch ?? options.providerBranch;
    run.publish = {
      "provider-version": options.providerVersion,
      ...(branch ? { branch } : {}),
      ...(options.providerVersionTags?.length ? { tags: options.providerVersionTags } : {}),
      ...(options.buildUrl ? { "build-url": options.buildUrl } : {}),
    };
  }
  return Object.keys(run).length > 0 ? { options: run } : {};
}

function transport(options: VerifierOptions): Pick<VerifierConfig, "transport"> {
  const given = options.customProviderHeaders ?? {};
  const pairs = Array.isArray(given)
    ? given.map((line) => {
        const at = line.indexOf(":");
        return [line.slice(0, at).trim(), line.slice(at + 1).trim()];
      })
    : Object.entries(given);
  const insecure = options.disableSslVerification === true || options.validateSSL === false;
  const transport: Record<string, unknown> = {
    ...(pairs.length > 0 ? { headers: Object.fromEntries(pairs) } : {}),
    ...(insecure ? { client: { tls: { insecure: true } } } : {}),
  };
  return Object.keys(transport).length > 0 ? { transport } : {};
}

/** Runs an Express middleware over the request's slots, and returns its headers. */
async function middleware(filter: RequestFilter, request: Record<string, unknown>): Promise<Record<string, unknown>> {
  const headers = Object.fromEntries(
    Object.entries((request.headers ?? {}) as Record<string, unknown[]>).map(([name, values]) => [
      name.toLowerCase(),
      values.join(", "),
    ]),
  );
  const query = (request.query ?? {}) as Record<string, unknown[]>;
  const search = new URLSearchParams(Object.entries(query).flatMap(([k, vs]) => vs.map((v) => [k, String(v)]))).toString();
  const path = String(request.path ?? "/");
  const req: FilterRequest = {
    method: String(request.method ?? "GET"),
    path,
    url: search ? `${path}?${search}` : path,
    headers,
    query: Object.fromEntries(Object.entries(query).map(([k, vs]) => [k, vs.length === 1 ? vs[0] : vs])),
    body: request.body,
  };
  const answered = (): never => {
    throw new Error("a request filter that answers the request itself is not supported: call next()");
  };
  const res = { send: answered, json: answered, end: answered, status: answered, sendStatus: answered };
  await new Promise<void>((resolve, reject) => {
    const next = (error?: unknown): void => (error ? reject(error instanceof Error ? error : new Error(String(error))) : resolve());
    Promise.resolve(filter(req, res, next)).catch(reject);
  });
  return { headers: Object.fromEntries(Object.entries(req.headers).map(([name, value]) => [name, [value]])) };
}

function descriptor(request: Record<string, unknown>): MessageDescriptor {
  const inner = (request.request ?? {}) as { contents?: { content?: unknown; encoded?: unknown }; metadata?: Record<string, unknown> };
  const contents = inner.contents;
  const content =
    contents && contents.encoded && typeof contents.content === "string"
      ? Buffer.from(contents.content, "base64")
      : contents?.content;
  return {
    description: String(request.description ?? ""),
    ...(Array.isArray(request.providerStates) ? { providerStates: request.providerStates as MessageDescriptor["providerStates"] } : {}),
    ...(inner.metadata ? { metadata: inner.metadata } : {}),
    ...(content !== undefined ? { content } : {}),
  };
}

/**
 * What pact-js's message proxy sends for a provider's answer: with metadata naming a content type,
 * the message labelled with it; otherwise the message as JSON (Express's `res.json`).
 */
function message(produced: unknown): Message {
  const { [METADATA]: metadata, message: body } = isTagged(produced) ? produced : { [METADATA]: undefined, message: produced };
  const named = metadata?.["content-type"] ?? metadata?.["Content-Type"] ?? metadata?.contentType;
  if (typeof named === "string") {
    const raw = typeof body === "string" || body instanceof Uint8Array;
    return { body: raw ? body : JSON.stringify(body), contentType: named, ...(metadata ? { metadata } : {}) };
  }
  return {
    body: JSON.stringify(body === undefined ? null : body),
    contentType: "application/json",
    ...(metadata ? { metadata } : {}),
  };
}
