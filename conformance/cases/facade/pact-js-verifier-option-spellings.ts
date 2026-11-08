// facade/pact-js-verifier-option-spellings: pact-js's other spellings of the same options: the
// deprecated providerBranch, validateSSL: false, and custom headers as "Name: value" lines.
import { Verifier } from "pact-janus/pact";
import type { FramePipe } from "pact-janus";

export async function otherSpellings({ engine }: { engine?: () => FramePipe }): Promise<void> {
  await new Verifier({
    providerBaseUrl: "http://127.0.0.1:8080",
    provider: "orders-api",
    pactBrokerUrl: "https://broker.example",
    consumerVersionSelectors: [{ mainBranch: true }],
    providerBranch: "main",
    publishVerificationResult: true,
    providerVersion: "1.2.3",
    customProviderHeaders: ["X-Api-Key: k-1"],
    validateSSL: false,
    engine,
  }).verifyProvider();
}
