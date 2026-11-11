// live/exchange-events-logged, in TypeScript: the client asks for the wrong order, and the line the SDK
// hands the console says so while the test runs.
import { Janus } from "pact-janus";

export async function aRejectedRequestIsLogged({ dir }: { dir: string }): Promise<void> {
  const janus = new Janus({ consumer: "web-app", provider: "orders-api", contractDir: dir });
  const getOrder = janus.interaction("get an order").request({ method: "GET", path: "/orders/42" }).response({ status: 200 });
  try {
    await janus.execute(getOrder, async (mock) => {
      await fetch(`${mock.url}/orders/7`);
    });
  } finally {
    await janus.finalise().catch(() => undefined);
  }
}
