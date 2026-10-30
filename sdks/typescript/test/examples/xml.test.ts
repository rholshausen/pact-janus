// pact-js examples/xml/provider.test.ts, on pact-janus/verifier (ADR 0056 decision 5).

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Verifier } from "../../src/facade/verifier.js";
import { corpusPact, serve, type Provider } from "./support.js";

type Book = { id: number; title: string; author: string };
const catalogue: Book[] = [];
function seedBooks(books: Book[]): void {
  catalogue.length = 0;
  catalogue.push(...books);
}

describe("CatalogueService", () => {
  let server: Provider;

  beforeAll(async () => {
    server = await serve((_request, _body, response) => {
      const books = catalogue
        .map((b) => `  <book id="${b.id}"><title>${b.title}</title><author>${b.author}</author></book>`)
        .join("\n");
      const xml = `<?xml version="1.0" encoding="UTF-8"?>\n<catalogue>\n${books}\n</catalogue>`;
      response.writeHead(200, { "Content-Type": "application/xml; charset=utf-8" }).end(xml);
    });
  });

  afterAll(() => server.close());

  it("satisfies all CatalogueConsumer expectations", async () => {
    const result = await new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [corpusPact("examples/pact-js/xml/pact.json")],
      stateHandlers: {
        "the catalogue has books": async () => {
          seedBooks([
            { id: 101, title: "Clean Code", author: "Robert Martin" },
            { id: 102, title: "Refactoring", author: "Martin Fowler" },
          ]);
        },
      },
      logLevel: "warn",
    }).verifyProvider();
    expect(result.status).toBe("verified");
  }, 30_000);
});
