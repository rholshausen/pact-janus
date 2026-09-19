// The RFC's consumer example, near-verbatim, against the real engine (janus-engine, the subprocess
// embedding): a real HTTP client, every selected variant, and the contract the engine writes. The
// careful client passes every variant; the careless one — task 4.6's, which assumes `shippedAt` is
// always there — fails, naming the variant it failed on, and gets no contract.

import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  ContractWithheldError,
  Janus,
  VariantsFailedError,
  anyOf,
  date,
  datetime,
  eachLike,
  integer,
  json,
  oneOf,
  optional,
  regex,
  string,
} from "../src/index.js";

/** The consumer's own code: what the test exercises. */
interface Order {
  id: number;
  status: string;
  shippedAt?: string;
  lineCount: number;
  shippedOn?: string;
}

class OrderClient {
  readonly baseUrl: string;
  readonly careless: boolean;

  constructor(baseUrl: string, careless = false) {
    this.baseUrl = baseUrl;
    this.careless = careless;
  }

  async getOrder(id: string): Promise<Order> {
    const response = await fetch(`${this.baseUrl}/orders/${id}`, { headers: { Accept: "application/json" } });
    if (!response.ok) {
      throw new Error(`GET /orders/${id}: ${response.status} ${await response.text()}`);
    }
    const body = (await response.json()) as { id: number; status: string; shippedAt?: string; items: unknown[] };
    return {
      id: body.id,
      status: body.status,
      lineCount: body.items.length,
      // The careless client assumes every order has shipped.
      shippedOn: this.careless ? new Date(body.shippedAt as string).toISOString().slice(0, 10) : body.shippedAt?.slice(0, 10),
    };
  }
}

const getOrder = (janus: Janus) =>
  janus
    .interaction("get an order")
    .given("an order exists", { id: "42" })
    .request({ method: "GET", path: "/orders/42", headers: { Accept: "application/json" } })
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

describe("the RFC consumer example against the engine", () => {
  it("runs every selected variant and writes the contract", async () => {
    const dir = await mkdtemp(join(tmpdir(), "janus-rfc-"));
    const janus = new Janus({ consumer: "web-app", provider: "orders-api", contractDir: dir });
    const variants: string[] = [];

    await janus.execute(getOrder(janus), async (mock, variant) => {
      variants.push(variant.id);
      const client = new OrderClient(mock.url);
      const order = await client.getOrder("42");
      expect(order.lineCount).toBeGreaterThan(0);
    });
    const { results, contractFile } = await janus.finalise();

    // 3 statuses x shippedAt present/absent x 2 payments x 2 item counts = 24; the engine's pairwise
    // selection covers them in 8, base first (variant semantics spec §3).
    expect(variants).toHaveLength(8);
    expect(variants[0]).toBe("base");
    expect(variants).toContain("response.body.shippedAt#presence=absent");
    expect(results).toMatchObject([{ status: "verified" }]);

    const bytes = await readFile(contractFile ?? "", "utf8");
    expect(bytes.startsWith('{"$format":"janus-contract/1"')).toBe(true);
    expect(bytes.endsWith("}\n")).toBe(true);
    const contract = JSON.parse(bytes);
    expect(contract.consumer).toEqual({ name: "web-app" });
    expect(contract.interactions).toHaveLength(1);
    expect(contract.interactions[0].description).toBe("get an order");
    expect(contract.interactions[0].selection.variants).toHaveLength(8);
  }, 30_000);

  it("fails a careless consumer on the variant it cannot handle, and writes no contract", async () => {
    const dir = await mkdtemp(join(tmpdir(), "janus-rfc-"));
    const janus = new Janus({ consumer: "web-app", provider: "orders-api", contractDir: dir });

    const error = await janus
      .execute(getOrder(janus), async (mock) => {
        const order = await new OrderClient(mock.url, true).getOrder("42");
        expect(order.lineCount).toBeGreaterThan(0);
      })
      .catch((e: unknown) => e);

    expect(error).toBeInstanceOf(VariantsFailedError);
    const failed = error as VariantsFailedError;
    // Every variant where shippedAt is absent fails — and only those.
    expect(failed.failures.length).toBeGreaterThan(0);
    for (const { variant, cause } of failed.failures) {
      expect(variant.id).toContain("shippedAt#presence=absent");
      expect(cause).toBeInstanceOf(RangeError);
    }
    expect(failed.message).toContain("shippedAt=absent (response.body.shippedAt#presence=absent)");

    await expect(janus.finalise()).rejects.toThrow(ContractWithheldError);
    await expect(readFile(join(dir, "web-app-orders-api.janus.json"))).rejects.toThrow();
  }, 30_000);

  it("reports a request the mock could not match when the suite finalises", async () => {
    const dir = await mkdtemp(join(tmpdir(), "janus-rfc-"));
    const janus = new Janus({ consumer: "web-app", provider: "orders-api", contractDir: dir });
    const interaction = janus
      .interaction("get an order with a trace header")
      .request({ method: "GET", path: "/orders/42", headers: { "X-Trace": "t-1" } })
      .response({ status: 200 });

    // The client forgets the header: the mock answers 500, and swallowing that is the client's bug.
    await janus.execute(interaction, async (mock) => {
      await fetch(`${mock.url}/orders/42`);
    });
    const error = await janus.finalise().catch((e: unknown) => e);
    expect(error).toBeInstanceOf(ContractWithheldError);
    expect((error as Error).message).toContain("get an order with a trace header: failed");
    expect((error as Error).message).toContain("$.request.headers.x-trace");
  }, 30_000);
});
