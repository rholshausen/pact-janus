// The other of two files recording one consumer and provider: see merged-orders.test.ts.
import { test } from "vitest";
import { useJanus } from "pact-janus/vitest";

const janus = useJanus({ consumer: "vitest-merged", provider: "orders-api" });

test("deletes an order", async () => {
  const deleteOrder = janus
    .interaction("delete an order")
    .request({ method: "DELETE", path: "/orders/42" })
    .response({ status: 204 });
  await janus.execute(deleteOrder, async (mock) => {
    await fetch(`${mock.url}/orders/42`, { method: "DELETE" });
  });
}, 30_000);
