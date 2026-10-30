// pact-js examples/matchers/provider.test.ts, on pact-janus/verifier (ADR 0056 decision 5). The
// provider answers with other values than the consumer's examples, which the matchers allow.

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Verifier } from "../../src/facade/verifier.js";
import { corpusPact, json, serve, type Provider } from "./support.js";

describe("MatchersProvider", () => {
  let server: Provider;

  beforeAll(async () => {
    server = await serve((request, _body, response) => {
      const path = request.url ?? "/";
      if (path.startsWith("/products/")) {
        json(response, 200, {
          id: 99,
          name: "Gadget Pro",
          price: 14.5,
          category: "gadgets",
          createdAt: "2025-06-15T12:00:00.000Z",
          tags: ["featured", "sale"],
        });
      } else if (path === "/products") {
        json(response, 200, [
          { id: 10, name: "Alpha", price: 1.5 },
          { id: 11, name: "Beta", price: 2.5 },
          { id: 12, name: "Gamma", price: 3.5 },
        ]);
      } else {
        json(response, 200, {
          "ALPHA-001": { name: "Alpha Device", price: 99.99 },
          "BETA-002": { name: "Beta Gadget", price: 149.99 },
        });
      }
    });
  });

  afterAll(() => server.close());

  // The third interaction checks the status by its class, v4's statusCode matcher, which the
  // built-in HTTP component runs (staged plan S1.35, ADR 0057).
  it("satisfies all MatchersConsumer expectations", async () => {
    const result = await new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [corpusPact("examples/pact-js/matchers/pact.json")],
      stateHandlers: {
        "a product exists": async () => {},
        "products exist": async () => {},
        "the catalog is populated": async () => {},
      },
      logLevel: "warn",
    }).verifyProvider();
    expect(result.results.map((r) => r.status)).toEqual(["verified", "verified", "verified", "verified"]);
  }, 30_000);
});
