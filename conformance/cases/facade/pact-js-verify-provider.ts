// facade/pact-js-verify-provider: pact-js's Verifier, run against a provider, from pact files, with
// its state handlers; a state with no handler warns; and a failed verification rejects.
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Verifier } from "pact-janus/pact";

const pact = (status: number) => ({
  consumer: { name: "web-app" },
  provider: { name: "orders-api" },
  interactions: [
    {
      description: "get an order",
      providerStates: [{ name: "an order exists", params: { id: 42 } }],
      request: { method: "GET", path: "/orders/42" },
      response: { status, headers: { "Content-Type": "application/json" }, body: { id: 42 } },
    },
    {
      description: "list orders",
      providerStates: [{ name: "orders exist" }],
      request: { method: "GET", path: "/orders" },
      response: { status: 200, headers: { "Content-Type": "application/json" }, body: [] },
    },
  ],
  metadata: { pactSpecification: { version: "3.0.0" } },
});

async function pactFile(document: unknown): Promise<string> {
  const file = join(await mkdtemp(join(tmpdir(), "pact-js-verify-")), "pact.json");
  await writeFile(file, JSON.stringify(document));
  return file;
}

/** The provider: the orders it was set up with, as JSON. */
async function provider(orders: number[]): Promise<{ url: string; server: Server }> {
  const server = createServer((request, response) => {
    const id = Number(request.url?.split("/")[2]);
    const body = request.url === "/orders" ? [] : orders.includes(id) ? { id } : undefined;
    response.writeHead(body === undefined ? 404 : 200, { "Content-Type": "application/json" }).end(JSON.stringify(body ?? {}));
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", () => resolve()));
  return { url: `http://127.0.0.1:${(server.address() as { port: number }).port}`, server };
}

export async function verifiesWithStateHandlers(): Promise<void> {
  const orders: number[] = [];
  const calls: string[] = [];
  const { url, server } = await provider(orders);
  try {
    const result = await new Verifier({
      providerBaseUrl: url,
      pactUrls: [await pactFile(pact(200))],
      stateHandlers: {
        "an order exists": {
          setup: (params) => {
            calls.push(`setup ${JSON.stringify(params)}`);
            orders.push(Number(params?.id));
            return { id: params?.id };
          },
          teardown: () => {
            calls.push("teardown");
            orders.length = 0;
          },
        },
        "orders exist": () => {
          calls.push("orders exist");
        },
      },
    }).verifyProvider();
    assert.equal(result.status, "verified");
    assert.deepEqual(calls.sort(), ['setup {"id":42}', "orders exist", "teardown"].sort());
  } finally {
    server.close();
  }
}

export async function aStateWithNoHandlerWarns(): Promise<void> {
  const { url, server } = await provider([42]);
  try {
    const result = await new Verifier({
      providerBaseUrl: url,
      pactUrls: [await pactFile(pact(200))],
      stateHandlers: { "an order exists": () => undefined },
    }).verifyProvider();
    assert.equal(result.status, "verified");
    assert.deepEqual(
      result.warnings.map((warning) => (warning as { code?: string }).code),
      ["state-unhandled"],
    );
  } finally {
    server.close();
  }
}

export async function aFailedVerificationRejects(): Promise<void> {
  const { url, server } = await provider([]);
  try {
    await new Verifier({
      providerBaseUrl: url,
      pactUrls: [await pactFile(pact(200))],
      stateHandlers: { "an order exists": () => undefined, "orders exist": () => undefined },
    }).verifyProvider();
  } finally {
    server.close();
  }
}
