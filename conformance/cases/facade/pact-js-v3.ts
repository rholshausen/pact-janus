// facade/pact-js-v3: PactV3's builder, its calls in pact-js's order, its binary files, multipart
// uploads and matching rules.
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Matchers, PactV3 } from "pact-janus/pact";

const { like, regex } = Matchers;
const options = (dir: string) => ({ consumer: "web-app", provider: "orders-api", dir });

async function file(name: string, contents: string): Promise<string> {
  const path = join(await mkdtemp(join(tmpdir(), "pact-js-v3-")), name);
  await writeFile(path, contents);
  return path;
}

export async function theBuilderInOrder({ dir }: { dir: string }): Promise<void> {
  const provider = new PactV3(options(dir));
  provider
    .given("an order exists", { id: 7 })
    .uponReceiving("get order 7 from V3")
    .withRequest({ method: "GET", path: "/orders/7", headers: { Accept: "application/json" } })
    .willRespondWith({ status: 200, headers: { "Content-Type": "application/json" }, body: { id: like(7) } });
  const id = await provider.executeTest(async (mock) => {
    const response = await fetch(`${mock.url}/orders/7`, { headers: { Accept: "application/json" } });
    return ((await response.json()) as { id: number }).id;
  });
  assert.equal(id, 7);
}

export async function addInteraction({ dir }: { dir: string }): Promise<void> {
  const provider = new PactV3(options(dir));
  provider.addInteraction({
    states: [{ description: "no orders exist" }],
    uponReceiving: "list orders from V3",
    withRequest: { method: "GET", path: "/orders" },
    willRespondWith: { status: 200, body: [] },
  });
  await provider.executeTest(async (mock) => {
    assert.deepEqual(await (await fetch(`${mock.url}/orders`)).json(), []);
  });
}

export async function binaryFiles({ dir }: { dir: string }): Promise<void> {
  const request = await file("order.pdf", "%PDF-1.4 an order");
  const response = await file("receipt.pdf", "%PDF-1.4 a receipt");
  const provider = new PactV3(options(dir));
  provider
    .uponReceiving("exchange PDFs from V3")
    .withRequestBinaryFile({ method: "POST", path: "/receipts" }, "application/pdf", request)
    .withResponseBinaryFile({ status: 201 }, "application/pdf", response);
  await provider.executeTest(async (mock) => {
    const answer = await fetch(`${mock.url}/receipts`, {
      method: "POST",
      headers: { "Content-Type": "application/pdf" },
      body: "%PDF-1.4 an order",
    });
    assert.equal(answer.status, 201);
    assert.equal(await answer.text(), "%PDF-1.4 a receipt");
  });
}

export async function multipartUploads({ dir }: { dir: string }): Promise<void> {
  const upload = await file("order.txt", "sku=A-1");
  const provider = new PactV3(options(dir));
  provider
    .uponReceiving("upload an order file from V3")
    .withRequestMultipartFileUpload({ method: "POST", path: "/uploads" }, "text/plain", upload, "order", "order-boundary")
    .withResponseMultipartFileUpload({ status: 201 }, "text/plain", upload, "receipt", "receipt-boundary");
  await provider.executeTest(async (mock) => {
    const form = new FormData();
    form.append("order", new Blob(["sku=A-1"], { type: "text/plain" }), "order.txt");
    const answer = await fetch(`${mock.url}/uploads`, { method: "POST", body: form });
    assert.equal(answer.status, 201, await answer.text());
  });
}

export async function matchingRules({ dir }: { dir: string }): Promise<void> {
  const provider = new PactV3(options(dir));
  // As pact-js's: each takes the request's details, and the response's rules come before its details.
  const request = { method: "POST", path: "/orders", headers: { "Content-Type": "application/json" }, body: { sku: "A-1" } };
  provider
    .uponReceiving("create an order with V3 rules")
    .withRequest(request)
    .withRequestMatchingRules(request, { body: { path: "$.sku", rules: [regex(/^[A-Z]-\d+$/, "A-1")] } })
    .withResponseMatchingRules(request, { body: { path: "$.id", rules: [Matchers.integer(1)] } })
    .willRespondWith({ status: 201, headers: { "Content-Type": "application/json" }, body: { id: 1 } });
  await provider.executeTest(async (mock) => {
    const answer = await fetch(`${mock.url}/orders`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ sku: "Q-4" }),
    });
    assert.equal(answer.status, 201);
  });
}
