// pact-js examples/messages, both sides, on pact-janus/pact (ADR 0056 decision 5, ADR 0066, staged
// plan S2.5c). The consumer test is the example's, with its import changed: it is handed the message
// rather than a mock, and records what `janus upgrade` makes of the pact pact-js wrote, which is S1.7's
// fixture messages/pact-js-order-placed. The provider test is the example's, verifying that pact and
// the contract the consumer test recorded.

import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { Matchers, MessageConsumerPact, MessageProviderPact, Pact, providerWithMetadata, synchronousBodyHandler, v4SynchronousBodyHandler } from "../../src/facade/pact.js";
import { contractDir, corpusPact, documents, recorded, upgraded } from "./support.js";

const { string, decimal, integer } = Matchers;

/** examples/messages/consumer.ts. */
interface OrderPlacedEvent {
  orderId: string;
  customerId: string;
  totalAmount: number;
  currency: string;
  items: Array<{ productId: string; quantity: number }>;
}

function handleOrderPlacedEvent(event: OrderPlacedEvent): void {
  if (!event.orderId) throw new Error("orderId is required");
  if (!event.customerId) throw new Error("customerId is required");
  if (event.totalAmount <= 0) throw new Error("totalAmount must be positive");
  if (!event.currency) throw new Error("currency is required");
  if (!event.items?.length) throw new Error("items must not be empty");
}

/** examples/messages/provider.ts. */
function createOrderPlacedEvent(orderId: string): Record<string, unknown> {
  return {
    orderId,
    customerId: "customer-001",
    totalAmount: 149.95,
    currency: "GBP",
    items: [
      { productId: "prod-a", quantity: 1 },
      { productId: "prod-b", quantity: 3 },
    ],
  };
}

const dir = contractDir();
const pact = corpusPact("messages/pact-js-order-placed/pact.json");
const contract = join(dir, "OrderConsumer-OrderProvider.janus.json");

describe("handleOrderPlacedEvent", () => {
  const pact = new Pact({ consumer: "OrderConsumer", provider: "OrderProvider", dir, logLevel: "warn" });

  it("handles a valid OrderPlaced event", () => {
    return pact
      .addAsynchronousInteraction()
      .given("an order has been placed")
      .expectsToReceive("an OrderPlaced event", (builder) => {
        builder
          .withJSONContent({
            orderId: string("order-123"),
            customerId: string("customer-456"),
            totalAmount: decimal(99.99),
            currency: Matchers.regex(/^[A-Z]{3}$/, "USD"),
            items: Matchers.eachLike({
              productId: string("prod-789"),
              quantity: integer(2),
            }),
          })
          .withMetadata({
            queue: "orders.placed",
            contentType: "application/json",
          });
      })
      .executeTest(v4SynchronousBodyHandler(handleOrderPlacedEvent as (body: unknown) => void));
  }, 30_000);
});

describe("the recorded contract", () => {
  it("is what janus upgrade makes of the pact pact-js wrote", async () => {
    const description = "an OrderPlaced event";
    expect(documents(recorded(dir, "OrderConsumer-OrderProvider.janus.json", description))).toEqual(documents(await upgraded(pact, description)));
  }, 30_000);
});

describe("OrderProvider — messages", () => {
  const verify = (pactUrls: string[]) =>
    new MessageProviderPact({
      pactUrls,
      messageProviders: {
        "an OrderPlaced event": providerWithMetadata(() => createOrderPlacedEvent("order-999"), {
          queue: "orders.placed",
          contentType: "application/json",
        }),
      },
      stateHandlers: {
        "an order has been placed": async () => {},
      },
      logLevel: "warn",
    }).verify();

  it("satisfies all OrderConsumer message expectations", async () => {
    expect((await verify([pact])).status).toBe("verified");
  }, 30_000);

  it("satisfies the contract the consumer test recorded", async () => {
    expect((await verify([contract])).status).toBe("verified");
  }, 30_000);
});

// Beyond the example: what a test is handed, and what fails it.
describe("a message test", () => {
  it("fails with its handler's error, and records nothing", async () => {
    const failing = contractDir();
    const pact = new Pact({ consumer: "OrderConsumer", provider: "OrderProvider", dir: failing, logLevel: "error" });
    const run = pact
      .addAsynchronousInteraction()
      .expectsToReceive("an empty order", (builder) => builder.withJSONContent({ orderId: string("order-1"), items: [] }))
      .executeTest(v4SynchronousBodyHandler(handleOrderPlacedEvent as (body: unknown) => void));
    await expect(run).rejects.toThrow("customerId is required");
  }, 30_000);

  it("of a synchronous message is handed the request and the response's octets", async () => {
    const pact = new Pact({ consumer: "Squares", provider: "Geometry", dir: contractDir(), logLevel: "warn" });
    await pact
      .addSynchronousInteraction("a square's area")
      .given("squares exist")
      .withRequest((request) => request.withJSONContent({ side: integer(3) }))
      .withResponse((response) => response.withJSONContent({ area: integer(9) }))
      .executeTest(async (message) => {
        expect(JSON.parse((message.Request.content as Buffer).toString())).toEqual({ side: 3 });
        expect(message.Response.map((r) => JSON.parse((r.content as Buffer).toString()) as unknown)).toEqual([{ area: 9 }]);
      });
  }, 30_000);

  it("on MessageConsumerPact is handed the V3 message", async () => {
    const messages = new MessageConsumerPact({ consumer: "Notes", provider: "Notebook", dir: contractDir(), logLevel: "warn" });
    const handed: unknown[] = [];
    await messages
      .given({ name: "a note exists", params: { id: 1 } })
      .expectsToReceive("a note")
      .withContent({ id: integer(1), text: string("hello") })
      .withMetadata({ topic: "notes" })
      .verify(async (message) => {
        handed.push(message);
      });
    await messages
      .expectsToReceive("a line")
      .withTextContent("hello", "text/plain")
      .verify(synchronousBodyHandler((body) => handed.push(body)));
    expect(handed).toEqual([
      {
        description: "a note",
        providerStates: [{ name: "a note exists", params: { id: 1 } }],
        contents: { id: 1, text: "hello" },
        metadata: { topic: "notes" },
      },
      "hello",
    ]);
  }, 30_000);

  it("given octets is handed them as a Buffer (S2.15)", async () => {
    const messages = new MessageConsumerPact({ consumer: "Blobs", provider: "Store", dir: contractDir(), logLevel: "warn" });
    await messages
      .expectsToReceive("a blob")
      .withBinaryContent(Buffer.from([0, 1, 2]), "application/octet-stream")
      .verify(synchronousBodyHandler((body) => expect(body).toEqual(Buffer.from([0, 1, 2]))));
  }, 30_000);
});
