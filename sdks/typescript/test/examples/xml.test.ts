// pact-js examples/xml, both sides, on pact-janus/pact (ADR 0056 decision 5, ADR 0066). The consumer
// test is the example's, with its import changed and its client on fetch, reading the XML with a
// pattern rather than fast-xml-parser; it records what `janus upgrade` makes of the pact pact-js
// wrote. As there, XmlBuilder's attributes are a Map, which pact-js loses, so neither has `id`.

import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Matchers, Pact, SpecificationVersion, Verifier, XmlBuilder } from "../../src/facade/pact.js";
import { contractDir, corpusPact, documents, recorded, serve, upgraded, type Provider } from "./support.js";

const { string } = Matchers;

type Book = { id: number; title: string; author: string };

/** examples/xml/consumer.ts, on fetch: each book's title and author. */
class CatalogueClient {
  readonly baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  async getCatalogue(): Promise<{ books: Omit<Book, "id">[] }> {
    const response = await fetch(`${this.baseUrl}/catalogue`, { headers: { Accept: "application/xml" } });
    const xml = await response.text();
    const books = [...xml.matchAll(/<book[^>]*>\s*<title>(.*?)<\/title>\s*<author>(.*?)<\/author>\s*<\/book>/g)];
    return { books: books.map(([, title, author]) => ({ title: title ?? "", author: author ?? "" })) };
  }
}

const dir = contractDir();
const pact = corpusPact("examples/pact-js/xml/pact.json");
const contract = "CatalogueConsumer-CatalogueService.janus.json";

describe("CatalogueClient", () => {
  const provider = new Pact({
    consumer: "CatalogueConsumer",
    provider: "CatalogueService",
    spec: SpecificationVersion.SPECIFICATION_VERSION_V4,
    dir,
    logLevel: "warn",
  });

  it("fetches the book catalogue as XML", async () => {
    await provider
      .addInteraction()
      .given("the catalogue has books")
      .uponReceiving("a GET request for the catalogue in XML")
      .withRequest("GET", "/catalogue", (builder) => {
        builder.headers({ Accept: "application/xml" });
      })
      .willRespondWith(200, (builder) => {
        builder.headers({ "Content-Type": "application/xml; charset=utf-8" });
        builder.xmlBody(
          new XmlBuilder("1.0", "UTF-8", "catalogue").build((catalogue) => {
            catalogue.eachLike("book", new Map([["id", "1"]]), (book) => {
              book.appendElement("title", new Map(), string("The Pragmatic Programmer"));
              book.appendElement("author", new Map(), string("David Thomas"));
            });
          }),
        );
      })
      .executeTest(async (mockserver) => {
        const catalogue = await new CatalogueClient(mockserver.url).getCatalogue();
        expect(catalogue.books.length).toBeGreaterThan(0);
        expect(typeof catalogue.books[0]?.title).toBe("string");
      });
  }, 30_000);

  it("records what janus upgrade makes of pact-js's pact", async () => {
    const description = "a GET request for the catalogue in XML";
    expect(documents(recorded(dir, contract, description))).toEqual(documents(await upgraded(pact, description)));
  }, 30_000);
});

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

  it.each([
    ["pact-js's pact", pact],
    ["the contract the consumer test recorded", join(dir, contract)],
  ])("satisfies all CatalogueConsumer expectations: %s", async (_name, file) => {
    const result = await new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [file],
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
