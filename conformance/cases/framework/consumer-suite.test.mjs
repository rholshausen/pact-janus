// framework/consumer-suite, under Jest as an ES module: `useJanus` once in the file, `execute` in each
// test, and `finalise` left to the integration's afterAll.
import { expect, test } from "@jest/globals";
import { integer, json, optional, string } from "pact-janus";
import { useJanus } from "pact-janus/jest";

const janus = useJanus({ consumer: "web-app", provider: "orders-api" });

const getOrder = janus
  .interaction("get an order")
  .request({ method: "GET", path: "/orders/42" })
  .response({ status: 200, body: json({ id: integer(42), shippedAt: optional(string("2026-07-30")) }) });

const deleteOrder = janus
  .interaction("delete an order")
  .request({ method: "DELETE", path: "/orders/42" })
  .response({ status: 204 });

test("gets an order", async () => {
  await janus.execute(getOrder, async (mock) => {
    const order = await (await fetch(`${mock.url}/orders/42`)).json();
    expect(order.id).toBe(42);
  });
}, 30_000);

test("deletes an order", async () => {
  await janus.execute(deleteOrder, async (mock) => {
    const response = await fetch(`${mock.url}/orders/42`, { method: "DELETE" });
    expect(response.status).toBe(204);
  });
}, 30_000);
