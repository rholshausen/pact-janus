// pact-js examples/provider-state/provider.test.ts, on pact-janus/verifier (ADR 0056 decision 5): a
// parameterised state whose handler's returned value feeds the pact's ProviderState generator.

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Verifier } from "../../src/facade/verifier.js";
import { corpusPact, json, serve, type Provider } from "./support.js";

type Account = { id: number; accountNumber: string; ownerName: string; balance: number };
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

  it("satisfies all AccountConsumer expectations", async () => {
    const result = await new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [corpusPact("examples/pact-js/provider-state/pact.json")],
      stateHandlers: {
        "an account with number {accountNumber} exists": async (parameters: Record<string, unknown> = {}) => {
          clearAccounts();
          const accountNumber = String(parameters.accountNumber ?? "");
          const id = seedAccount(accountNumber, "Jane Smith");
          return { accountId: id };
        },
      },
      logLevel: "warn",
    }).verifyProvider();
    expect(result.status).toBe("verified");
  }, 30_000);
});
