// The RFC consumer example as a user writes it: `useJanus` once per suite, `execute` in a test,
// and `finalise` left to the integration's own `afterAll` (behavioural spec 'finalise': the
// test-framework integration runs it after the suite's last execute).

import { mkdtempSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { anyOf, date, datetime, eachLike, integer, json, oneOf, optional, regex, string } from "../src/index.js";
import { useJanus } from "../src/vitest.js";

const contractDir = mkdtempSync(join(tmpdir(), "janus-vitest-"));

describe("web-app's contract with orders-api", () => {
  const janus = useJanus({ consumer: "web-app", provider: "orders-api", contractDir });

  const getOrder = janus
    .interaction("get an order")
    .given("an order exists", { id: "42" })
    .request({ method: "GET", path: "/orders/42" })
    .response({
      status: 200,
      body: json({
        id: integer(42),
        status: anyOf("PENDING", "SHIPPED", "DELIVERED"),
        shippedAt: optional(datetime("2026-07-30T10:00:00Z")),
        payment: oneOf("type", {
          card: { type: "card", last4: regex(/\d{4}/, "1234") },
          invoice: { type: "invoice", dueDate: date("2026-08-30") },
        }),
        items: eachLike({ sku: string("SKU-1"), qty: integer(1) }, { min: 1 }),
      }),
    });

  it("gets an order", async () => {
    await janus.execute(getOrder, async (mock, variant) => {
      const response = await fetch(`${mock.url}/orders/42`);
      const order = (await response.json()) as { items: unknown[]; shippedAt?: string };
      expect(order.items.length, variant.id).toBeGreaterThan(0);
    });
  }, 30_000);
});

// Runs after the suite above, and so after its afterAll.
it("has the suite's contract written by the time the suite ends", async () => {
  const contract = JSON.parse(await readFile(join(contractDir, "web-app-orders-api.janus.json"), "utf8"));
  expect(contract.interactions.map((i: { description: string }) => i.description)).toEqual(["get an order"]);
});
