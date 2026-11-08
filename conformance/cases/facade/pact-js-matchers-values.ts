// facade/pact-js-matchers-values: pact-js's matchers of single values, each in a response the mock
// serves, and reify, extractPayload and isMatcher over the data they return.
import assert from "node:assert/strict";
import { Matchers, Pact } from "pact-janus/pact";

const { like, string, boolean, integer, decimal, number, includes, nullValue, equal, contentType } = Matchers;

async function served(dir: string, description: string, body: unknown): Promise<unknown> {
  return new Pact({ consumer: "web-app", provider: "orders-api", dir })
    .addInteraction()
    .uponReceiving(description)
    .withRequest("GET", "/orders/1")
    .willRespondWith(200, (builder) => {
      builder.jsonBody(body);
    })
    .executeTest(async (mock) => (await fetch(`${mock.url}/orders/1`)).json());
}

export async function likeOnAValue({ dir }: { dir: string }): Promise<void> {
  const order = await served(dir, "an order of typed values", {
    sku: like("A-1"),
    name: string("Widget"),
    paid: boolean(true),
  });
  assert.deepEqual(order, { sku: "A-1", name: "Widget", paid: true });
}

export async function likeOnAnObject({ dir }: { dir: string }): Promise<void> {
  const order = await served(dir, "an order like an example", like({ id: 1, customer: { name: "Ada" }, tags: ["new"] }));
  assert.deepEqual(order, { id: 1, customer: { name: "Ada" }, tags: ["new"] });
}

export async function numbers({ dir }: { dir: string }): Promise<void> {
  const order = (await served(dir, "an order of numbers", {
    id: integer(1),
    total: decimal(9.99),
    weight: number(2),
    lines: integer(),
  })) as Record<string, number>;
  assert.equal(order.id, 1);
  assert.equal(order.total, 9.99);
  assert.equal(order.weight, 2);
  assert.ok(Number.isInteger(order.lines));
}

export async function includesNullEqualContentType({ dir }: { dir: string }): Promise<void> {
  const order = await served(dir, "an order of the other values", {
    note: includes("fragile"),
    coupon: nullValue(),
    currency: equal("EUR"),
  });
  assert.deepEqual(order, { note: "fragile", coupon: null, currency: "EUR" });
  await new Pact({ consumer: "web-app", provider: "orders-api", dir })
    .addInteraction()
    .uponReceiving("an order's invoice of a content type")
    .withRequest("GET", "/orders/1/invoice")
    .willRespondWith(200, (builder) => {
      builder.headers({ "Content-Type": "application/pdf" });
      builder.body("application/pdf", Buffer.from("%PDF-1.4 invoice"));
      builder.matchingRules({ body: { path: "$", rules: [contentType("application/pdf")] } });
    })
    .executeTest(async (mock) => {
      assert.equal((await fetch(`${mock.url}/orders/1/invoice`)).status, 200);
    });
}

export async function reifyExtractPayloadIsMatcher(): Promise<void> {
  const body = { id: integer(1), tags: Matchers.eachLike(string("new")), name: "Ada" };
  assert.deepEqual(Matchers.reify(body), { id: 1, tags: ["new"], name: "Ada" });
  assert.deepEqual(Matchers.extractPayload(body), { id: 1, tags: ["new"], name: "Ada" });
  assert.equal(Matchers.isMatcher(body.id), true);
  assert.equal(Matchers.isMatcher(body.name), false);
}
