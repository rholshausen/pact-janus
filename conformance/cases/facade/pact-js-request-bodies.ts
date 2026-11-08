// facade/pact-js-request-bodies: each of the V4 request builder's bodies and its matching rules, sent
// by the test as the mock expects them.
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Matchers, Pact, XmlBuilder } from "pact-janus/pact";

const { like, integer, string } = Matchers;
const options = (dir: string) => ({ consumer: "web-app", provider: "orders-api", dir });

async function created(url: string, init: RequestInit): Promise<void> {
  const response = await fetch(url, { method: "POST", ...init });
  assert.equal(response.status, 201, await response.text());
}

export async function jsonBody({ dir }: { dir: string }): Promise<void> {
  await new Pact(options(dir))
    .addInteraction()
    .uponReceiving("create an order from JSON")
    .withRequest("POST", "/orders", (builder) => {
      builder.headers({ "Content-Type": "application/json" });
      builder.jsonBody({ sku: like("A-1"), quantity: integer(2) });
    })
    .willRespondWith(201)
    .executeTest((mock) =>
      created(`${mock.url}/orders`, { headers: { "Content-Type": "application/json" }, body: JSON.stringify({ sku: "B-7", quantity: 5 }) }),
    );
}

export async function xmlBody({ dir }: { dir: string }): Promise<void> {
  await new Pact(options(dir))
    .addInteraction()
    .uponReceiving("create an order from XML")
    .withRequest("POST", "/orders", (builder) => {
      builder.headers({ "Content-Type": "application/xml" });
      builder.xmlBody(
        new XmlBuilder("1.0", "UTF-8", "order").build((order) => {
          order.appendElement("sku", new Map(), string("A-1"));
        }),
      );
    })
    .willRespondWith(201)
    .executeTest((mock) =>
      created(`${mock.url}/orders`, { headers: { "Content-Type": "application/xml" }, body: "<?xml version='1.0'?><order><sku>C-3</sku></order>" }),
    );
}

export async function binaryFile({ dir }: { dir: string }): Promise<void> {
  const file = join(await mkdtemp(join(tmpdir(), "pact-js-binary-")), "receipt.pdf");
  await writeFile(file, Buffer.from("%PDF-1.4 a receipt"));
  await new Pact(options(dir))
    .addInteraction()
    .uponReceiving("upload a receipt file")
    .withRequest("POST", "/receipts", (builder) => {
      builder.binaryFile("application/pdf", file);
    })
    .willRespondWith(201)
    .executeTest((mock) =>
      created(`${mock.url}/receipts`, { headers: { "Content-Type": "application/pdf" }, body: Buffer.from("%PDF-1.4 a receipt") }),
    );
}

export async function body({ dir }: { dir: string }): Promise<void> {
  await new Pact(options(dir))
    .addInteraction()
    .uponReceiving("create an order from text")
    .withRequest("POST", "/orders", (builder) => {
      builder.body("text/plain", Buffer.from("sku=A-1"));
    })
    .willRespondWith(201)
    .executeTest((mock) => created(`${mock.url}/orders`, { headers: { "Content-Type": "text/plain" }, body: "sku=A-1" }));
}

export async function matchingRules({ dir }: { dir: string }): Promise<void> {
  await new Pact(options(dir))
    .addInteraction()
    .uponReceiving("create an order with rules")
    .withRequest("POST", "/orders", (builder) => {
      builder.headers({ "Content-Type": "application/json" });
      builder.jsonBody({ sku: "A-1" });
      builder.matchingRules({ body: { path: "$.sku", rules: [Matchers.regex(/^[A-Z]-\d+$/, "A-1")] } });
    })
    .willRespondWith(201)
    .executeTest((mock) =>
      created(`${mock.url}/orders`, { headers: { "Content-Type": "application/json" }, body: JSON.stringify({ sku: "Z-9" }) }),
    );
}
