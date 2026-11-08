// facade/pact-js-http-interaction: an HTTP interaction built as pact-js's V4 builder builds it, under
// both of its names, and served once to the test.
import assert from "node:assert/strict";
import { HTTPResponseStatusClass, Matchers, Pact, PactV4 } from "pact-janus/pact";

const { like, integer, matchStatus } = Matchers;

export async function servesTheInteractionOnce({ dir }: { dir: string }): Promise<void> {
  const provider = new PactV4({ consumer: "web-app", provider: "orders-api", dir });
  let calls = 0;
  const order = await provider
    .addInteraction()
    .given("an order exists", { id: 1 })
    .uponReceiving("get an order")
    .withRequest("GET", "/orders/1", (builder) => {
      builder.query({ fields: "id,total" });
      builder.headers({ Accept: "application/json" });
    })
    .willRespondWith(matchStatus(200, HTTPResponseStatusClass.Success), (builder) => {
      builder.headers({ "Content-Type": "application/json" });
      builder.jsonBody({ id: integer(1), total: like(9.5) });
    })
    .executeTest(async (mock) => {
      calls += 1;
      const response = await fetch(`${mock.url}/orders/1?fields=id,total`, { headers: { Accept: "application/json" } });
      assert.equal(response.status, 200);
      return response.json();
    });
  assert.equal(calls, 1);
  assert.deepEqual(order, { id: 1, total: 9.5 });
}

export async function recordsUnderTheNamePact({ dir }: { dir: string }): Promise<void> {
  const provider = new Pact({ consumer: "web-app", provider: "orders-api", dir });
  await provider
    .addInteraction()
    .given("no orders exist")
    .uponReceiving("list no orders")
    .withRequest("GET", "/orders")
    .willRespondWith(200, (builder) => {
      builder.jsonBody([]);
    })
    .executeTest(async (mock) => {
      assert.deepEqual(await (await fetch(`${mock.url}/orders`)).json(), []);
    });
}
