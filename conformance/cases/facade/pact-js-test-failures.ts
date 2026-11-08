// facade/pact-js-test-failures: a test that throws, an interaction never called, and a request the mock
// did not match each fail executeTest as pact-js's does, and none of them records a contract.
import { Pact } from "pact-janus/pact";

function getOrder(dir: string, description: string) {
  return new Pact({ consumer: "web-app", provider: "orders-api", dir })
    .addInteraction()
    .uponReceiving(description)
    .withRequest("GET", "/orders/1")
    .willRespondWith(200, (builder) => {
      builder.jsonBody({ id: 1 });
    });
}

export async function testThrows({ dir }: { dir: string }): Promise<void> {
  await getOrder(dir, "get an order for a test that throws").executeTest(async () => {
    throw new Error("the client could not read the order");
  });
}

export async function requestMissing({ dir }: { dir: string }): Promise<void> {
  await getOrder(dir, "get an order nobody asks for").executeTest(async () => undefined);
}

export async function requestMismatched({ dir }: { dir: string }): Promise<void> {
  await getOrder(dir, "get an order at the wrong path").executeTest(async (mock) => {
    await fetch(`${mock.url}/orders/2`);
  });
}
