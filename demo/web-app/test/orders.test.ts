// web-app's contract with order-service: one consumer test, run once per variant the engine selects.

import { expect, it } from "vitest";
import { anyOf, eachLike, integer, json, string, variantCases, whenVariant } from "pact-janus";
import { useJanus } from "pact-janus/vitest";
import { OrderClient, summarise } from "../src/orders.js";

const janus = useJanus({ consumer: "web-app", provider: "order-service", contractDir: "contracts" });

const getOrder = janus
  .interaction("a request for an order")
  // What the provider must set up for each variant: the status it is testing, and as many items.
  .given("an order exists", {
    id: "42",
    status: variantCases("status", { PENDING: "PENDING", SHIPPED: "SHIPPED" }),
    shipped: whenVariant("status", "SHIPPED"),
    items: variantCases("items", { min: 1, "min+1": 2 }),
  })
  .request({ method: "GET", path: "/orders/42", headers: { Accept: "application/json" } })
  .response({
    status: 200,
    body: json({
      id: string("42"),
      status: anyOf("PENDING", "SHIPPED"),
      items: eachLike({ sku: string("sku-0"), quantity: integer(1) }),
    }),
  });

it("shows an order", async () => {
  await janus.execute(getOrder, async (mock) => {
    const order = await new OrderClient(mock.url).getOrder("42");
    expect(summarise(order)).toMatch(/^Order 42 /);
  });
}, 30_000);
