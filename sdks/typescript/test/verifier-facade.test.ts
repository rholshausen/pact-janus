// pact-janus/verifier (ADR 0056 decision 4): pact-js's options become the protocol's documents as
// S1.13 maps today's CLI options, its handlers become the Janus verifier's, and the options with no
// Janus meaning are refused by name. The ported examples (test/examples) run it end to end; this
// covers what they do not use.

import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Verifier, MessageProviderPact } from "../src/facade/verifier.js";
import { ScriptedPipe } from "./conformance/scripted-engine.js";
import { json, serve, type Provider } from "./examples/support.js";

const pact = {
  consumer: { name: "web-app" },
  provider: { name: "orders-api" },
  interactions: [
    {
      description: "get an order",
      request: { method: "GET", path: "/orders/42", headers: { Accept: "application/json" } },
      response: { status: 200, headers: { "Content-Type": "application/json" }, body: { id: 42 } },
    },
  ],
  metadata: { pactSpecification: { version: "3.0.0" } },
};

async function pactFile(document: unknown = pact): Promise<string> {
  const file = join(await mkdtemp(join(tmpdir(), "janus-facade-")), "pact.json");
  await writeFile(file, JSON.stringify(document), "utf8");
  return file;
}

describe("pact-js's options", () => {
  it("are a broker source and publishing options, as S1.13 maps the CLI's", async () => {
    const pipe = new ScriptedPipe();
    await new Verifier({
      providerBaseUrl: "http://127.0.0.1:8080",
      provider: "orders-api",
      pactBrokerUrl: "https://broker.example",
      pactBrokerToken: "t0k3n",
      consumerVersionSelectors: [{ mainBranch: true }],
      consumerVersionTags: ["prod"],
      enablePending: true,
      includeWipPactsSince: "2026-01-01",
      providerVersionBranch: "main",
      publishVerificationResult: true,
      providerVersion: "1.2.3",
      buildUrl: "https://ci.example/1",
      consumerFilters: ["web-app"],
      customProviderHeaders: ["X-Api-Key: k-1"],
      disableSslVerification: true,
      engine: () => pipe,
    }).verifyProvider();
    const frame = pipe.bodyOf("verification/verify");
    expect(frame?.source).toEqual({
      kind: "broker",
      broker: { url: "https://broker.example", auth: { kind: "bearer", token: "t0k3n" } },
      provider: "orders-api",
      "consumer-version-selectors": [{ mainBranch: true }, { tag: "prod", latest: true }],
      "include-pending": true,
      "provider-branch": "main",
      "include-wip-since": "2026-01-01",
    });
    expect(frame?.options).toEqual({
      report: {},
      filter: { consumers: ["web-app"] },
      publish: { "provider-version": "1.2.3", branch: "main", "build-url": "https://ci.example/1" },
    });
    expect(frame?.target).toEqual({
      transports: [
        {
          transport: "http",
          options: { headers: { "X-Api-Key": "k-1" }, client: { tls: { insecure: true } }, "base-url": "http://127.0.0.1:8080" },
        },
      ],
    });
  });

  it("ask for no pending or work-in-progress pacts without selectors, as today's verifier", async () => {
    const pipe = new ScriptedPipe();
    await new Verifier({
      providerBaseUrl: "http://127.0.0.1:8080",
      provider: "orders-api",
      pactBrokerUrl: "https://broker.example",
      pactBrokerUsername: "bob",
      pactBrokerPassword: "pw",
      enablePending: true,
      includeWipPactsSince: "2026-01-01",
      failIfNoPactsFound: false,
      engine: () => pipe,
    }).verifyProvider();
    expect(pipe.bodyOf("verification/verify")?.source).toEqual({
      kind: "broker",
      broker: { url: "https://broker.example", auth: { kind: "basic", username: "bob", password: "pw" } },
      provider: "orders-api",
      "consumer-version-selectors": [],
      "include-pending": false,
      "require-pacts": false,
    });
  });

  it("read pactUrls that are URLs as a urls source", async () => {
    const pipe = new ScriptedPipe();
    await new Verifier({
      providerBaseUrl: "http://127.0.0.1:8080",
      pactUrls: ["https://broker.example/pacts/1"],
      pactBrokerToken: "t",
      engine: () => pipe,
    }).verifyProvider();
    expect(pipe.bodyOf("verification/verify")?.source).toEqual({
      kind: "urls",
      urls: ["https://broker.example/pacts/1"],
      auth: { kind: "bearer", token: "t" },
    });
  });

  it.each([
    [{ beforeEach: () => undefined }, "'beforeEach'"],
    [{ providerStatesSetupUrl: "http://localhost/setup" }, "'providerStatesSetupUrl'"],
    [{ pactUrls: ["https://broker.example/pacts/1", "./pact.json"] }, "both files and URLs"],
    [{ pactUrls: ["./pact.json"], pactBrokerUrl: "https://broker.example" }, "one kind of source"],
    [{ publishVerificationResult: true }, "providerVersion"],
  ])("refuse %o before an engine starts", async (options, message) => {
    const pipe = new ScriptedPipe();
    const run = new Verifier({ providerBaseUrl: "http://127.0.0.1:8080", engine: () => pipe, ...options }).verifyProvider();
    await expect(run).rejects.toThrow(message);
    expect(pipe.ops).toEqual([]);
  });
});

describe("a request filter", () => {
  let provider: Provider;

  beforeAll(async () => {
    provider = await serve((request, _body, response) => {
      if (request.headers.authorization === "Bearer from-the-filter") {
        json(response, 200, { id: 42 });
      } else {
        json(response, 401, { error: "no token" });
      }
    });
  });

  afterAll(() => provider.close());

  it("is an Express middleware whose headers reach the provider", async () => {
    const seen: string[] = [];
    const result = await new Verifier({
      providerBaseUrl: provider.url,
      pactUrls: [await pactFile()],
      requestFilter: (req, _res, next) => {
        seen.push(`${req.method} ${req.path} ${req.headers.accept}`);
        req.headers.authorization = "Bearer from-the-filter";
        next();
      },
    }).verifyProvider();
    expect(result.status).toBe("verified");
    expect(seen).toEqual(["GET /orders/42 application/json"]);
  }, 30_000);

  it("that answers the request itself fails the exchange", async () => {
    const run = new Verifier({
      providerBaseUrl: provider.url,
      pactUrls: [await pactFile()],
      requestFilter: (_req, res) => {
        (res as { sendStatus(status: number): void }).sendStatus(200);
      },
    }).verifyProvider();
    await expect(run).rejects.toThrow("get an order");
  }, 30_000);
});

describe("a message provider", () => {
  it("with no metadata is JSON, as pact-js's res.json writes it", async () => {
    const messages = {
      consumer: { name: "web-app" },
      provider: { name: "orders-api" },
      messages: [{ description: "an order", contents: { id: 42 }, metadata: { contentType: "application/json" } }],
      metadata: { pactSpecification: { version: "3.0.0" } },
    };
    const received: unknown[] = [];
    const result = await new MessageProviderPact({
      pactUrls: [await pactFile(messages)],
      messageProviders: {
        "an order": (descriptor) => {
          received.push(descriptor);
          return { id: 42 };
        },
      },
    }).verify();
    expect(result.status).toBe("verified");
    expect(received).toEqual([{ description: "an order" }]);
  }, 30_000);
});
