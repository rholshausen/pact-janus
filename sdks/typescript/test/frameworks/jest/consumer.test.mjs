// A consumer suite as ES modules, which Jest runs with --experimental-vm-modules: `useJanus` once
// per file, `execute` in a test, and `finalise` left to the integration's afterAll.
import { expect, test } from "@jest/globals";
import { integer, json, optional, string } from "pact-janus";
import { useJanus } from "pact-janus/jest";

const janus = useJanus({ consumer: "jest-esm", provider: "orders-api" });

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
