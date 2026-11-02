// A provider test under Vitest: the Janus Verifier.
import { createServer } from "node:http";
import type { AddressInfo } from "node:net";
import { afterAll, beforeAll, expect, test } from "vitest";
import { Verifier } from "pact-janus";

type User = { id: number; name: string; email: string };
const users = new Map<number, User>();
const server = createServer((request, response) => {
  const path = request.url ?? "/";
  const body = path === "/users" ? [...users.values()] : users.get(Number(path.split("/")[2]));
  response.writeHead(body ? 200 : 404, { "Content-Type": "application/json" }).end(JSON.stringify(body ?? {}));
});

beforeAll(() => new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve)));

afterAll(() => new Promise<void>((resolve) => server.close(() => resolve())));

test("satisfies UserConsumer's pact", async () => {
  const alice = () => void users.set(1, { id: 1, name: "Alice", email: "alice@example.com" });
  const { port } = server.address() as AddressInfo;
  const result = await new Verifier({ providerUrl: `http://127.0.0.1:${port}`, pacts: ["pact.json"] })
    .state("users exist", alice)
    .state("a user with ID 1 exists", alice)
    .verify();
  expect(result.status).toBe("verified");
}, 30_000);
