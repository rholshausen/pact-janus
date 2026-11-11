// The session primitives (behavioural spec 'janus', 'execute', 'finalise'). Every step here is one
// protocol operation with the caller's values substituted in; the only thing this file decides is
// *when* — one engine and one session per configured object, which is to say per test suite.

import { isAbsolute, resolve } from "node:path";
import { protocolVocabulary, type protocol } from "./generated/index.js";
import { Engine } from "./engine/engine.js";
import type { FramePipe } from "./engine/pipe.js";
import { SubprocessPipe, type SubprocessOptions } from "./engine/subprocess.js";
import { causeText, ContractWithheldError, VariantsFailedError, type VariantFailure } from "./errors.js";
import { sdkLogLevel } from "./engine/release.js";
import { InteractionBuilder } from "./interaction.js";
import { MessageBuilder, received, type ReceivedMessage } from "./message.js";

const Op = protocolVocabulary.RequestFrameOp;

export interface JanusConfig {
  consumer: string;
  provider: string;
  /**
   * Where the engine writes the contract at `finalise`. Default: `contracts` under the working
   * directory. Several test files recording one consumer and provider merge into its one file.
   */
  contractDir?: string;
  /** Replace the contract file rather than merge into it (ADR 0065). Default: merge. */
  overwriteContract?: boolean;
  /** How the engine is reached: subprocess options, or any frame pipe (a test double, or a later embedding). */
  engine?: SubprocessOptions | (() => FramePipe);
  /**
   * How much the engine logs on standard error: `trace`, `debug`, `info`, `warn` or `error`, as
   * pact-js's `logLevel` (ADR 0061). Default: `LOG_LEVEL`, else `warn`. `RUST_LOG` overrides it.
   */
  logLevel?: string;
  /**
   * The project's declared components (component-interfaces spec §10.2) — a content handler for a
   * type the engine does not have built in, say. Handed to the engine as they are, with a `file`
   * source's relative path made absolute against the working directory; whether one loads is the
   * engine's answer, at the first `execute`.
   */
  components?: readonly ComponentDeclaration[];
  /**
   * What the contract records redacted (ADR 0074). The transport already redacts the credentials it
   * declares: HTTP's `authorization`, `proxy-authorization` and `cookie` request headers and the
   * `set-cookie` response header. `add` names more members by path (`request.headers.x-api-key`,
   * `request.body.password`); `keep` names ones to record as they are, a fixed token the provider's
   * test environment accepts. Handed to the engine as written.
   */
  redaction?: Redaction;
}

/** Members a contract records redacted, by path: a part, a slot, then member names. */
export type Redaction = protocol.Redaction;

/** One declared component (component-interfaces `component-config.schema.json`). */
export interface ComponentDeclaration {
  name: string;
  source: { kind: string; reference?: string; digest?: string; [k: string]: unknown };
  grants?: { env?: string[]; fs?: { path: string; access?: string }[]; network?: boolean };
  limits?: { "deadline-ms"?: number; instances?: string };
  [k: string]: unknown;
}

/** The started transport, as the closure sees it. */
export interface Mock {
  /** The HTTP transport's `base-url` — where the consumer's client should point. */
  url: string;
  /** The transport's whole endpoint descriptor (engine-protocol spec §8.2), for anything beyond `url`. */
  endpoint: Record<string, unknown>;
}

/** A selected variant, exactly as `consumer-session/variants` described it (variant semantics spec §5). */
export type Variant = protocol.VariantDescriptor;

export type Closure = (mock: Mock, variant: Variant) => unknown;

/** A message test: handed each variant's message in place of a mock (ADR 0068 decision 3). */
export type MessageClosure = (message: ReceivedMessage, variant: Variant) => unknown;

/** One variant as a test of its own (`variants`): its name, and what running it does. */
export interface VariantTest {
  /** `label [id]`, or the id alone when the label is the id: `base`. */
  name: string;
  /** Serves the variant, runs the closure, reports it; rejects with the closure's error or the engine's verdict. */
  run(): Promise<void>;
}

