// facade/pact-js-message-provider: pact-js's MessageProviderPact, its message providers asked for
// each message, and providerWithMetadata labelling one.
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { MessageProviderPact, providerWithMetadata } from "pact-janus/pact";

const pact = {
  consumer: { name: "web-app" },
  provider: { name: "orders-api" },
  messages: [
    {
      description: "an order placed event",
      providerStates: [{ name: "an order was placed", params: { id: 1 } }],
      contents: { id: 1, sku: "A-1" },
      metadata: { contentType: "application/json", topic: "orders" },
    },
    {
      description: "an order note",
      contents: "call before delivery",
      metadata: { contentType: "text/plain" },
    },
  ],
  metadata: { pactSpecification: { version: "3.0.0" } },
};

export async function producesEachMessage(): Promise<void> {
  const file = join(await mkdtemp(join(tmpdir(), "pact-js-messages-")), "pact.json");
  await writeFile(file, JSON.stringify(pact));
  const asked: unknown[] = [];
  const result = await new MessageProviderPact({
    pactUrls: [file],
    messageProviders: {
      "an order placed event": providerWithMetadata(
        (descriptor) => {
          asked.push(descriptor);
          return { id: 1, sku: "A-1" };
        },
        { topic: "orders" },
      ),
      "an order note": providerWithMetadata(() => "call before delivery", { contentType: "text/plain" }),
    },
  }).verify();
  assert.equal(result.status, "verified");
  assert.deepEqual(asked, [{ description: "an order placed event", providerStates: [{ name: "an order was placed", params: { id: 1 } }] }]);
}
