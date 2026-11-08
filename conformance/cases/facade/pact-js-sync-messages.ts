// facade/pact-js-sync-messages: a synchronous message's response metadata is accepted and has no
// effect, as pact_ffi writes no synchronous message's metadata.
import assert from "node:assert/strict";
import { Pact } from "pact-janus/pact";

export async function responseMetadataAccepted({ dir }: { dir: string }): Promise<void> {
  await new Pact({ consumer: "web-app", provider: "orders-api", dir })
    .addSynchronousInteraction("ask for an order's status")
    .given("an order exists")
    .withRequest((builder) => {
      builder.withJSONContent({ id: 1 });
    })
    .withResponse((builder) => {
      builder.withJSONContent({ id: 1, status: "shipped" }).withMetadata({ queue: "statuses" });
    })
    .executeTest(async (message) => {
      // As pact-js hands it: each half's octets.
      assert.deepEqual(JSON.parse(String(message.Request.content)), { id: 1 });
      assert.deepEqual(JSON.parse(String(message.Response[0]?.content)), { id: 1, status: "shipped" });
    });
}
