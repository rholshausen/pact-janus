// pact-js examples/multipart, both sides, on pact-janus/pact (ADR 0056 decision 5, ADR 0066). The
// consumer test is the example's, with its import changed and its client on fetch and FormData; it
// records what `janus upgrade` makes of the pact pact-js wrote, but for the boundary, which pact_ffi
// drew at random and the engine derives from the part (ADR 0066 decision 7). The provider accepts a
// multipart/form-data body with a 'file' part, as multer's upload.single('file').

import { readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Pact, SpecificationVersion, Verifier } from "../../src/facade/pact.js";
import { contractDir, corpusPact, json, recorded, serve, shapes, upgraded, type Provider } from "./support.js";

/** examples/multipart/consumer.ts, on fetch and FormData. */
async function uploadFile(baseUrl: string, filePath: string, fieldName: string): Promise<{ success: boolean }> {
  const form = new FormData();
  form.append(fieldName, new Blob([readFileSync(filePath)], { type: "text/plain" }), basename(filePath));
  const response = await fetch(`${baseUrl}/upload`, { method: "POST", body: form });
  return (await response.json()) as { success: boolean };
}

const dir = contractDir();
const pact = corpusPact("examples/pact-js/multipart/pact.json");
const contract = "UploadConsumer-UploadProvider.janus.json";
const description = "a multipart file upload";

describe("uploadFile", () => {
  const provider = new Pact({
    consumer: "UploadConsumer",
    provider: "UploadProvider",
    spec: SpecificationVersion.SPECIFICATION_VERSION_V4,
    dir,
    logLevel: "warn",
  });

  it("uploads a file successfully", async () => {
    // examples/multipart/fixtures/sample.txt
    const testFile = join(dir, "sample.txt");
    writeFileSync(testFile, "Sample file for multipart upload testing.\nThis file is a test fixture.\n");

    await provider
      .addInteraction()
      .given("the upload service is ready")
      .uponReceiving(description)
      .withRequest("POST", "/upload", (builder) => {
        builder.multipartBody("text/plain", testFile, "file");
      })
      .willRespondWith(200, (builder) => {
        builder.headers({ "Content-Type": "application/json" });
        builder.jsonBody({ success: true });
      })
      .executeTest(async (mockserver) => {
        const result = await uploadFile(mockserver.url, testFile, "file");
        expect(result.success).toBe(true);
      });
  }, 30_000);

  it("records what janus upgrade makes of pact-js's pact, but for the boundary", async () => {
    const original = /boundary=U9iXwlWODmM9xpRU/g;
    const ours = JSON.stringify(shapes(recorded(dir, contract, description))).replace(/boundary=[0-9a-f]{16}/g, "boundary=U9iXwlWODmM9xpRU");
    expect(original.test(ours)).toBe(true);
    expect(JSON.parse(ours)).toEqual(shapes(await upgraded(pact, description)));
  }, 30_000);
});

describe("UploadProvider", () => {
  let server: Provider;

  beforeAll(async () => {
    server = await serve((request, body, response) => {
      const multipart = (request.headers["content-type"] ?? "").startsWith("multipart/form-data");
      if (multipart && /name="file"/.test(body)) {
        json(response, 200, { success: true });
      } else {
        json(response, 400, { success: false, error: "No file uploaded" });
      }
    });
  });

  afterAll(() => server.close());

  const verifier = (file: string) =>
    new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [file],
      stateHandlers: {
        "the upload service is ready": async () => {},
      },
      logLevel: "warn",
    });

  it("satisfies all UploadConsumer expectations", async () => {
    const result = await verifier(pact).verifyProvider();
    expect(result.status).toBe("verified");
  }, 30_000);

  it("verifies the contract the consumer test recorded", async () => {
    const result = await verifier(join(dir, contract)).verifyProvider();
    expect(result.status).toBe("verified");
  }, 30_000);
});
