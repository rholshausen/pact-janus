// facade/pact-js-async-messages: an asynchronous message built as pact-js's V4 builder builds it,
// handed to the test with no mock, its contents as JSON, as octets, and through a body handler.
import assert from "node:assert/strict";
import { Matchers, Pact, v4AsynchronousBodyHandler, v4SynchronousBodyHandler } from "pact-janus/pact";

const { like, integer } = Matchers;
const options = (dir: string) => ({ consumer: "web-app", provider: "orders-api", dir });

export async function jsonContents({ dir }: { dir: string }): Promise<void> {
  const handled = await new Pact(options(dir))
    .addAsynchronousInteraction()
    .given("an order was placed", { id: 1 })
    .expectsToReceive("an order placed event", (builder) => {
      builder
        .withJSONContent({ id: integer(1), sku: "A-1" })
        .withMetadata({ topic: "orders", contentType: "application/json; charset=utf-8" })
        .withMatchingRules({ body: { path: "$.sku", rules: [like("A-1")] } });
    })
    .executeTest(async (message) => {
      assert.deepEqual(message.contents.content, { id: 1, sku: "A-1" });
      assert.equal(message.metadata.topic, "orders");
      return "handled";
    });
  assert.equal(handled, "handled");
}

export async function octetContents({ dir }: { dir: string }): Promise<void> {
  await new Pact(options(dir))
    .addAsynchronousInteraction()
    .expectsToReceive("an order image event", (builder) => {
      builder.withContent("image/png", Buffer.from([0x89, 0x50, 0x4e, 0x47]));
    })
    .executeTest(async (message) => {
      assert.ok(Buffer.isBuffer(message.contents.content));
      assert.deepEqual([...(message.contents.content as Buffer)], [0x89, 0x50, 0x4e, 0x47]);
    });
}

export async function bodyHandlers({ dir }: { dir: string }): Promise<void> {
  const seen: unknown[] = [];
  for (const [description, handler] of [
    ["an order shipped event", v4AsynchronousBodyHandler(async (body) => seen.push(body))],
    ["an order delivered event", v4SynchronousBodyHandler((body) => seen.push(body))],
  ] as const) {
    await new Pact(options(dir))
      .addAsynchronousInteraction()
      .expectsToReceive(description, (builder) => {
        builder.withJSONContent({ event: description });
      })
      .executeTest(handler);
  }
  assert.deepEqual(seen, [{ event: "an order shipped event" }, { event: "an order delivered event" }]);
}
