// One of two files recording one consumer and provider, which Vitest runs in workers of their own:
// the engine merges both into one contract (ADR 0065).
import { test } from "vitest";
import { integer, json } from "pact-janus";
import { useJanus } from "pact-janus/vitest";

const janus = useJanus({ consumer: "vitest-merged", provider: "orders-api" });

test("gets an order", async () => {
  const getOrder = janus
    .interaction("get an order")
    .request({ method: "GET", path: "/orders/42" })
    .response({ status: 200, body: json({ id: integer(42) }) });
  await janus.execute(getOrder, async (mock) => {
    await fetch(`${mock.url}/orders/42`);
  });
}, 30_000);
