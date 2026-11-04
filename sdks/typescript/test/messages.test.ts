// Message interactions in a consumer test (staged plan S2.14, ADR 0068), against the real engine:
// the closure is handed each variant's message, the contract records it, and the provider's message
// producers verify that contract, each told by a provider-state parameter which variant to produce.

import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { Janus, Verifier, VariantsFailedError, content, integer, optional, string, whenVariant } from "../src/index.js";

/** The consumer's own code: what an order-placed event becomes. */
function label(event: { id: number; note?: string }): string {
  return event.note === undefined ? `order ${event.id}` : `order ${event.id} (${event.note})`;
}

function orderPlaced(janus: Janus) {
  return janus
    .message("an order was placed")
    .given("an order exists", { noted: whenVariant("note", "present") })
    .contents({
      metadata: { queue: "orders.placed" },
      body: { id: integer(42), note: optional(string("fragile")) },
    });
}

describe("a message interaction", () => {
  it("hands each variant its message, and the contract it records is verified by a message producer", async () => {
    const dir = await mkdtemp(join(tmpdir(), "janus-messages-"));
    const janus = new Janus({ consumer: "shipping", provider: "orders", contractDir: dir });
    const labels: string[] = [];
    await janus.execute(orderPlaced(janus), (message) => {
      expect(message.contentType).toBe("application/json");
      expect(message.metadata).toEqual({ queue: "orders.placed" });
      labels.push(label(message.json() as { id: number; note?: string }));
    });
    expect(labels).toEqual(["order 42 (fragile)", "order 42"]);

    // A synchronous message: the request the consumer sends, and the reply it is handed.
    const ask = janus
      .message("a square's area")
      .request({ body: { side: 3 } })
      .contents({ body: { area: integer(9) } });
    await janus.execute(ask, (reply) => {
      expect(reply.request?.json()).toEqual({ side: 3 });
      expect(reply.json()).toEqual({ area: 9 });
    });

    const cancelled = janus.message("an order was cancelled").contents({ body: content("text/plain", "cancelled") });
    await janus.execute(cancelled, (message) => {
      expect(message.text()).toBe("cancelled");
      expect(message.contentType).toBe("text/plain");
    });

    const { contractFile } = await janus.finalise();
    expect(contractFile).toBeDefined();

    const asked: Record<string, unknown>[] = [];
    const result = await new Verifier({ providerUrl: "http://127.0.0.1:1", pacts: [contractFile as string] })
      .state("an order exists", () => undefined)
      .message("an order was placed", (request) => {
        asked.push(request);
        const [state] = request.providerStates as { params?: { noted?: boolean } }[];
        return {
          body: state?.params?.noted ? { id: 7, note: "handle with care" } : { id: 7 },
          contentType: "application/json",
          metadata: { queue: "orders.placed" },
        };
      })
      .message("a square's area", (request) => {
        const side = ((request.request as { contents: { content: { side: number } } }).contents.content).side;
        return { body: { area: side * side }, contentType: "application/json" };
      })
      .message("an order was cancelled", () => ({ body: "cancelled", contentType: "text/plain" }))
      .verify();

    expect(result.status).toBe("verified");
    expect(result.results).toHaveLength(4);
    expect(asked.map((request) => request.providerStates)).toEqual([
      [{ name: "an order exists", params: { noted: true } }],
      [{ name: "an order exists", params: { noted: false } }],
    ]);
  }, 60_000);

  it("fails the variant its closure fails on, naming it", async () => {
    const dir = await mkdtemp(join(tmpdir(), "janus-messages-"));
    const janus = new Janus({ consumer: "shipping", provider: "orders", contractDir: dir });
    const failed = await janus
      .execute(orderPlaced(janus), (message) => {
        const event = message.json() as { note?: string };
        // A careless consumer: it assumes every order has a note.
        if (event.note === undefined) {
          throw new TypeError("note is undefined");
        }
      })
      .catch((error: unknown) => error);
    expect(failed).toBeInstanceOf(VariantsFailedError);
    expect((failed as VariantsFailedError).failures.map((f) => f.variant.id)).toEqual([
      "message.body.note#presence=absent",
    ]);
    await expect(janus.finalise()).rejects.toThrow(/an order was placed/);
  }, 60_000);
});
