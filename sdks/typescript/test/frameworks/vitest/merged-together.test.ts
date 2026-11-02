// Both interactions of merged-orders.test.ts and merged-deletes.test.ts, recorded in one file into
// a directory of its own: the merged contract must be these bytes.
import { test } from "vitest";
import { integer, json } from "pact-janus";
import { useJanus } from "pact-janus/vitest";

const janus = useJanus({ consumer: "vitest-merged", provider: "orders-api", contractDir: "contracts/together" });

test("deletes an order", async () => {
  const deleteOrder = janus
    .interaction("delete an order")
    .request({ method: "DELETE", path: "/orders/42" })
    .response({ status: 204 });
  await janus.execute(deleteOrder, async (mock) => {
    await fetch(`${mock.url}/orders/42`, { method: "DELETE" });
  });
}, 30_000);

test("gets an order", async () => {
  const getOrder = janus
    .interaction("get an order")
    .request({ method: "GET", path: "/orders/42" })
    .response({ status: 200, body: json({ id: integer(42) }) });
  await janus.execute(getOrder, async (mock) => {
    await fetch(`${mock.url}/orders/42`);
  });
}, 30_000);
