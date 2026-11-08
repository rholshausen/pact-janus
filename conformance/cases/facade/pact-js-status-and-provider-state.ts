// facade/pact-js-status-and-provider-state: matchStatus with a list of statuses, and fromProviderState,
// whose value a verifier fills from the provider state's values.
import assert from "node:assert/strict";
import { Matchers, Pact } from "pact-janus/pact";

const { matchStatus, fromProviderState, integer } = Matchers;

export async function matchStatusWithAList({ dir }: { dir: string }): Promise<void> {
  await new Pact({ consumer: "web-app", provider: "orders-api", dir })
    .addInteraction()
    .uponReceiving("cancel an order, accepted or done")
    .withRequest("DELETE", "/orders/1")
    .willRespondWith(matchStatus(202, [202, 204]))
    .executeTest(async (mock) => {
      assert.equal((await fetch(`${mock.url}/orders/1`, { method: "DELETE" })).status, 202);
    });
}

export async function fromProviderStateValues({ dir }: { dir: string }): Promise<void> {
  await new Pact({ consumer: "web-app", provider: "orders-api", dir })
    .addInteraction()
    .given("an order exists", { id: 42 })
    .uponReceiving("get the order the state made")
    .withRequest("GET", fromProviderState("/orders/${id}", "/orders/42"))
    .willRespondWith(200, (builder) => {
      builder.jsonBody({ id: fromProviderState("${id}", 42), lines: integer(1) });
    })
    .executeTest(async (mock) => {
      assert.deepEqual(await (await fetch(`${mock.url}/orders/42`)).json(), { id: 42, lines: 1 });
    });
}
