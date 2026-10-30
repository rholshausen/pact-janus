// pact-js examples/messages/provider.test.ts, on pact-janus/verifier (ADR 0056 decision 5). Its pact
// is S1.7's fixture messages/pact-js-order-placed, which the example's consumer writes.

import { describe, expect, it } from "vitest";
import { MessageProviderPact, providerWithMetadata } from "../../src/facade/verifier.js";
import { corpusPact } from "./support.js";

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

describe("OrderProvider — messages", () => {
  it("satisfies all OrderConsumer message expectations", async () => {
    const p = new MessageProviderPact({
      pactUrls: [corpusPact("messages/pact-js-order-placed/pact.json")],
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
    });
    const result = await p.verify();
    expect(result.status).toBe("verified");
  }, 30_000);
});
