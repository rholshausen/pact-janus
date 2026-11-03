// pact-js examples/matchers, both sides, on pact-janus/pact (ADR 0056 decision 5, ADR 0066). The
// consumer test is the example's, with its import changed and its client on fetch; it records what
// `janus upgrade` makes of the pact pact-js wrote. The provider answers with other values than the
// consumer's examples, which the matchers allow.

import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Matchers, Pact, SpecificationVersion, Verifier } from "../../src/facade/pact.js";
import { contractDir, corpusPact, documents, failsOnlyOnMediaType, json, recorded, serve, upgraded, type Provider } from "./support.js";

const { integer, decimal, string, regex, datetime, eachLike, eachKeyMatches, matchStatus, HTTPResponseStatusClass } = Matchers;

type Product = { id: number; name: string; price: number; category: string; createdAt: string; tags: string[] };

/** examples/matchers/consumer.ts, on fetch. */
class ProductApiClient {
  readonly baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  async #get<T>(path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, { headers: { Accept: "application/json" } });
    return (await response.json()) as T;
  }

  getProduct = (id: number) => this.#get<Product>(`/products/${id}`);
  getProducts = () => this.#get<Product[]>("/products");
  getCatalog = () => this.#get<Record<string, { name: string; price: number }>>("/catalog");
}

const dir = contractDir();
const pact = corpusPact("examples/pact-js/matchers/pact.json");
const contract = "MatchersConsumer-MatchersProvider.janus.json";

describe("ProductApiClient — matchers showcase", () => {
  const provider = new Pact({
    consumer: "MatchersConsumer",
    provider: "MatchersProvider",
    spec: SpecificationVersion.SPECIFICATION_VERSION_V4,
    dir,
    logLevel: "error",
  });

  it("fetches a product — like(), integer(), decimal(), string(), regex(), datetime(), eachLike()", async () => {
    await provider
      .addInteraction()
      .given("a product exists")
      .uponReceiving("a GET request for a product")
      .withRequest("GET", "/products/1", (builder) => {
        builder.headers({ Accept: "application/json" });
      })
      .willRespondWith(200, (builder) => {
        builder.headers({ "Content-Type": "application/json" });
        builder.jsonBody({
          id: integer(1),
          name: string("Widget"),
          price: decimal(9.99),
          category: regex(/^[a-z-]+$/, "electronics"),
          createdAt: datetime("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", "2024-01-01T00:00:00.000Z"),
          tags: eachLike("featured"),
        });
      })
      .executeTest(async (mockserver) => {
        const product = await new ProductApiClient(mockserver.url).getProduct(1);
        expect(typeof product.id).toBe("number");
        expect(typeof product.name).toBe("string");
        expect(product.tags.length).toBeGreaterThan(0);
      });
  }, 30_000);

  it("fetches all products — eachLike() on array response", async () => {
    await provider
      .addInteraction()
      .given("products exist")
      .uponReceiving("a GET request for all products")
      .withRequest("GET", "/products", (builder) => {
        builder.headers({ Accept: "application/json" });
      })
      .willRespondWith(200, (builder) => {
        builder.headers({ "Content-Type": "application/json" });
        builder.jsonBody(eachLike({ id: integer(1), name: string("Widget"), price: decimal(9.99) }));
      })
      .executeTest(async (mockserver) => {
        const products = await new ProductApiClient(mockserver.url).getProducts();
        expect(products.length).toBeGreaterThan(0);
        expect(typeof products[0]?.name).toBe("string");
      });
  }, 30_000);

  it("fetches the catalog — eachKeyMatches() for dictionary responses", async () => {
    await provider
      .addInteraction()
      .given("the catalog is populated")
      .uponReceiving("a GET request for the product catalog")
      .withRequest("GET", "/catalog", (builder) => {
        builder.headers({ Accept: "application/json" });
      })
      .willRespondWith(200, (builder) => {
        builder.headers({ "Content-Type": "application/json" });
        builder.jsonBody(
          eachKeyMatches({ "WIDGET-001": { name: string("Widget"), price: decimal(9.99) } }, [regex(/^[A-Z]+-\d{3}$/, "WIDGET-001")]),
        );
      })
      .executeTest(async (mockserver) => {
        const catalog = await new ProductApiClient(mockserver.url).getCatalog();
        expect(Object.keys(catalog).length).toBeGreaterThan(0);
      });
  }, 30_000);

  it("fetches products with flexible status code — matchStatus() for status code matching", async () => {
    await provider
      .addInteraction()
      .given("products exist")
      .uponReceiving("a GET request for all products with flexible status matching")
      .withRequest("GET", "/products", (builder) => {
        builder.headers({ Accept: "application/json" });
      })
      .willRespondWith(matchStatus(200, HTTPResponseStatusClass.Success), (builder) => {
        builder.headers({ "Content-Type": "application/json" });
        builder.jsonBody(eachLike({ id: integer(1), name: string("Widget"), price: decimal(9.99) }));
      })
      .executeTest(async (mockserver) => {
        const products = await new ProductApiClient(mockserver.url).getProducts();
        expect(products.length).toBeGreaterThan(0);
        expect(typeof products[0]?.name).toBe("string");
      });
  }, 30_000);

  it.each([
    "a GET request for a product",
    "a GET request for all products",
    "a GET request for the product catalog",
    "a GET request for all products with flexible status matching",
  ])(
    "records what janus upgrade makes of pact-js's pact: %s",
    async (description) => {
      expect(documents(recorded(dir, contract, description))).toEqual(documents(await upgraded(pact, description)));
    },
    30_000,
  );
});

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

  const verifier = (file: string) =>
    new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [file],
      stateHandlers: {
        "a product exists": async () => {},
        "products exist": async () => {},
        "the catalog is populated": async () => {},
      },
      logLevel: "warn",
    });

  // The status-class interaction checks the status by its class, v4's statusCode matcher, which the
  // built-in HTTP component runs (staged plan S1.35, ADR 0057).
  it("satisfies all MatchersConsumer expectations", async () => {
    const result = await verifier(pact).verifyProvider();
    expect(result.results.map((r) => r.status)).toEqual(["verified", "verified", "verified", "verified"]);
  }, 30_000);

  it("verifies the contract the consumer test recorded, but for S3.4's Content-Type", async () => {
    await failsOnlyOnMediaType(verifier(join(dir, contract)).verifyProvider(), 4);
  }, 30_000);
});
