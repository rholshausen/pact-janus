// framework/withheld-contract-fails-the-suite, under Jest from CommonJS: one test's client crashes
// on the variant without `shippedAt`, so that test fails, the other passes, and the integration's
// afterAll fails the suite with the contract the engine withheld.
const { expect, test } = require("@jest/globals");
const { integer, json, optional, string } = require("pact-janus");
const { useJanus } = require("pact-janus/jest");

const janus = useJanus({ consumer: "web-app", provider: "orders-api" });

const getOrder = janus
  .interaction("get an order")
  .request({ method: "GET", path: "/orders/42" })
  .response({ status: 200, body: json({ id: integer(42), shippedAt: optional(string("2026-07-30")) }) });

const deleteOrder = janus
  .interaction("delete an order")
  .request({ method: "DELETE", path: "/orders/42" })
  .response({ status: 204 });

test("reads when an order shipped", async () => {
  await janus.execute(getOrder, async (mock) => {
    const order = await (await fetch(`${mock.url}/orders/42`)).json();
    expect(order.shippedAt.slice(0, 4)).toBe("2026");
  });
}, 30_000);

test("deletes an order", async () => {
  await janus.execute(deleteOrder, async (mock) => {
    const response = await fetch(`${mock.url}/orders/42`, { method: "DELETE" });
    expect(response.status).toBe(204);
  });
}, 30_000);
