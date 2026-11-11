// The message builder and what a message test is handed (behavioural spec 'message',
// 'message-contents', 'message-request', and 'execute' for a message; ADR 0068). Building makes no
// protocol call; reading a delivered message decodes nothing but its octets.

import type { contract, protocol, shape } from "./generated/index.js";
import { type ExcludedPoints, type Exclusion, exclusion, policyMember, providerState } from "./interaction.js";
import { compile, Content, type Template } from "./shapes.js";

/** A message's slots: its metadata, name by name, and its body. */
export interface MessageSlots {
  metadata?: { readonly [name: string]: Template };
  body?: Template | Content;
}

export class MessageBuilder {
  readonly #description: string;
  readonly #states: contract.State[] = [];
  readonly #exclusions: Exclusion[] = [];
  readonly #parts = new Map<string, Record<string, shape.Shape>>();
  /** Part -> the body's media type: `content`'s, else JSON (ADR 0068 decision 2). */
  readonly #bodyTypes = new Map<string, string>();

  constructor(description: string) {
    this.#description = description;
  }

  get description(): string {
    return this.#description;
  }

  /** As an interaction's `given`. */
  given(name: string, params?: { readonly [k: string]: unknown }): this {
    this.#states.push(providerState(name, params));
    return this;
  }

  /** As an interaction's `exclude`. */
  exclude(when: ExcludedPoints, reason: string): this {
    this.#exclusions.push(exclusion(when, reason));
    return this;
  }

  /** `message-contents`: the message delivered, `parts.message`. */
  contents(slots: MessageSlots): this {
    return this.#declare("message", slots);
  }

  /** `message-request`: the request a synchronous message answers, `parts.request`. */
  request(slots: MessageSlots): this {
    return this.#declare("request", slots);
  }

  #declare(part: string, slots: MessageSlots): this {
    const compiled: Record<string, shape.Shape> = {};
    this.#bodyTypes.delete(part);
    if (slots.metadata !== undefined) {
      // A `content` inside it is refused by `compile`: a metadata member is not a slot.
      compiled.metadata = compile(slots.metadata);
    }
    if (slots.body !== undefined) {
      const content = slots.body instanceof Content ? slots.body : undefined;
      compiled.body = compile(content ? content.document : (slots.body as Template));
      this.#bodyTypes.set(part, content ? content.mediaType : "application/json");
    }
    this.#parts.set(part, compiled);
    return this;
  }

  /** The interaction-spec document this chain describes. */
  build(): contract.InteractionSpec {
    const parts: contract.InteractionSpec["parts"] = {};
    for (const part of ["request", "message"]) {
      const slots = this.#parts.get(part);
      if (slots) {
        parts[part] = { ...slots };
      }
    }
    return {
      description: this.#description,
      transport: { kind: "message", mode: "emissive" },
      ...(this.#states.length > 0 ? { states: this.#states.map((s) => ({ ...s })) } : {}),
      parts,
      ...(this.#bodyTypes.size > 0
        ? { "content-types": Object.fromEntries([...this.#bodyTypes].map(([part, type]) => [part, { body: type }])) }
        : {}),
      ...policyMember(this.#exclusions),
    };
  }
}

/** One slot of a delivered message, as the engine answers it (engine-protocol spec §8.2). */
type Slot = protocol.DeliveredSlotValue;

/** The message a message test is handed: what the engine delivered for the variant (ADR 0068 decision 3). */
export class ReceivedMessage {
  /** The body's octets; empty when the message has none. */
  readonly body: Uint8Array;
  /** The body's media type, when it is labelled with one. */
  readonly contentType: string | undefined;
  /** The message's metadata; empty when it has none. */
  readonly metadata: Readonly<Record<string, unknown>>;
  /** A synchronous message's request: the message the consumer sends. */
  readonly request: ReceivedMessage | undefined;

  /** @internal Read from `serve-variant`'s parts by `execute`. */
  constructor(part: Record<string, Slot> | undefined, request?: ReceivedMessage) {
    const body = part?.body;
    this.body = body === undefined ? new Uint8Array() : octets(body);
    this.contentType = body?.["content-type"];
    const metadata = part?.metadata?.content;
    this.metadata = metadata !== null && typeof metadata === "object" ? { ...(metadata as Record<string, unknown>) } : {};
    this.request = request;
  }

  /** The body as UTF-8 text. */
  text(): string {
    return new TextDecoder().decode(this.body);
  }

  /** The body read as JSON. */
  json(): unknown {
    return JSON.parse(this.text());
  }
}

/** A slot's octets: base64 decoded, text as its UTF-8 encoding, any other value as its JSON text. */
function octets(slot: Slot): Uint8Array {
  if (slot.encoded === "base64" && typeof slot.content === "string") {
    return new Uint8Array(Buffer.from(slot.content, "base64"));
  }
  const text = slot.encoded === "text" && typeof slot.content === "string" ? slot.content : JSON.stringify(slot.content);
  return new TextEncoder().encode(text);
}

/** The message `serve-variant` answered, with a synchronous message's request. */
export function received(parts: NonNullable<protocol.ServeVariantResult["parts"]>): ReceivedMessage {
  const request = parts.request === undefined ? undefined : new ReceivedMessage(parts.request);
  return new ReceivedMessage(parts.message, request);
}
