// facade/pact-js-message-consumer-pact: pact-js's MessageConsumerPact, a V3 message pact's builder,
// and the body handlers it is used with.
import assert from "node:assert/strict";
import { MessageConsumerPact, asynchronousBodyHandler, synchronousBodyHandler } from "pact-janus/pact";

const options = (dir: string) => ({ consumer: "web-app", provider: "orders-api", dir });

export async function jsonContent({ dir }: { dir: string }): Promise<void> {
  const seen: unknown[] = [];
  await new MessageConsumerPact(options(dir))
    .given({ name: "an order was cancelled", params: { id: 3 } })
    .expectsToReceive("an order cancelled message")
    .withContent({ id: 3, reason: "out of stock" })
    .withMetadata({ queue: "orders" })
    .verify(async (message) => {
      seen.push(message);
    });
  assert.deepEqual(seen, [
    {
      description: "an order cancelled message",
      providerStates: [{ name: "an order was cancelled", params: { id: 3 } }],
      contents: { id: 3, reason: "out of stock" },
      metadata: { queue: "orders" },
    },
  ]);
}

export async function textContent({ dir }: { dir: string }): Promise<void> {
  const seen: unknown[] = [];
  await new MessageConsumerPact(options(dir))
    .given("an order was noted")
    .expectsToReceive("an order note message")
    .withTextContent("call before delivery", "text/plain")
    .verify(synchronousBodyHandler((body) => seen.push(body)));
  assert.deepEqual(seen, ["call before delivery"]);
}

export async function binaryContent({ dir }: { dir: string }): Promise<void> {
  const seen: unknown[] = [];
  await new MessageConsumerPact(options(dir))
    .expectsToReceive("an order label message")
    .withBinaryContent(Buffer.from([1, 2, 3]), "application/octet-stream")
    .verify(asynchronousBodyHandler(async (body) => seen.push(body)));
  assert.ok(Buffer.isBuffer(seen[0]));
  assert.deepEqual([...(seen[0] as Buffer)], [1, 2, 3]);
}
