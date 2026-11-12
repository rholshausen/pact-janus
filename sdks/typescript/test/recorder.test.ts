// The shape recorder's own defaults (behavioural spec 'shape-recorder', STYLE.md "Recording a
// provider shape"): what a conformance case cannot say, because every case names its own source.

import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { ShapeRecorder, pathTemplate } from "../src/index.js";
import { useShapeRecorder } from "../src/vitest.js";
import { ScriptedPipe } from "./conformance/scripted-engine.js";

describe("useShapeRecorder", () => {
  const pipe = new ScriptedPipe();
  const dir = join(tmpdir(), `janus-recorder-${process.pid}`);
  const recorder = useShapeRecorder({ provider: "orders-api", shapeDir: dir, evidenceDir: dir, engine: () => pipe });

  it("names the recorder's source after this test file, relative to the working directory", async () => {
    await recorder.observe({ method: "GET", path: pathTemplate("/orders/{id}") }, { status: 200 });
    await recorder.finish();
    expect(pipe.bodyOf("provider-shape-session/finalise")?.write).toMatchObject({ source: "test/recorder.test.ts" });
  });
});

describe("ShapeRecorder", () => {
  it("refuses to finish with no source when no test file says what it is", async () => {
    const pipe = new ScriptedPipe();
    const dir = await mkdtemp(join(tmpdir(), "janus-recorder-"));
    const recorder = new ShapeRecorder({ provider: "orders-api", shapeDir: dir, evidenceDir: dir, engine: () => pipe });
    await recorder.observe({ method: "GET", path: "/orders" }, { status: 200 });
    await expect(recorder.finish()).rejects.toThrow("name the recorder's 'source'");
    expect(pipe.ops).not.toContain("provider-shape-session/finalise");
    expect(pipe.closed).toBe(true);
  });

  it("reads a fetch Response from a clone, its Set-Cookie lines kept apart", async () => {
    const pipe = new ScriptedPipe();
    const dir = await mkdtemp(join(tmpdir(), "janus-recorder-"));
    const recorder = new ShapeRecorder({ provider: "orders-api", source: "s", shapeDir: dir, evidenceDir: dir, engine: () => pipe });
    const headers = new Headers({ "Content-Type": "application/json" });
    headers.append("Set-Cookie", "a=1");
    headers.append("Set-Cookie", "b=2");
    const response = new Response('{"id":"66"}', { status: 200, headers });
    await recorder.observe({ method: "GET", path: "/orders/66" }, response);
    expect(await response.json()).toEqual({ id: "66" });
    expect(pipe.bodyOf("provider-shape-session/observe")?.parts).toEqual({
      response: {
        status: { content: 200 },
        headers: { content: { "content-type": ["application/json"], "set-cookie": ["a=1", "b=2"] } },
        body: { content: Buffer.from('{"id":"66"}').toString("base64"), encoded: "base64", "content-type": "application/json" },
      },
    });
    await recorder.finish();
  });
});
