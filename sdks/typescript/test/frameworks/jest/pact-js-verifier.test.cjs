// pact-js's provider test, in CommonJS as most of them are, run by Jest: its Verifier, from pact-janus/verifier.
const { createServer } = require("node:http");
const { afterAll, beforeAll, expect, test } = require("@jest/globals");
const { Verifier } = require("pact-janus/verifier");

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
  const result = await new Verifier({
    providerBaseUrl: `http://127.0.0.1:${server.address().port}`,
    pactUrls: ["pact.json"],
    stateHandlers: {
      "users exist": async () => void users.set(1, { id: 1, name: "Alice", email: "alice@example.com" }),
      "a user with ID 1 exists": async () => void users.set(1, { id: 1, name: "Alice", email: "alice@example.com" }),
    },
  }).verifyProvider();
  expect(result.status).toBe("verified");
}, 30_000);