/** A variant as a test or a log line names it: `label [id]`, or the id alone when the label is the id. */
export function displayName(variant: Variant): string {
  const label = typeof variant.label === "string" ? variant.label : variant.id;
  return label === variant.id ? variant.id : `${label} [${variant.id}]`;
}

/** How often the session's stream is drained while a closure runs (behavioural spec 'execute'). */
const POLL_EVERY_MS = 100;

/** An interaction added to the session: what each of its variants' runs needs. */
interface Begun {
  description: string;
  handle: string;
  session: string;
  variants: Variant[];
  message: boolean;
  mock: Mock | undefined;
}

/** What `finalise` returns when a contract was written. */
export interface Finalised {
  results: protocol.InteractionResult[];
  /** The file the contract was written to; absent when no interaction was executed. */
  contractFile?: string;
}

export class Janus {
  readonly #config: JanusConfig;
  #engine: Promise<Engine> | undefined;
  #session: string | undefined;
  #mock: Mock | undefined;
  /** The session's stream, when the engine answered `create` with one (engine-protocol spec §9.6). */
  #stream: string | undefined;
  /** Handle -> description, so a withheld contract names interactions the way the test did. */
  readonly #descriptions = new Map<string, string>();
  /** Handle -> its selected variants, so an event names the variant it was about. */
  readonly #selected = new Map<string, Variant[]>();

  constructor(config: JanusConfig) {
    this.#config = config;
  }

  interaction(description: string): InteractionBuilder {
    return new InteractionBuilder(description);
  }

  /** Begins a message interaction: a message the provider emits and the consumer receives. */
  message(description: string): MessageBuilder {
    return new MessageBuilder(description);
  }

  /**
   * Submits `interaction` and runs `closure` once per variant the engine selects, reporting each
   * closure's verdict to the engine and failing on each variant the engine did not verify. A message
   * starts no transport, and its closure is handed the variant's message instead of a mock.
   */
  async execute(interaction: InteractionBuilder, closure: Closure): Promise<void>;
  async execute(message: MessageBuilder, closure: MessageClosure): Promise<void>;
  async execute(interaction: InteractionBuilder | MessageBuilder, closure: Closure | MessageClosure): Promise<void> {
    const begun = await this.#begin(interaction);
    const failures: VariantFailure[] = [];
    for (const variant of begun.variants) {
      const failure = await this.#run(begun, variant, closure);
      if (failure) {
        failures.push(failure);
      }
    }
    if (failures.length > 0) {
      throw new VariantsFailedError(begun.description, failures, begun.variants.length);
    }
  }

  /**
   * `execute`, as one test per variant for a framework that registers tests after an `await` (the
   * Vitest and Jest integrations' `variants`): submits the interaction now, so an invalid one fails
   * before any test is registered, and answers each selected variant as a test, in the engine's order.
   * A test rejects with its closure's own error, or with the engine's verdict when the engine did not
   * verify the variant, and the engine withholds the contract.
   */
  async variantTests(interaction: InteractionBuilder, closure: Closure): Promise<VariantTest[]>;
  async variantTests(message: MessageBuilder, closure: MessageClosure): Promise<VariantTest[]>;
  async variantTests(interaction: InteractionBuilder | MessageBuilder, closure: Closure | MessageClosure): Promise<VariantTest[]> {
    const begun = await this.#begin(interaction);
    return begun.variants.map((variant) => ({
      name: displayName(variant),
      run: async () => {
        const failure = await this.#run(begun, variant, closure);
        if (failure) {
          throw failure.cause ?? new VariantsFailedError(begun.description, [failure], 1);
        }
      },
    }));
  }

  /** `create` if no session is open, `add-interaction`, `start-transport` if none is started, `variants`. */
  async #begin(interaction: InteractionBuilder | MessageBuilder): Promise<Begun> {
    const engine = await this.#start();
    const spec = interaction.build();
    const session = await this.#openSession(engine);
    const message = interaction instanceof MessageBuilder;

    const { handle } = await engine.call(
      Op.ConsumerSessionAddInteraction,
      { session, interaction: spec },
      `'${spec.description}'`,
    );
    this.#descriptions.set(handle, spec.description);

