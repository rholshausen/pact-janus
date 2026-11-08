// facade/pact-js-verifier-broker-options: pact-js's Verifier options for a broker, publishing,
// filtering and the provider's transport, as one verification asks for them.
import { Verifier } from "pact-janus/pact";
import type { FramePipe } from "pact-janus";

export async function brokerOptions({ engine }: { engine?: () => FramePipe }): Promise<void> {
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
    providerVersionTags: ["release"],
    failIfNoPactsFound: false,
    publishVerificationResult: true,
    providerVersion: "1.2.3",
    buildUrl: "https://ci.example/1",
    consumerFilters: ["web-app"],
    customProviderHeaders: { "X-Api-Key": "k-1" },
    disableSslVerification: true,
    engine,
  }).verifyProvider();
}
