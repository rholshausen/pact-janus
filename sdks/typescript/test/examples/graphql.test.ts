// pact-js examples/graphql/provider.test.ts, on pact-janus/verifier (ADR 0056 decision 5). The
// provider answers the one query its schema has, as the example's graphql resolver does.

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Verifier } from "../../src/facade/verifier.js";
import { corpusPact, json, serve, type Provider } from "./support.js";

describe("GraphQLProvider", () => {
  let server: Provider;

  beforeAll(async () => {
    server = await serve((_request, body, response) => {
      const { query } = JSON.parse(body) as { query: string };
      json(response, 200, /\bhello\b/.test(query) ? { data: { hello: "Hello, World!" } } : { errors: [{ message: "unknown query" }] });
    });
  });

  afterAll(() => server.close());

  it("satisfies all GraphQLConsumer expectations", async () => {
    const result = await new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [corpusPact("examples/pact-js/graphql/pact.json")],
      stateHandlers: {
        "the hello service is available": async () => {},
      },
      logLevel: "warn",
    }).verifyProvider();
    expect(result.status).toBe("verified");
  }, 30_000);
});
