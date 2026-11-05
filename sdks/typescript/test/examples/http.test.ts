// pact-js examples/http, both sides, on pact-janus/pact (ADR 0056 decision 5, ADR 0066). The consumer
// test is the example's, with its import changed and its client on fetch rather than axios; it records
// what `janus upgrade` makes of the pact pact-js wrote. The provider test is the example's, on
// node:http, verifying that pact and the contract the consumer test recorded.

import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Matchers, Pact, SpecificationVersion, Verifier } from "../../src/facade/pact.js";
import { contractDir, corpusPact, documents, json, recorded, serve, upgraded, type Provider } from "./support.js";

const { like, integer, eachLike } = Matchers;

type User = { id: number; name: string; email: string };

/** examples/http/consumer.ts, on fetch. */
class UserServiceClient {
  readonly baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  async getUser(id: number): Promise<User> {
    const response = await fetch(`${this.baseUrl}/users/${id}`, { headers: { Accept: "application/json" } });
    return (await response.json()) as User;
  }

  async getUsers(): Promise<User[]> {
    const response = await fetch(`${this.baseUrl}/users`, { headers: { Accept: "application/json" } });
    return (await response.json()) as User[];
  }
}

const dir = contractDir();
const pact = corpusPact("examples/pact-js/http/pact.json");

describe("UserServiceClient", () => {
  const provider = new Pact({
    consumer: "UserConsumer",
    provider: "UserProvider",
    spec: SpecificationVersion.SPECIFICATION_VERSION_V4,
    dir,
    logLevel: "warn",
  });

  it("fetches a user by ID", async () => {
    await provider
      .addInteraction()
      .given("a user with ID 1 exists")
      .uponReceiving("a GET request for user 1")
      .withRequest("GET", "/users/1", (builder) => {
        builder.headers({ Accept: "application/json" });
      })
      .willRespondWith(200, (builder) => {
        builder.headers({ "Content-Type": "application/json" });
        builder.jsonBody(like({ id: integer(1), name: like("Alice"), email: like("alice@example.com") }));
      })
      .executeTest(async (mockserver) => {
        const user = await new UserServiceClient(mockserver.url).getUser(1);
        expect(user.id).toBe(1);
        expect(user.name).toBe("Alice");
        expect(user.email).toBe("alice@example.com");
      });
  }, 30_000);

  it("fetches all users", async () => {
    await provider
      .addInteraction()
      .given("users exist")
      .uponReceiving("a GET request for all users")
      .withRequest("GET", "/users", (builder) => {
        builder.headers({ Accept: "application/json" });
      })
      .willRespondWith(200, (builder) => {
        builder.headers({ "Content-Type": "application/json" });
        builder.jsonBody(eachLike({ id: integer(1), name: like("Alice"), email: like("alice@example.com") }));
      })
      .executeTest(async (mockserver) => {
        const users = await new UserServiceClient(mockserver.url).getUsers();
        expect(users.length).toBeGreaterThan(0);
        expect(users[0]?.id).toBe(1);
      });
  }, 30_000);

  it.each(["a GET request for user 1", "a GET request for all users"])(
    "records what janus upgrade makes of pact-js's pact: %s",
    async (description) => {
      expect(documents(recorded(dir, "UserConsumer-UserProvider.janus.json", description))).toEqual(documents(await upgraded(pact, description)));
    },
    30_000,
  );
});

describe("UserProvider", () => {
  let server: Provider;
  const users = new Map<number, User>();
  const seedUser = (user: User): void => void users.set(user.id, user);
  const clearUsers = (): void => users.clear();

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

  const verifier = (file: string) =>
    new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [file],
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
    });

  it("satisfies all UserConsumer expectations", async () => {
    const result = await verifier(pact).verifyProvider();
    expect(result.status).toBe("verified");
    expect(result.results.map((r) => r.status)).toEqual(["verified", "verified"]);
  }, 30_000);

  it("verifies the contract the consumer test recorded", async () => {
    const result = await verifier(join(dir, "UserConsumer-UserProvider.janus.json")).verifyProvider();
    expect(result.status).toBe("verified");
  }, 30_000);
});
