// pact-js examples/provider-state, both sides, on pact-janus/pact (ADR 0056 decision 5, ADR 0066): a
// PactV3 consumer test with a parameterised state and fromProviderState, which records what `janus
// upgrade` makes of the pact pact-js wrote; and the provider, whose state handler's returned value
// feeds the ProviderState generator.

import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Matchers, PactV3, Verifier } from "../../src/facade/pact.js";
import { contractDir, corpusPact, documents, json, recorded, serve, upgraded, type Provider } from "./support.js";

const { integer, string, fromProviderState } = Matchers;

type Account = { id: number; accountNumber: string; ownerName: string; balance: number };

/** examples/provider-state/consumer.ts, on fetch. */
class AccountServiceClient {
  readonly baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  async getAccountByNumber(accountNumber: string): Promise<Account> {
    const response = await fetch(`${this.baseUrl}/accounts/${accountNumber}`, { headers: { Accept: "application/json" } });
    return (await response.json()) as Account;
  }
}

const dir = contractDir();
const pact = corpusPact("examples/pact-js/provider-state/pact.json");
const contract = "AccountConsumer-AccountService.janus.json";
const description = "a GET request for account ACC-001";

describe("AccountServiceClient", () => {
  const provider = new PactV3({ consumer: "AccountConsumer", provider: "AccountService", dir, logLevel: "warn" });

  it("fetches an account by account number — parameterised state + fromProviderState()", () => {
    provider
      .given("an account with number {accountNumber} exists", { accountNumber: "ACC-001" })
      .uponReceiving(description)
      .withRequest({ method: "GET", path: "/accounts/ACC-001", headers: { Accept: "application/json" } })
      .willRespondWith({
        status: 200,
        headers: { "Content-Type": "application/json" },
        body: {
          id: fromProviderState("${accountId}", 42),
          accountNumber: string("ACC-001"),
          ownerName: string("Jane Smith"),
          balance: integer(1000),
        },
      });

    return provider.executeTest(async (mockserver) => {
      const account = await new AccountServiceClient(mockserver.url).getAccountByNumber("ACC-001");
      expect(typeof account.id).toBe("number");
      expect(account.accountNumber).toBe("ACC-001");
    });
  }, 30_000);

  it("records what janus upgrade makes of pact-js's pact", async () => {
    expect(documents(recorded(dir, contract, description))).toEqual(documents(await upgraded(pact, description)));
  }, 30_000);
});

const accounts = new Map<string, Account>();
let nextId = 1;
function seedAccount(accountNumber: string, ownerName: string): number {
  const id = nextId++;
  accounts.set(accountNumber, { id, accountNumber, ownerName, balance: 1000 });
  return id;
}
function clearAccounts(): void {
  accounts.clear();
  nextId = 1;
}

describe("AccountService", () => {
  let server: Provider;

  beforeAll(async () => {
    server = await serve((request, _body, response) => {
      const account = accounts.get(decodeURIComponent((request.url ?? "").split("/")[2] ?? ""));
      if (account) {
        json(response, 200, account);
      } else {
        json(response, 404, { error: "Account not found" });
      }
    });
  });

  afterAll(() => server.close());

  const verifier = (file: string) =>
    new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [file],
      stateHandlers: {
        "an account with number {accountNumber} exists": async (parameters: Record<string, unknown> = {}) => {
          clearAccounts();
          const accountNumber = String(parameters.accountNumber ?? "");
          const id = seedAccount(accountNumber, "Jane Smith");
          return { accountId: id };
        },
      },
      logLevel: "warn",
    });

  it("satisfies all AccountConsumer expectations", async () => {
    const result = await verifier(pact).verifyProvider();
    expect(result.status).toBe("verified");
  }, 30_000);

  it("verifies the contract the consumer test recorded", async () => {
    const result = await verifier(join(dir, contract)).verifyProvider();
    expect(result.status).toBe("verified");
  }, 30_000);
});
