// pact-js examples/multipart/provider.test.ts, on pact-janus/verifier (ADR 0056 decision 5). The
// provider accepts a multipart/form-data body with a 'file' part, as multer's upload.single('file').

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Verifier } from "../../src/facade/verifier.js";
import { corpusPact, json, serve, type Provider } from "./support.js";

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

  it("satisfies all UploadConsumer expectations", async () => {
    const result = await new Verifier({
      providerBaseUrl: server.url,
      pactUrls: [corpusPact("examples/pact-js/multipart/pact.json")],
      stateHandlers: {
        "the upload service is ready": async () => {},
      },
      logLevel: "warn",
    }).verifyProvider();
    expect(result.status).toBe("verified");
  }, 30_000);
});
