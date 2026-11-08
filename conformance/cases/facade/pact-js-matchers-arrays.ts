// facade/pact-js-matchers-arrays: pact-js's array and map matchers, each in a response the mock serves.
import assert from "node:assert/strict";
import { Matchers, Pact } from "pact-janus/pact";

const { eachLike, atLeastOneLike, atLeastLike, atMostLike, constrainedArrayLike, eachKeyLike, eachValueMatches, integer, regex } =
  Matchers;

async function served(dir: string, description: string, body: unknown): Promise<unknown> {
  return new Pact({ consumer: "web-app", provider: "orders-api", dir })
    .addInteraction()
    .uponReceiving(description)
    .withRequest("GET", "/orders")
    .willRespondWith(200, (builder) => {
      builder.jsonBody(body);
    })
    .executeTest(async (mock) => (await fetch(`${mock.url}/orders`)).json());
}

export async function eachLikeAndAtLeast({ dir }: { dir: string }): Promise<void> {
  const orders = await served(dir, "orders with minimums", {
    any: eachLike({ id: integer(1) }),
    two: eachLike("x", 2),
    one: atLeastOneLike("y"),
    three: atLeastLike("z", 3),
    between: constrainedArrayLike("w", 1, 3, 2),
  });
  assert.deepEqual(orders, { any: [{ id: 1 }], two: ["x", "x"], one: ["y"], three: ["z", "z", "z"], between: ["w", "w"] });
}

export async function atMostLikeHasNoMinimum({ dir }: { dir: string }): Promise<void> {
  const orders = await served(dir, "orders with a maximum", { recent: atMostLike({ id: 1 }, 5, 2) });
  assert.deepEqual(orders, { recent: [{ id: 1 }, { id: 1 }] });
}

export async function eachKeyAndValue({ dir }: { dir: string }): Promise<void> {
  const orders = await served(dir, "orders by key", {
    byId: eachKeyLike("order-1", { total: integer(10) }),
    codes: eachValueMatches({ first: "A-1" }, regex(/^[A-Z]-\d+$/, "A-1")),
  });
  assert.deepEqual(orders, { byId: { "order-1": { total: 10 } }, codes: { first: "A-1" } });
}