    if (!message) {
      this.#mock ??= mockFor(await engine.call(Op.ConsumerSessionStartTransport, { session, transport: "http" }));
    }
    const { variants } = await engine.call(Op.ConsumerSessionVariants, { session, handle }, `'${spec.description}'`);
    this.#selected.set(handle, variants);
    return { description: spec.description, handle, session, variants, message, mock: this.#mock };
  }

  /**
   * `serve-variant`, the closure, then `report-variant` with its verdict, draining the session's stream
   * while the closure runs. The failure, or nothing when the engine verified the variant.
   */
  async #run(begun: Begun, variant: Variant, closure: Closure | MessageClosure): Promise<VariantFailure | undefined> {
    const engine = await this.#start();
    const { session, handle } = begun;
    const served = await engine.call(Op.ConsumerSessionServeVariant, { session, handle, variant: variant.id });
    if (begun.message && served.parts === undefined) {
      throw new Error("the engine served a message with no parts: it predates ADR 0067, which this SDK needs; use the engine this SDK pins");
    }
    const watching = this.#watch(engine);
    let failure: { cause: unknown } | undefined;
    try {
      await (begun.message
        ? (closure as MessageClosure)(received(served.parts ?? {}), variant)
        : (closure as Closure)(begun.mock as Mock, variant));
    } catch (cause) {
      failure = { cause };
    }
    await watching.stop();
    // The test's verdict goes to the engine, which answers with the variant's (ADR 0064).
    const verdict = failure ? { status: "failed", reason: causeText(failure.cause) } : { status: "passed" };
    const { result } = await engine.call(Op.ConsumerSessionReportVariant, { session, handle, variant: variant.id, ...verdict });
    return result.status === "verified" ? undefined : { variant, result, ...failure };
  }

  /**
   * Drains the session's stream every {@link POLL_EVERY_MS} until stopped, and once more then, logging
   * each event (behavioural spec 'execute'). A poll never waits, so the engine is never held; one that
   * fails ends the watch, said at warn, and is not the test's verdict.
   */
  #watch(engine: Engine): { stop(): Promise<void> } {
    const stream = this.#stream;
    if (stream === undefined) {
      return { stop: async () => {} };
    }
    let failed = false;
    let draining = Promise.resolve();
    const drain = () => {
      draining = draining.then(async () => {
        if (failed) {
          return;
        }
        try {
          const { events } = await engine.call(Op.EventsPoll, { streams: [stream], "wait-ms": 0 });
          for (const event of events) {
            this.#tell(event);
          }
        } catch (error) {
          failed = true;
          log("warn", this.#config.logLevel, `could not read the mock's exchanges, so they are not logged: ${causeText(error)}`);
        }
      });
    };
    const timer = setInterval(drain, POLL_EVERY_MS);
    timer.unref?.();
    return {
      stop: async () => {
        clearInterval(timer);
        drain();
        await draining;
      },
    };
  }

  /** One `consumer-session/exchange` event, as a log line (engine-protocol spec §9.6). */
  #tell(event: protocol.Event): void {
    if (event.kind !== protocolVocabulary.EventKind.ConsumerSessionExchange) {
      return;
    }
    const payload = event.payload as { handle?: string; variant?: string; status?: string; mismatches?: { path?: unknown; message?: unknown }[] };
    if (payload.status === "unmatched") {
      log("warn", this.#config.logLevel, "a request arrived with no variant armed for it, and the mock refused it");
      return;
    }
    const description = this.#descriptions.get(payload.handle ?? "") ?? payload.handle;
    const variant = this.#selected.get(payload.handle ?? "")?.find((v) => v.id === payload.variant);
    const name = `'${description}' [${variant ? displayName(variant) : payload.variant}]`;
    const said = {
      matched: "the request matched",
      mismatched: "the request did not match",
      undelivered: "the request matched, but its reply could not be delivered",
    }[payload.status ?? ""] ?? `the request was ${payload.status}`;
    const lines = (payload.mismatches ?? []).map((m) => `\n    ${m.path === undefined ? "" : `${String(m.path)}: `}${String(m.message)}`);
    log(payload.status === "matched" ? "debug" : "warn", this.#config.logLevel, `${name}: ${said}${lines.join("")}`);
  }

  /**
   * Ends the session and the engine, writing the contract when the engine produced one; throws
   * `ContractWithheldError` otherwise. Safe to call with nothing executed.
   */
  async finalise(): Promise<Finalised> {
    const started = this.#engine;
    this.#engine = undefined;
    if (!started) {
      return { results: [] };
    }
    const engine = await started;
    const session = this.#session;
    this.#session = undefined;
    this.#stream = undefined;
    this.#mock = undefined;
    try {
      if (session === undefined) {
        return { results: [] };
      }
      // The engine's host writes the file, merging into it (ADR 0065): the SDK names it, and writes nothing.
      const { consumer, provider, contractDir, overwriteContract } = this.#config;
      const path = resolve(contractDir ?? "contracts", `${consumer}-${provider}.janus.json`);
      const write = { path, mode: overwriteContract ? "overwrite" : "merge" };
      const { results, refused, written, contract, "contract-text": text } = await engine.call(Op.ConsumerSessionFinalise, { session, write });
      if (written === undefined && (contract ?? text) !== undefined) {
        // An engine older than ADR 0065 ignored 'write' and answered the contract: nothing was withheld.
        throw new Error("the engine answered finalise with the contract instead of writing it: it predates 'contract-writing' (ADR 0065), which this SDK needs; use the engine this SDK pins");
      }
      if (written === undefined) {
        throw new ContractWithheldError(results, this.#descriptions, refused ?? []);
      }
      return { results, contractFile: written };
    } finally {
      await engine.close();
    }
  }

  #start(): Promise<Engine> {
    this.#engine ??= (async () => {
      const option = this.#config.engine;
      return Engine.start(
        typeof option === "function" ? option() : await SubprocessPipe.start(option, this.#config.logLevel),
      );
    })();
    // A failed start is not cached: the next execute tries again, and reports its own failure.
    this.#engine.catch(() => {
      this.#engine = undefined;
    });
    return this.#engine;
  }

  async #openSession(engine: Engine): Promise<string> {
    if (this.#session === undefined) {
      // `events`: what the mock makes of each request, said while the test runs (engine-protocol spec §9.6).
      const created = await engine.call(Op.ConsumerSessionCreate, {
        events: true,
        config: {
          consumer: { name: this.#config.consumer },
          provider: { name: this.#config.provider },
          ...(this.#config.components === undefined ? {} : { components: this.#config.components.map(resolved) }),
          ...(this.#config.redaction === undefined ? {} : { redaction: this.#config.redaction }),
        },
      });
      this.#session = created.session;
      this.#stream = created.stream;
    }
    return this.#session;
  }
}

/**
 * A declaration as the engine receives it (lifecycle-hooks spec §7.1: the loader resolves paths, the
 * engine resolves none against a directory of its own). A reference with a root is passed as written,
 * and Node's `isAbsolute` is that rule: a leading separator counts on Windows too, drive or no drive.
 */
export function resolved(declaration: ComponentDeclaration): ComponentDeclaration {
  const reference = declaration.source.reference;
  if (declaration.source.kind !== "file" || reference === undefined || isAbsolute(reference)) {
    return declaration;
  }
  return { ...declaration, source: { ...declaration.source, reference: resolve(reference) } };
}

function mockFor(started: protocol.StartTransportResult): Mock {
  const url = started.endpoint["base-url"];
  if (typeof url !== "string") {
    throw new Error(`the http transport's endpoint has no base-url: ${JSON.stringify(started.endpoint)}`);
  }
  return { url, endpoint: started.endpoint };
}

const SEVERITY: Record<string, number> = { trace: 0, debug: 1, info: 2, warn: 3, error: 4, off: 5 };

/** A line of the SDK's own, at `level`, when the test's `logLevel` (else `LOG_LEVEL`, else `warn`) shows it. */
function log(level: "debug" | "warn", logLevel: string | undefined, text: string): void {
  if ((SEVERITY[level] ?? 0) < (SEVERITY[sdkLogLevel(process.env, logLevel)] ?? 3)) {
    return;
  }
  (level === "warn" ? console.warn : console.debug)(`pact-janus: ${text}`);
}
