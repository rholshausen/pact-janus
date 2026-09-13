// Plan task 4.5: the same protocol-level consumer flow
// engine/kernel/tests/consumer_flow.rs proves in-process, proven here over the `janus-engine`
// subprocess binary instead — the 1.3 embedding, against the real engine rather than 1.3's own
// toy. Submit the RFC order interaction, iterate its variants, run a real HTTP client (Node's
// built-in `http`, no new dependency) against the mock per variant, finalise, assert the written
// contract.

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { execFileSync } from "node:child_process";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { EngineClient } from "./engine-client.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../../../..");
const binaryPath = path.join(repoRoot, "target", "debug", "janus-engine");

function orderInteraction() {
  return {
    description: "a request for an order",
    transport: { kind: "http", mode: "passive" },
    parts: {
      request: {
        method: { shape: "equality", example: "GET" },
        path: { shape: "equality", example: "/orders/66" },
      },
      response: {
        status: { shape: "equality", example: 200 },
        body: { shape: "object", members: {} },
      },
    },
  };
}

function httpGet(baseUrl: string, requestPath: string): Promise<{ status: number; body: unknown }> {
  return new Promise((resolve, reject) => {
    const request = http.get(`${baseUrl}${requestPath}`, (response) => {
      const chunks: Buffer[] = [];
      response.on("data", (chunk: Buffer) => chunks.push(chunk));
      response.on("end", () => {
        const text = Buffer.concat(chunks).toString("utf8");
        resolve({ status: response.statusCode ?? 0, body: text.length > 0 ? JSON.parse(text) : null });
      });
    });
    request.on("error", reject);
  });
}

describe("janus-engine subprocess: a real HTTP client driven through the mock to a written contract", () => {
  beforeAll(() => {
    execFileSync("cargo", ["build", "-p", "pact_janus_cli", "--bin", "janus-engine"], {
      cwd: repoRoot,
      stdio: "inherit",
    });
  }, 120_000);

  it("submits the RFC order interaction, iterates its variants, and finalises honestly", async () => {
    const engine = new EngineClient(binaryPath);
    try {
      const hello = await engine.send("engine/hello", {
        "protocol-versions": [1],
        host: { name: "janus-engine-node-test", version: "0.0.0" },
        capabilities: {},
      });
      expect((hello.ok as Record<string, unknown>)["protocol-version"]).toBe(1);

      const created = await engine.send("consumer-session/create", {
        config: { consumer: { name: "web-app-ts" }, provider: { name: "order-api" } },
      });
      const session = (created.ok as { session: string }).session;

      const added = await engine.send("consumer-session/add-interaction", {
        session,
        interaction: orderInteraction(),
      });
      const handle = (added.ok as { handle: string }).handle;

      const variants = await engine.send("consumer-session/variants", { session, handle });
      const variantIds = (variants.ok as { variants: Array<{ id: string }> }).variants.map((v) => v.id);
      expect(variantIds).toEqual(["base"]);

      const started = await engine.send("consumer-session/start-transport", { session, transport: "http" });
      const endpoint = (started.ok as { endpoint: { "base-url": string } }).endpoint;
      const baseUrl = endpoint["base-url"];

      for (const variantId of variantIds) {
        const served = await engine.send("consumer-session/serve-variant", { session, handle, variant: variantId });
        expect(served.ok).toEqual({});

        const { status, body } = await httpGet(baseUrl, "/orders/66");
        expect(status).toBe(200);
        expect(body).toEqual({});
      }

      const finalised = await engine.send("consumer-session/finalise", { session });
      expect(finalised.ok).toMatchObject({
        results: [{ handle: "i-1", status: "verified", variants: [{ variant: "base", status: "verified" }] }],
      });
      const contract = (finalised.ok as { contract: Record<string, unknown> }).contract;
      expect(contract["$format"]).toBe("janus-contract/1");
      expect((contract.consumer as { name: string }).name).toBe("web-app-ts");
      expect((contract.provider as { name: string }).name).toBe("order-api");
    } finally {
      engine.close();
    }
  }, 30_000);

  afterAll(() => {
    // Nothing to tear down beyond each test's own `engine.close()`: EOF ends the janus-engine
    // process itself (engine-protocol spec §7.1) — there is no separate cleanup call.
  });
});
