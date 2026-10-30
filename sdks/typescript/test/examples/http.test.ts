// pact-js examples/http/provider.test.ts, on pact-janus/verifier (ADR 0056 decision 5). The User
// Service is provider.ts's, on node:http; the test is the example's, with its import changed.

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Verifier } from "../../src/facade/verifier.js";
import { corpusPact, json, serve, type Provider } from "./support.js";

type User = { id: number; name: string; email: string };
const users = new Map<number, User>();
const seedUser = (user: User): void => void users.set(user.id, user);
const clearUsers = (): void => users.clear();

describe("UserProvider", () => {
  let server: Provider;

  beforeAll(async () => {
    server = await serve((request, _body, response) => {
      const path = request.url ?? "/";
      if (path === "/users") {
        json(response, 200, [...users.values()]);
        return;
      }
      const user = users.get(Number(path.split("/")[2]));
      if (user) {
        json(response, 200, user);
      } else {
        json(response, 404, { error: "User not found" });
      }
    });
  });

  afterAll(() => server.close());

  it("satisfies all UserConsumer expectations", async () => {
    const result = await new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [corpusPact("examples/pact-js/http/pact.json")],
      stateHandlers: {
        "a user with ID 1 exists": async () => {
          clearUsers();
          seedUser({ id: 1, name: "Alice", email: "alice@example.com" });
        },
        "users exist": async () => {
          clearUsers();
          seedUser({ id: 1, name: "Alice", email: "alice@example.com" });
          seedUser({ id: 2, name: "Bob", email: "bob@example.com" });
        },
      },
      logLevel: "warn",
    }).verifyProvider();
    expect(result.status).toBe("verified");
    expect(result.results.map((r) => r.status)).toEqual(["verified", "verified"]);
  }, 30_000);
});
