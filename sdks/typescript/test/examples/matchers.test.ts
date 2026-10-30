// pact-js examples/matchers/provider.test.ts, on pact-janus/verifier (ADR 0056 decision 5). The
// provider answers with other values than the consumer's examples, which the matchers allow.

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { VerificationFailedError } from "../../src/index.js";
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

  // Today's verifier passes all four. Janus fails the third: its v4 statusCode matcher is not
  // compiled yet, which corpora/verifier/disagreements.json records against S1.35.
  it("satisfies all MatchersConsumer expectations but the status class one, until S1.35", async () => {
    const run = new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [corpusPact("examples/pact-js/matchers/pact.json")],
      stateHandlers: {
        "a product exists": async () => {},
        "products exist": async () => {},
        "the catalog is populated": async () => {},
      },
      logLevel: "warn",
    }).verifyProvider();
    const error = await run.then(
      () => undefined,
      (e: unknown) => e,
    );
    expect(error).toBeInstanceOf(VerificationFailedError);
    const { result } = error as VerificationFailedError;
    expect(result.results.map((r) => r.status)).toEqual(["verified", "verified", "failed", "verified"]);
    expect(JSON.stringify(result.results[2])).toContain("legacy matching rule 'status-code' is not yet supported");
  }, 30_000);
});
