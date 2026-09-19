// The engine as the idiomatic layer sees it: typed operations over a frame pipe. Operation names
// and body/result types all come from the generated bindings (task 6.1); this file only pairs
// them up and turns an error frame into a thrown `JanusError`.

import { type protocol, protocolVocabulary } from "../generated/index.js";
import { JanusError } from "../errors.js";
import type { FramePipe } from "./pipe.js";

const Op = protocolVocabulary.RequestFrameOp;

/** Operation name -> [request body, result]. */
interface Operations {
  [Op.EngineHello]: [protocol.Hello, protocol.HelloResult];
  [Op.ConsumerSessionCreate]: [protocol.Create, protocol.CreateResult];
  [Op.ConsumerSessionAddInteraction]: [protocol.AddInteraction, protocol.AddInteractionResult];
  [Op.ConsumerSessionStartTransport]: [protocol.StartTransport, protocol.StartTransportResult];
  [Op.ConsumerSessionVariants]: [protocol.Variants, protocol.VariantsResult];
  [Op.ConsumerSessionServeVariant]: [protocol.ServeVariant, protocol.ServeVariantResult];
  [Op.ConsumerSessionFinalise]: [protocol.Finalise, protocol.FinaliseResult];
}

/** The only protocol version this SDK speaks (engine-protocol spec §5). */
const PROTOCOL_VERSION = 1;

export const SDK = { name: "pact-janus-typescript", version: "0.0.0" };

export class Engine {
  readonly #pipe: FramePipe;
  #nextId = 0;

  private constructor(pipe: FramePipe) {
    this.#pipe = pipe;
  }

  /** Opens an engine over `pipe`: the handshake, then nothing until the first operation. */
  static async start(pipe: FramePipe): Promise<Engine> {
    const engine = new Engine(pipe);
    try {
      const hello = await engine.call(Op.EngineHello, {
        "protocol-versions": [PROTOCOL_VERSION],
        host: SDK,
        capabilities: {},
      });
      if (hello["protocol-version"] !== PROTOCOL_VERSION) {
        throw new Error(
          `the engine agreed protocol version ${hello["protocol-version"]}; this SDK speaks only ${PROTOCOL_VERSION}`,
        );
      }
    } catch (error) {
      await pipe.close();
      throw error;
    }
    return engine;
  }

  async call<O extends keyof Operations>(op: O, body: Operations[O][0], context?: string): Promise<Operations[O][1]> {
    const response = await this.#pipe.exchange({ type: "request", id: `r-${++this.#nextId}`, op, body });
    if (response.error) {
      throw new JanusError(response.error, context);
    }
    return (response.ok ?? {}) as Operations[O][1];
  }

  close(): Promise<void> {
    return this.#pipe.close();
  }
}
