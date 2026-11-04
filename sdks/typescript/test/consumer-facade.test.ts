// pact-janus/pact's consumer API (ADR 0066): what it does beyond pact-js's examples, which
// test/examples runs. Its matchers are pact-js's data; what it cannot carry is refused at the call;
// executeTest fails as pact-js's does; and two suites recording one pair merge into one contract.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { Matchers, Pact, PactV3 } from "../src/facade/pact.js";
import { contractDir } from "./examples/support.js";

const { like, eachLike, regex, integer, reify } = Matchers;

const options = (dir: string) => ({ consumer: "Web", provider: "Orders", dir, logLevel: "error" as const });

function getOrder(provider: Pact, description = "get an order") {
  return provider
    .addInteraction()
    .uponReceiving(description)
    .withRequest("GET", "/orders/1")
    .willRespondWith(200, (builder) => builder.jsonBody({ id: integer(1) }));
}

describe("the matchers", () => {
  it("are pact-js's integration JSON, which reify reads back", () => {
    expect(like({ id: 1 })).toEqual({ "pact:matcher:type": "type", value: { id: 1 } });
    expect(eachLike("a", 2)).toEqual({ "pact:matcher:type": "type", min: 2, value: ["a", "a"] });
    expect(regex(/\d+/, "12")).toEqual({ "pact:matcher:type": "regex", regex: "\\d+", value: "12" });
    expect(reify({ ids: eachLike(integer(7)), name: like("x") })).toEqual({ ids: [7], name: "x" });
  });

  it("refuse a url fragment whose example randexp would have generated", () => {
    expect(() => Matchers.url([/\d+/])).toThrow(/regex\(/);
  });
});

describe("what has no conversion", () => {
  const dir = contractDir();

  it("is refused at the call, by name", () => {
    expect(() => new Pact({ ...options(dir), tls: true })).toThrow(/tls/);
    expect(() => new Pact(options(dir)).addInteraction().usingPlugin({ plugin: "protobuf", version: "0.4" })).toThrow(/'protobuf'/);
    expect(() => new Pact(options(dir)).addAsynchronousInteraction().usingPlugin({ plugin: "protobuf", version: "0.4" })).toThrow(/'protobuf'/);
    expect(() => new Pact(options(dir)).addSynchronousInteraction("a plugin message").usingPlugin({ plugin: "protobuf", version: "0.4" })).toThrow(/'protobuf'/);
  });

});

// ADR 0069 (staged plan S2.13): every interaction pact-js would serve is armed at once, each for
// as many calls as the test makes.
describe("several interactions", () => {
  const order = (path: string) => ({ method: "GET", path });

  it("are each served in one PactV3 executeTest, in any order, and recorded", async () => {
    const dir = contractDir();
    const provider = new PactV3(options(dir));
    for (const id of [1, 2]) {
      provider
        .uponReceiving(`get order ${id}`)
        .withRequest(order(`/orders/${id}`))
        .willRespondWith({ status: 200, body: { id: integer(id) } });
    }
    await provider.executeTest(async (mock) => {
      expect(await (await fetch(`${mock.url}/orders/2`)).json()).toEqual({ id: 2 });
      expect(await (await fetch(`${mock.url}/orders/1`)).json()).toEqual({ id: 1 });
    });
    const contract = JSON.parse(readFileSync(join(dir, "Web-Orders.janus.json"), "utf8")) as { interactions: { description: string }[] };
    expect(contract.interactions.map((i) => i.description).sort()).toEqual(["get order 1", "get order 2"]);
  }, 30_000);

  it("are every one PactV4 added since its last executeTest", async () => {
    const dir = contractDir();
    const provider = new Pact(options(dir));
    getOrder(provider, "get order 1");
    await provider
      .addInteraction()
      .uponReceiving("get order 2")
      .withRequest("GET", "/orders/2")
      .willRespondWith(200, (builder) => builder.jsonBody({ id: integer(2) }))
      .executeTest(async (mock) => {
        expect((await fetch(`${mock.url}/orders/1`)).status).toBe(200);
        expect((await fetch(`${mock.url}/orders/2`)).status).toBe(200);
      });
    // Taken by that executeTest: the next one serves only its own.
    await getOrder(provider, "get order 1 again").executeTest(async (mock) => {
      expect((await fetch(`${mock.url}/orders/1`)).status).toBe(200);
    });
  }, 30_000);

  it("serve an interaction as often as the test calls it", async () => {
    const dir = contractDir();
    await getOrder(new Pact(options(dir))).executeTest(async (mock) => {
      for (let page = 0; page < 3; page++) {
        expect(await (await fetch(`${mock.url}/orders/1`)).json()).toEqual({ id: 1 });
      }
    });
    expect(existsSync(join(dir, "Web-Orders.janus.json"))).toBe(true);
  }, 30_000);

  it("fail a test that never called one of them, naming it, and write no contract", async () => {
    const dir = contractDir();
    const provider = new PactV3(options(dir));
    for (const id of [1, 2]) {
      provider.uponReceiving(`get order ${id}`).withRequest(order(`/orders/${id}`)).willRespondWith({ status: 200 });
    }
    const run = provider.executeTest(async (mock) => {
      await fetch(`${mock.url}/orders/1`);
    });
    await expect(run).rejects.toThrow(/Missing request: the interaction 'get order 2' was expected, but not received/);
    await expect(run).rejects.not.toThrow(/'get order 1'/);
    expect(existsSync(join(dir, "Web-Orders.janus.json"))).toBe(false);
  }, 30_000);
});

describe("executeTest", () => {
  it("rethrows the test's own error and writes no contract", async () => {
    const dir = contractDir();
    const failure = new Error("the client broke");
    await expect(
      getOrder(new Pact(options(dir))).executeTest(async (mock) => {
        await fetch(`${mock.url}/orders/1`);
        throw failure;
      }),
    ).rejects.toBe(failure);
    expect(existsSync(join(dir, "Web-Orders.janus.json"))).toBe(false);
  }, 30_000);

  it("fails a test that never called the mock, naming the missing request", async () => {
    const dir = contractDir();
    await expect(getOrder(new Pact(options(dir))).executeTest(async () => undefined)).rejects.toThrow(
      /Test failed for the following reasons:[\s\S]*Missing request: the interaction 'get an order'/,
    );
  }, 30_000);

  it("fails a test whose request the mock did not match", async () => {
    const dir = contractDir();
    await expect(
      getOrder(new Pact(options(dir))).executeTest(async (mock) => {
        await fetch(`${mock.url}/orders/2`);
      }),
    ).rejects.toThrow(/Test failed for the following reasons:[\s\S]*\$\.request\.path/);
  }, 30_000);

  it("resolves with what the test returned", async () => {
    const dir = contractDir();
    const order = await getOrder(new Pact(options(dir))).executeTest(async (mock) => (await fetch(`${mock.url}/orders/1`)).json());
    expect(order).toEqual({ id: 1 });
  }, 30_000);
});

it("merges two suites recording one pair into one contract (ADR 0065)", async () => {
  const dir = contractDir();
  for (const description of ["get an order", "get the order again"]) {
    await getOrder(new Pact(options(dir)), description).executeTest(async (mock) => {
      await fetch(`${mock.url}/orders/1`);
    });
  }
  const contract = JSON.parse(readFileSync(join(dir, "Web-Orders.janus.json"), "utf8")) as { interactions: { description: string }[] };
  expect(contract.interactions.map((i) => i.description)).toEqual(["get an order", "get the order again"]);
}, 30_000);

it("serves a matcher with no example a fresh value, as pact-js's mock does", async () => {
  const dir = contractDir();
  // integer() with no example is 101 with a RandomInt generator, which draws from 0 to 10.
  const served = await new Pact(options(dir))
    .addInteraction()
    .uponReceiving("a random order")
    .withRequest("GET", "/orders/random")
    .willRespondWith(200, (builder) => builder.jsonBody({ id: integer() }))
    .executeTest(async (mock) => (await (await fetch(`${mock.url}/orders/random`)).json()) as { id: number });
  expect(served?.id).toBeLessThanOrEqual(10);
}, 30_000);
