// A consumer suite in CommonJS, as a pact-js user's Jest suite is: `useJanus` once per file,
// `execute` in a test, and `finalise` left to the integration's afterAll.
const { expect, test } = require("@jest/globals");
const { integer, json, optional, string } = require("pact-janus");
const { useJanus } = require("pact-janus/jest");

const janus = useJanus({ consumer: "jest-cjs", provider: "orders-api" });

const getOrder = janus
  .interaction("get an order")
  .request({ method: "GET", path: "/orders/42" })
  .response({ status: 200, body: json({ id: integer(42), shippedAt: optional(string("2026-07-30")) }) });

test("gets an order", async () => {
  await janus.execute(getOrder, async (mock) => {
    const order = await (await fetch(`${mock.url}/orders/42`)).json();
    expect(order.id).toBe(42);
  });
}, 30_000);
