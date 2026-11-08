// facade/pact-js-log-level: logLevel, on a consumer test and on a verification, is how much the engine
// logs on its standard error.
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Pact, Verifier } from "pact-janus/pact";

export async function consumerAtDebug({ dir }: { dir: string }): Promise<void> {
  await new Pact({ consumer: "web-app", provider: "orders-api", dir, logLevel: "debug" })
    .addInteraction()
    .uponReceiving("ping the mock, logging at debug")
    .withRequest("GET", "/ping")
    .willRespondWith(204)
    .executeTest(async (mock) => {
      assert.equal((await fetch(`${mock.url}/ping`)).status, 204);
    });
}

export async function verifierAtDebug(): Promise<void> {
  const server = createServer((_request, response) => response.writeHead(204).end());
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", () => resolve()));
  const file = join(await mkdtemp(join(tmpdir(), "pact-js-log-")), "pact.json");
  await writeFile(
    file,
    JSON.stringify({
      consumer: { name: "web-app" },
      provider: { name: "orders-api" },
      interactions: [{ description: "ping", request: { method: "GET", path: "/ping" }, response: { status: 204 } }],
      metadata: { pactSpecification: { version: "3.0.0" } },
    }),
  );
  try {
    const result = await new Verifier({
      providerBaseUrl: `http://127.0.0.1:${(server.address() as { port: number }).port}`,
      pactUrls: [file],
      logLevel: "debug",
    }).verifyProvider();
    assert.equal(result.status, "verified");
  } finally {
    server.close();
  }
}
