// A TypeScript SDK release, installed and used as a user would (staged plan S1.36, ADR 0060
// decision 7): a consumer test records a contract against the engine's mock, and a provider test
// verifies that contract against a provider it serves. Nothing names an engine, so the SDK fetches
// the one it was released with. release-sdks.yml runs this outside the repository, with the packed
// tarball installed and an empty JANUS_ENGINE_CACHE, before anything is published.

import assert from "node:assert/strict";
import { mkdtemp, readdir } from "node:fs/promises";
import { createServer } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { Janus, Verifier, integer, json, string } from "pact-janus";

let contractFile: string | undefined;

test("nothing names an engine", () => {
  assert.equal(process.env.JANUS_ENGINE ?? "", "");
});

test("a consumer test records a contract", async () => {
  const janus = new Janus({
    consumer: "release-check-web",
    provider: "release-check-orders",
    contractDir: await mkdtemp(join(tmpdir(), "pact-janus-release-check-")),
  });
  const interaction = janus
    .interaction("get an order")
    .given("an order exists", { id: 42 })
    .request({ method: "GET", path: "/orders/42" })
    .response({ status: 200, body: json({ id: integer(42), status: string("SHIPPED") }) });

  await janus.execute(interaction, async (mock) => {
    const response = await fetch(`${mock.url}/orders/42`);
    assert.equal(response.status, 200);
    const order = (await response.json()) as { id: number };
    assert.equal(typeof order.id, "number");
  });
  ({ contractFile } = await janus.finalise());
  assert.ok(contractFile, "a contract was written");
});

test("a provider test verifies it", async () => {
  assert.ok(contractFile, "the consumer test wrote a contract");
  const states: unknown[] = [];
  const provider = createServer((request, response) => {
    if (request.method === "GET" && request.url === "/orders/42") {
      response.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify({ id: 42, status: "PENDING" }));
    } else {
      response.writeHead(404).end();
    }
  });
  await new Promise<void>((listening) => provider.listen(0, "127.0.0.1", listening));
  try {
    const result = await new Verifier({
      providerUrl: `http://127.0.0.1:${(provider.address() as AddressInfo).port}`,
      pacts: [contractFile],
    })
      .state("an order exists", (params) => {
        states.push(params);
      })
      .verify();
    assert.equal(result.status, "verified");
    assert.deepEqual(states, [{ id: 42 }]);
  } finally {
    await new Promise<void>((closed) => provider.close(() => closed()));
  }
});

test("the engine it ran is the one the release pins, fetched into the cache", async (t) => {
  const cache = process.env.JANUS_ENGINE_CACHE;
  if (!cache) {
    t.skip("JANUS_ENGINE_CACHE is not set, so the cache is the user's");
    return;
  }
  const [version, ...others] = await readdir(cache);
  assert.deepEqual(others, []);
  assert.match(version ?? "", /^\d+\.\d+\.\d+/);
  assert.deepEqual((await readdir(join(cache, version ?? ""))).filter((file) => file.startsWith("janus-engine-")).length, 1);
});
