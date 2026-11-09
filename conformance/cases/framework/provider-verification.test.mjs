// framework/provider-verifies-in-a-test, under Jest as an ES module: the provider is started for the
// file, and one test verifies it, the Verifier serving its state handler for the run.
import { createServer } from "node:http";
import { afterAll, beforeAll, expect, test } from "@jest/globals";
import { Verifier } from "pact-janus";

const orders = new Map();
const server = createServer((request, response) => {
  const order = orders.get(Number(request.url.split("/")[2]));
  response.writeHead(order ? 200 : 404, { "Content-Type": "application/json" }).end(JSON.stringify(order ?? {}));
});

beforeAll(() => new Promise((resolve) => server.listen(0, "127.0.0.1", resolve)));

afterAll(() => new Promise((resolve) => server.close(resolve)));

test("verifies the orders API", async () => {
  const result = await new Verifier({
    providerUrl: `http://127.0.0.1:${server.address().port}`,
    pacts: ["pacts/web-app-orders-api.json"],
  })
    .state("an order exists", ({ id }) => void orders.set(id, { id, status: "shipped" }))
    .verify();
  expect(result.status).toBe("verified");
}, 30_000);
