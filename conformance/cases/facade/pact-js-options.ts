// facade/pact-js-options: the V4 builder's `spec`, which pact version its calls build to, and `host`
// and `port`, the mock's address.
import assert from "node:assert/strict";
import { createServer } from "node:net";
import { Pact, SpecificationVersion } from "pact-janus/pact";

async function upload(dir: string, spec: SpecificationVersion, description: string): Promise<void> {
  await new Pact({ consumer: "web-app", provider: "orders-api", dir, spec })
    .addInteraction()
    .uponReceiving(description)
    .withRequest("POST", "/receipts", (builder) => {
      builder.body("application/octet-stream", Buffer.from([0, 1, 2, 3]));
    })
    .willRespondWith(201)
    .executeTest(async (mock) => {
      const response = await fetch(`${mock.url}/receipts`, {
        method: "POST",
        headers: { "Content-Type": "application/octet-stream" },
        body: Buffer.from([0, 1, 2, 3]),
      });
      assert.equal(response.status, 201);
    });
}

export async function specV2({ dir }: { dir: string }): Promise<void> {
  await upload(dir, SpecificationVersion.SPECIFICATION_VERSION_V2, "upload octets to a V2 pact");
}

export async function specV3({ dir }: { dir: string }): Promise<void> {
  await upload(dir, SpecificationVersion.SPECIFICATION_VERSION_V3, "upload octets to a V3 pact");
}

/** A port nothing listens on, as a test that fixes its mock's port would choose one. */
async function freePort(): Promise<number> {
  const server = createServer();
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", () => resolve()));
  const { port } = server.address() as { port: number };
  await new Promise<void>((resolve) => server.close(() => resolve()));
  return port;
}

export async function hostAndPort({ dir }: { dir: string }): Promise<void> {
  const port = await freePort();
  await new Pact({ consumer: "web-app", provider: "orders-api", dir, host: "127.0.0.1", port })
    .addInteraction()
    .uponReceiving("ping the mock at its address")
    .withRequest("GET", "/ping")
    .willRespondWith(204)
    .executeTest(async (mock) => {
      assert.equal(mock.port, port);
      assert.equal(mock.url, `http://127.0.0.1:${port}`);
      assert.equal((await fetch(`http://127.0.0.1:${port}/ping`)).status, 204);
    });
}
