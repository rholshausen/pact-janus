// pact-js examples/graphql, both sides, on pact-janus/pact (ADR 0056 decision 5, ADR 0066). The
// consumer test is the example's, with its import changed and its client on fetch; it records what
// `janus upgrade` makes of the pact pact-js wrote. The provider answers the one query its schema has,
// as the example's graphql resolver does, and verifies both.

import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Matchers, Pact, SpecificationVersion, Verifier } from "../../src/facade/pact.js";
import { contractDir, corpusPact, documents, json, recorded, serve, upgraded, type Provider } from "./support.js";

const { like } = Matchers;

/** examples/graphql/consumer.ts, on fetch. */
async function fetchHello(baseUrl: string): Promise<string> {
  const response = await fetch(`${baseUrl}/graphql`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({
      operationName: "HelloQuery",
      query: `
        query HelloQuery {
          hello
        }
      `,
      variables: { name: "World" },
    }),
  });
  return ((await response.json()) as { data: { hello: string } }).data.hello;
}

const dir = contractDir();
const pact = corpusPact("examples/pact-js/graphql/pact.json");
const contract = "GraphQLConsumer-GraphQLProvider.janus.json";

describe("fetchHello — GraphQL consumer", () => {
  const provider = new Pact({
    consumer: "GraphQLConsumer",
    provider: "GraphQLProvider",
    spec: SpecificationVersion.SPECIFICATION_VERSION_V4,
    dir,
    logLevel: "warn",
  });

  it("sends a HelloQuery and receives a greeting", async () => {
    await provider
      .addGraphQLInteraction()
      .given("the hello service is available")
      .uponReceiving("a HelloQuery request")
      .withOperation("HelloQuery")
      .withVariables({ name: "World" })
      .withRequest("POST", "/graphql")
      .withQuery(`
        query HelloQuery {
          hello
        }
      `)
      .willRespondWith(200, (builder) => {
        builder.headers({ "Content-Type": "application/json; charset=utf-8" });
        builder.jsonBody({ data: { hello: like("Hello, World!") } });
      })
      .executeTest(async (mockserver) => {
        expect(await fetchHello(mockserver.url)).toBe("Hello, World!");
      });
  }, 30_000);

  it("records what janus upgrade makes of pact-js's pact", async () => {
    const description = "a HelloQuery request";
    expect(documents(recorded(dir, contract, description))).toEqual(documents(await upgraded(pact, description)));
  }, 30_000);
});

describe("GraphQLProvider", () => {
  let server: Provider;

  beforeAll(async () => {
    server = await serve((_request, body, response) => {
      const { query } = JSON.parse(body) as { query: string };
      json(response, 200, /\bhello\b/.test(query) ? { data: { hello: "Hello, World!" } } : { errors: [{ message: "unknown query" }] });
    });
  });

  afterAll(() => server.close());

  it.each([
    ["pact-js's pact", pact],
    ["the contract the consumer test recorded", join(dir, contract)],
  ])("satisfies all GraphQLConsumer expectations: %s", async (_name, file) => {
    const result = await new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [file],
      stateHandlers: {
        "the hello service is available": async () => {},
      },
      logLevel: "warn",
    }).verifyProvider();
    expect(result.status).toBe("verified");
  }, 30_000);
});
