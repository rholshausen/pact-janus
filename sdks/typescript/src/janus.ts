// The session primitives (behavioural spec 'janus', 'execute', 'finalise'). Every step here is one
// protocol operation with the caller's values substituted in; the only thing this file decides is
// *when* — one engine and one session per configured object, which is to say per test suite.

import { mkdir, writeFile } from "node:fs/promises";
import { isAbsolute, join, resolve } from "node:path";
import { protocolVocabulary, type protocol } from "./generated/index.js";
import { Engine } from "./engine/engine.js";
import type { FramePipe } from "./engine/pipe.js";
import { SubprocessPipe, type SubprocessOptions } from "./engine/subprocess.js";
import { ContractWithheldError, VariantsFailedError, type VariantFailure } from "./errors.js";
import { InteractionBuilder } from "./interaction.js";

const Op = protocolVocabulary.RequestFrameOp;

export interface JanusConfig {
  consumer: string;
  provider: string;
  /** Where `finalise` writes the contract. Default: `contracts` under the working directory. */
  contractDir?: string;
  /** How the engine is reached: subprocess options, or any frame pipe (a test double, or a later embedding). */
  engine?: SubprocessOptions | (() => FramePipe);
  /**
   * The project's declared components (component-interfaces spec §10.2) — a content handler for a
   * type the engine does not have built in, say. Handed to the engine as they are, with a `file`
   * source's relative path made absolute against the working directory; whether one loads is the
   * engine's answer, at the first `execute`.
   */
  components?: readonly ComponentDeclaration[];
}

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
  /** Handle -> description, so a withheld contract names interactions the way the test did. */
  readonly #descriptions = new Map<string, string>();
  /** Interactions whose `execute` failed this session: their contract is not written. */
  #failedTests: string[] = [];

  constructor(config: JanusConfig) {
    this.#config = config;
  }

  interaction(description: string): InteractionBuilder {
    return new InteractionBuilder(description);
  }

  /** Submits `interaction` and runs `closure` once per variant the engine selects. */
  async execute(interaction: InteractionBuilder, closure: Closure): Promise<void> {
    try {
      await this.#execute(interaction, closure);
    } catch (error) {
      this.#failedTests.push(interaction.description);
      throw error;
    }
  }

  async #execute(interaction: InteractionBuilder, closure: Closure): Promise<void> {
    const engine = await this.#start();
    const spec = interaction.build();
    const session = await this.#openSession(engine);

    const { handle } = await engine.call(
      Op.ConsumerSessionAddInteraction,
      { session, interaction: spec },
      `'${spec.description}'`,
    );
    this.#descriptions.set(handle, spec.description);

    this.#mock ??= mockFor(await engine.call(Op.ConsumerSessionStartTransport, { session, transport: "http" }));
    const mock = this.#mock;

    const { variants } = await engine.call(Op.ConsumerSessionVariants, { session, handle }, `'${spec.description}'`);
    const failures: VariantFailure[] = [];
    for (const variant of variants) {
      await engine.call(Op.ConsumerSessionServeVariant, { session, handle, variant: variant.id });
      try {
        await closure(mock, variant);
      } catch (cause) {
        failures.push({ variant, cause });
      }
    }
    if (failures.length > 0) {
      throw new VariantsFailedError(spec.description, failures, variants.length);
    }
  }

  /**
   * Ends the session and the engine, writing the contract when the engine produced one and no test
   * in the suite failed; throws `ContractWithheldError` otherwise. Safe to call with nothing executed.
   */
  async finalise(): Promise<Finalised> {
    const started = this.#engine;
    this.#engine = undefined;
    if (!started) {
      return { results: [] };
    }
    const engine = await started;
    const session = this.#session;
    const failedTests = this.#failedTests;
    this.#session = undefined;
    this.#mock = undefined;
    this.#failedTests = [];
    try {
      if (session === undefined) {
        return { results: [] };
      }
      const { results, contract } = await engine.call(Op.ConsumerSessionFinalise, { session });
      if (!contract || failedTests.length > 0) {
        throw new ContractWithheldError(results, this.#descriptions, failedTests, !contract);
      }
      return { results, contractFile: await this.#write(contract) };
    } finally {
      await engine.close();
    }
  }

  #start(): Promise<Engine> {
    this.#engine ??= (async () => {
      const option = this.#config.engine;
      return Engine.start(typeof option === "function" ? option() : new SubprocessPipe(option));
    })();
    // A failed start is not cached: the next execute tries again, and reports its own failure.
    this.#engine.catch(() => {
      this.#engine = undefined;
    });
    return this.#engine;
  }

  async #openSession(engine: Engine): Promise<string> {
    this.#session ??= (
      await engine.call(Op.ConsumerSessionCreate, {
        config: {
          consumer: { name: this.#config.consumer },
          provider: { name: this.#config.provider },
          ...(this.#config.components === undefined ? {} : { components: this.#config.components.map(resolved) }),
        },
      })
    ).session;
    return this.#session;
  }

  /** Compact UTF-8 JSON and one LF (contract spec §2.4), members in the order the engine sent them. */
  async #write(contract: Record<string, unknown>): Promise<string> {
    const dir = this.#config.contractDir ?? "contracts";
    await mkdir(dir, { recursive: true });
    const file = join(dir, `${this.#config.consumer}-${this.#config.provider}.janus.json`);
    await writeFile(file, `${JSON.stringify(contract)}\n`, "utf8");
    return file;
  }
}

/**
 * A declaration as the engine receives it (lifecycle-hooks spec §7.1: the loader resolves paths, the
 * engine reads none it was not given absolute). Everything else is passed as written.
 */
function resolved(declaration: ComponentDeclaration): ComponentDeclaration {
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
