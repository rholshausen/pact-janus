// A provider test as ES modules, run by Jest with --experimental-vm-modules: the Janus Verifier.
import { createServer } from "node:http";
import { afterAll, beforeAll, expect, test } from "@jest/globals";
import { Verifier } from "pact-janus";

const users = new Map();
let server;

beforeAll(async () => {
  server = createServer((request, response) => {
    const body = request.url === "/users" ? [...users.values()] : users.get(Number(request.url.split("/")[2]));
    response.writeHead(body ? 200 : 404, { "Content-Type": "application/json" }).end(JSON.stringify(body ?? {}));
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
});

afterAll(() => new Promise((resolve) => server.close(resolve)));

test("satisfies UserConsumer's pact", async () => {
  const alice = () => void users.set(1, { id: 1, name: "Alice", email: "alice@example.com" });
  const result = await new Verifier({ providerUrl: `http://127.0.0.1:${server.address().port}`, pacts: ["pact.json"] })
    .state("users exist", alice)
    .state("a user with ID 1 exists", alice)
    .verify();
  expect(result.status).toBe("verified");
}, 30_000);
