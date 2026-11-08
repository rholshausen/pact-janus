// facade/pact-js-several-interactions: one executeTest serves every interaction added since the last,
// each as often as the test calls it; and withCompleteRequest throws, as pact-js's does.
import assert from "node:assert/strict";
import { Matchers, Pact } from "pact-janus/pact";

const { integer } = Matchers;

export async function servesEveryInteractionAddedSinceTheLast({ dir }: { dir: string }): Promise<void> {
  const provider = new Pact({ consumer: "web-app", provider: "orders-api", dir });
  for (const id of [1, 2]) {
    provider
      .addInteraction()
      .uponReceiving(`get order ${id}`)
      .withRequest("GET", `/orders/${id}`)
      .willRespondWith(200, (builder) => {
        builder.jsonBody({ id: integer(id) });
      });
  }
  await provider
    .addInteraction()
    .uponReceiving("get order 3")
    .withRequest("GET", "/orders/3")
    .willRespondWith(200, (builder) => {
      builder.jsonBody({ id: integer(3) });
    })
    .executeTest(async (mock) => {
      for (const id of [3, 1, 2, 1]) {
        assert.deepEqual(await (await fetch(`${mock.url}/orders/${id}`)).json(), { id });
      }
    });
}

export async function withCompleteRequestThrows({ dir }: { dir: string }): Promise<void> {
  new Pact({ consumer: "web-app", provider: "orders-api", dir })
    .addInteraction()
    .uponReceiving("a complete request")
    .withCompleteRequest({ method: "GET", path: "/orders" });
}
