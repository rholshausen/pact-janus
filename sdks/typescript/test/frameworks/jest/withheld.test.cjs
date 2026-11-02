// A suite whose contract the engine withholds: one test's client crashes on the variant without
// `shippedAt`, so that test fails, the other passes, and finalise fails the suite.
const { test } = require("@jest/globals");
const { integer, json, optional, string } = require("pact-janus");
const { useJanus } = require("pact-janus/jest");

const janus = useJanus({ consumer: "jest-cjs-careless", provider: "orders-api" });

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
    order.shippedAt.slice(0, 4);
  });
}, 30_000);

test("deletes an order", async () => {
  await janus.execute(deleteOrder, async (mock) => {
    await fetch(`${mock.url}/orders/42`, { method: "DELETE" });
  });
}, 30_000);
