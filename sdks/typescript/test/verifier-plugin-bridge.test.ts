// Where the verifier finds janus-plugin-bridge (behavioural spec 'verify', ADR 0112 decision 5): its
// own option, else `JANUS_PLUGIN_BRIDGE`. The conformance cases cover the option, which every SDK runs;
// the variable is read here, because a case cannot set a process's environment in every language.

import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { Verifier } from "../src/verifier.js";
import { ScriptedPipe } from "./conformance/scripted-engine.js";

const pact = {
  consumer: { name: "web-app" },
  provider: { name: "orders-api" },
  interactions: [{ description: "get an order", request: { method: "GET", path: "/orders/42" }, response: { status: 200 } }],
  metadata: { pactSpecification: { version: "3.0.0" } },
};

async function sentOptions(pluginBridge?: string): Promise<Record<string, unknown>> {
  const file = join(await mkdtemp(join(tmpdir(), "janus-bridge-")), "pact.json");
  await writeFile(file, JSON.stringify(pact), "utf8");
  const pipe = new ScriptedPipe();
  await new Verifier({
    providerUrl: "http://127.0.0.1:8080",
    pacts: [file],
    ...(pluginBridge !== undefined ? { pluginBridge } : {}),
    engine: () => pipe,
  }).verify();
  return pipe.bodyOf("verification/verify")?.options as Record<string, unknown>;
}

describe("the plugin bridge", () => {
  let saved: string | undefined;
  beforeEach(() => {
    saved = process.env.JANUS_PLUGIN_BRIDGE;
    delete process.env.JANUS_PLUGIN_BRIDGE;
  });
  afterEach(() => {
    if (saved === undefined) {
      delete process.env.JANUS_PLUGIN_BRIDGE;
    } else {
      process.env.JANUS_PLUGIN_BRIDGE = saved;
    }
  });

  it("is JANUS_PLUGIN_BRIDGE's when no option names one, resolved against the working directory", async () => {
    process.env.JANUS_PLUGIN_BRIDGE = join("tools", "janus-plugin-bridge");
    expect((await sentOptions())["plugin-bridge"]).toEqual({
      source: { kind: "subprocess", reference: resolve("tools", "janus-plugin-bridge") },
      grants: { env: ["PACT_PLUGIN_DIR", "HOME", "USERPROFILE", "PACT_DO_NOT_TRACK", "RUST_LOG"] },
      limits: { "deadline-ms": 30000 },
    });
  });

  it("is the option's, before JANUS_PLUGIN_BRIDGE's", async () => {
    process.env.JANUS_PLUGIN_BRIDGE = resolve("from-the-environment");
    const bridge = (await sentOptions(resolve("from-the-option")))["plugin-bridge"] as { source: { reference: string } };
    expect(bridge.source.reference).toBe(resolve("from-the-option"));
  });

  it("is not declared when nothing names one, or JANUS_PLUGIN_BRIDGE is empty", async () => {
    expect(await sentOptions()).not.toHaveProperty("plugin-bridge");
    process.env.JANUS_PLUGIN_BRIDGE = "";
    expect(await sentOptions()).not.toHaveProperty("plugin-bridge");
  });
});
