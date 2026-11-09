// framework/provider-verifies-in-a-test, under Vitest: the provider is started for the file, and one
// test verifies it, the Verifier serving its state handler for the run.
import { createServer } from "node:http";
import type { AddressInfo } from "node:net";
import { afterAll, beforeAll, expect, test } from "vitest";
import { Verifier } from "pact-janus";

type Order = { id: number; status: string };
const orders = new Map<number, Order>();
const server = createServer((request, response) => {
  const order = orders.get(Number((request.url ?? "/").split("/")[2]));
  response.writeHead(order ? 200 : 404, { "Content-Type": "application/json" }).end(JSON.stringify(order ?? {}));
});

beforeAll(() => new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve)));

afterAll(() => new Promise<void>((resolve) => server.close(() => resolve())));

test("verifies the orders API", async () => {
  const { port } = server.address() as AddressInfo;
  const result = await new Verifier({ providerUrl: `http://127.0.0.1:${port}`, pacts: ["pacts/web-app-orders-api.json"] })
    .state("an order exists", (params) => {
      const id = params.id as number;
      orders.set(id, { id, status: "shipped" });
    })
    .verify();
  expect(result.status).toBe("verified");
}, 30_000);
