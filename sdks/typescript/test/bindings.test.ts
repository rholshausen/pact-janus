// The generated bindings (plan task 6.1) are only worth having if they are the protocol's own
// types: these tests pin the properties spike 1.1's bindings round found and the SDK relies on.

import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import { contract, protocol, protocolVocabulary, shape } from "../src/generated/index.js";

const specs = new URL("../../../Documentation/specs/", import.meta.url);

describe("generated bindings", () => {
  it("name every operation the frame schema knows", async () => {
    const frame = JSON.parse(
      await readFile(new URL("engine-protocol/schemas/v1/frame.schema.json", specs), "utf8"),
    );
    const known: string[] = frame.$defs.RequestFrame.properties.op["x-known-values"];
    expect(Object.values(protocolVocabulary.RequestFrameOp)).toEqual(known);
    expect(protocolVocabulary.RequestFrameOp.ConsumerSessionCreate).toBe("consumer-session/create");
  });

  it("type a request frame, and reject one missing a required member", () => {
    const create: protocol.Create = { config: { consumer: { name: "web" }, provider: { name: "orders" } } };
    const frame: protocol.RequestFrame = {
      type: "request",
      id: "r-1",
      op: protocolVocabulary.RequestFrameOp.ConsumerSessionCreate,
      body: create,
    };
    // @ts-expect-error — 'op' is required (engine-protocol spec §4)
    const missingOp: protocol.RequestFrame = { type: "request", id: "r-2", body: {} };
    expect([frame, missingOp]).toHaveLength(2);
  });

  it("keep members they do not know, so a newer engine's additions survive a round trip", () => {
    const wire = '{"code":"interaction-invalid","message":"bad","retryable":false}';
    const error = JSON.parse(wire) as protocol.EngineError;
    expect(error.retryable).toBe(false);
    expect(JSON.stringify(error)).toBe(wire);
  });

  it("type the interaction specification an SDK builds, with shapes in its parts", () => {
    const status: shape.Shape = { shape: "any-of", options: ["PENDING", "SHIPPED"], example: "PENDING" };
    const interaction: contract.InteractionSpec = {
      description: "get an order",
      states: [{ name: "an order exists", params: { id: 1 } }],
      parts: { response: { body: { shape: "object", members: { status } } } },
    };
    expect(interaction.parts.response?.body).toBeDefined();
  });
});
