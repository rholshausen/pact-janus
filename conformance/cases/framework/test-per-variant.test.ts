// framework/test-per-variant, under Vitest: `await janus.variants` at the top level makes each variant
// the engine selects a test of its own. The client reads shippedAt as if it were always there, so the
// variant without it fails its own test, the base passes, and the withheld contract fails the file.
import { expect } from "vitest";
import { integer, json, optional, string } from "pact-janus";
import { useJanus } from "pact-janus/vitest";

const janus = useJanus({ consumer: "web-app", provider: "orders-api" });

const getOrder = janus
  .interaction("get an order")
  .request({ method: "GET", path: "/orders/42" })
  .response({ status: 200, body: json({ id: integer(42), shippedAt: optional(string("2026-07-30")) }) });

await janus.variants(getOrder, async (mock) => {
  const order = await (await fetch(`${mock.url}/orders/42`)).json();
  expect(order.shippedAt, "shippedAt is missing").toBeDefined();
});
