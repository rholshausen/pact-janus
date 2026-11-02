// One of two files recording 'create an order' differently: 200 here, 202 in the other. Whichever
// finalises second fails, the engine naming the interaction (ADR 0065).
import { test } from "vitest";
import { useJanus } from "pact-janus/vitest";

const janus = useJanus({ consumer: "vitest-conflicting", provider: "orders-api" });

test("creates an order", async () => {
  const createOrder = janus
    .interaction("create an order")
    .request({ method: "POST", path: "/orders" })
    .response({ status: 200 });
  await janus.execute(createOrder, async (mock) => {
    await fetch(`${mock.url}/orders`, { method: "POST" });
  });
}, 30_000);
