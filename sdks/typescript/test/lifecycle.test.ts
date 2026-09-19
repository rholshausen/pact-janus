// Session lifecycle and variant iteration (SDK spec §7 categories 2 and 3), against a scripted
// frame pipe: what matters here is which operations the SDK sends, in what order, and what it does
// with what comes back — not what an engine would decide.

import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import type { protocol } from "../src/generated/index.js";
import {
  ContractWithheldError,
  Janus,
  JanusError,
  VariantsFailedError,
  type FramePipe,
  type JanusConfig,
  type Variant,
} from "../src/index.js";

type Reply = { ok: Record<string, unknown> } | { error: protocol.EngineError };

/** A pipe that records every operation and answers from a script, with sensible defaults. */
class ScriptedPipe implements FramePipe {
  readonly ops: string[] = [];
  readonly bodies: Record<string, unknown>[] = [];
  closed = false;
  private readonly script: Partial<Record<string, (body: Record<string, unknown>) => Reply>>;
  private readonly variants: string[];

  constructor(script: ScriptedPipe["script"] = {}, variants: string[] = ["base"]) {
    this.script = script;
    this.variants = variants;
  }

  async exchange(frame: protocol.RequestFrame): Promise<protocol.ResponseFrame> {
    this.ops.push(frame.op);
    this.bodies.push(frame.body);
    const reply = this.script[frame.op]?.(frame.body) ?? this.defaultReply(frame.op);
    return { type: "response", id: frame.id, ...reply };
  }

  private defaultReply(op: string): Reply {
    switch (op) {
      case "engine/hello":
        return { ok: { "protocol-version": 1 } };
      case "consumer-session/create":
        return { ok: { session: "s-1" } };
      case "consumer-session/add-interaction":
        return { ok: { handle: `i-${this.ops.filter((o) => o === op).length}` } };
      case "consumer-session/start-transport":
        return { ok: { endpoint: { kind: "http", "base-url": "http://127.0.0.1:9999" } } };
      case "consumer-session/variants":
        return { ok: { variants: this.variants.map((id) => ({ id, label: id === "base" ? "base" : `label of ${id}` })) } };
      case "consumer-session/finalise":
        return { ok: { results: [{ handle: "i-1", status: "verified" }], contract: { $format: "janus-contract/1" } } };
      default:
        return { ok: {} };
    }
  }

  async close(): Promise<void> {
    this.closed = true;
  }
}

function janusOver(pipe: FramePipe, config: Partial<JanusConfig> = {}): Janus {
  return new Janus({ consumer: "web-app", provider: "orders-api", engine: () => pipe, ...config });
}

const noop = () => undefined;

describe("execute", () => {
  it("makes no call until execute (session.janus.no-call-until-execute, session.interaction.no-call-until-execute)", () => {
    const pipe = new ScriptedPipe();
    janusOver(pipe).interaction("x").given("s").request({ method: "GET" });
    expect(pipe.ops).toEqual([]);
  });

  it("sends the full call sequence, offering protocol version 1 (session.execute.full-call-sequence, session.janus.hello-offers-v1)", async () => {
    const pipe = new ScriptedPipe();
    const janus = janusOver(pipe);
    await janus.execute(janus.interaction("x"), noop);
    expect(pipe.ops).toEqual([
      "engine/hello",
      "consumer-session/create",
      "consumer-session/add-interaction",
      "consumer-session/start-transport",
      "consumer-session/variants",
      "consumer-session/serve-variant",
    ]);
    expect(pipe.bodies[0]).toMatchObject({ "protocol-versions": [1] });
    expect(pipe.bodies[1]).toEqual({ config: { consumer: { name: "web-app" }, provider: { name: "orders-api" } } });
    expect(pipe.bodies[5]).toEqual({ session: "s-1", handle: "i-1", variant: "base" });
  });

  it("opens one engine, one session and one transport per suite (session.execute.one-session-per-suite)", async () => {
    const pipe = new ScriptedPipe();
    const janus = janusOver(pipe);
    await janus.execute(janus.interaction("first"), noop);
    await janus.execute(janus.interaction("second"), noop);
    const count = (op: string) => pipe.ops.filter((o) => o === op).length;
    expect([count("engine/hello"), count("consumer-session/create"), count("consumer-session/start-transport")]).toEqual([1, 1, 1]);
    expect(count("consumer-session/add-interaction")).toBe(2);
  });

  it("runs the closure once per selected variant, in order, with the mock and the descriptor (session.execute.closure-per-selected-variant)", async () => {
    const pipe = new ScriptedPipe({}, ["base", "a", "b"]);
    const janus = janusOver(pipe);
    const seen: Array<[string, string]> = [];
    await janus.execute(janus.interaction("x"), (mock, variant) => {
      seen.push([mock.url, variant.id]);
    });
    expect(seen).toEqual([
      ["http://127.0.0.1:9999", "base"],
      ["http://127.0.0.1:9999", "a"],
      ["http://127.0.0.1:9999", "b"],
    ]);
    // serve-variant arms each variant immediately before its closure runs.
    expect(pipe.ops.filter((o) => o === "consumer-session/serve-variant")).toHaveLength(3);
  });

  it("still runs the closure for a one-variant selection (session.execute.single-variant-still-runs)", async () => {
    const pipe = new ScriptedPipe({}, ["base"]);
    const janus = janusOver(pipe);
    let runs = 0;
    await janus.execute(janus.interaction("x"), () => {
      runs++;
    });
    expect(runs).toBe(1);
  });

  it("runs every variant after a failure, then fails naming each failed one (session.execute.failing-variant-fails-build, session.execute.every-variant-runs-after-a-failure)", async () => {
    const pipe = new ScriptedPipe({}, ["base", "a", "b"]);
    const janus = janusOver(pipe);
    const ran: string[] = [];
    const run = janus.execute(janus.interaction("get an order"), async (_mock, variant: Variant) => {
      ran.push(variant.id);
      if (variant.id !== "a") {
        throw new Error(`boom in ${variant.id}`);
      }
    });
    const error = await run.catch((e: unknown) => e);
    expect(ran).toEqual(["base", "a", "b"]);
    expect(error).toBeInstanceOf(VariantsFailedError);
    const failed = error as VariantsFailedError;
    expect(failed.failures.map((f) => f.variant.id)).toEqual(["base", "b"]);
    expect(failed.message).toContain("2 of 3 variants failed for 'get an order'");
    expect(failed.message).toContain("✗ label of b (b)");
    expect(failed.message).toContain("Error: boom in b");
  });

  it("surfaces interaction-invalid with the engine's problems verbatim, before any variant runs", async () => {
    const problems = [{ pointer: "/parts/response/body/alternatives", message: "not disjoint (spec §5.4)" }];
    const pipe = new ScriptedPipe({
      "consumer-session/add-interaction": () => ({
        error: { code: "interaction-invalid", category: "document", message: "interaction specification is not valid", details: { problems } },
      }),
    });
    const janus = janusOver(pipe);
    let ran = false;
    const error = await janus
      .execute(janus.interaction("get an order"), () => {
        ran = true;
      })
      .catch((e: unknown) => e);
    expect(ran).toBe(false);
    expect(error).toBeInstanceOf(JanusError);
    expect((error as JanusError).code).toBe("interaction-invalid");
    expect((error as JanusError).problems).toEqual(problems);
    expect((error as JanusError).message).toContain("'get an order': interaction specification is not valid");
    expect((error as JanusError).message).toContain("/parts/response/body/alternatives: not disjoint");
  });
});

describe("finalise", () => {
  it("runs after a failed execute, and ends the engine (session.finalise.always-runs)", async () => {
    const pipe = new ScriptedPipe();
    const janus = janusOver(pipe, { contractDir: await mkdtemp(join(tmpdir(), "janus-")) });
    await janus.execute(janus.interaction("x"), () => {
      throw new Error("closure failed");
    }).catch(() => undefined);
    await expect(janus.finalise()).rejects.toThrow(ContractWithheldError);
    expect(pipe.ops.at(-1)).toBe("consumer-session/finalise");
    expect(pipe.closed).toBe(true);
  });

  it("writes no contract when a test in the suite failed, even though the engine verified every exchange (session.finalise.failed-test-withholds-contract)", async () => {
    // The engine returned a contract: every request matched. But the closure crashed on what the
    // mock sent, and a contract must not claim a variant the consumer's own test failed on.
    const pipe = new ScriptedPipe({}, ["base", "response.body.shippedAt#presence=absent"]);
    const dir = await mkdtemp(join(tmpdir(), "janus-"));
    const janus = janusOver(pipe, { contractDir: dir });
    await janus.execute(janus.interaction("get an order"), (_mock, variant) => {
      if (variant.id !== "base") {
        throw new TypeError("Cannot read properties of undefined (reading 'shippedAt')");
      }
    }).catch(() => undefined);
    const error = await janus.finalise().catch((e: unknown) => e);
    expect(error).toBeInstanceOf(ContractWithheldError);
    expect((error as ContractWithheldError).failedTests).toEqual(["get an order"]);
    expect((error as Error).message).toContain("get an order: its test failed");
    await expect(readFile(join(dir, "web-app-orders-api.janus.json"))).rejects.toThrow();
  });

  it("writes the contract the engine returned, compact and unaltered, with one trailing LF (session.finalise.writes-contract-when-verified, session.finalise.contract-bytes-unaltered)", async () => {
    const contract = { $format: "janus-contract/1", consumer: { name: "web-app" }, zeta: 1, alpha: [2, 1] };
    const pipe = new ScriptedPipe({
      "consumer-session/finalise": () => ({ ok: { results: [{ handle: "i-1", status: "verified" }], contract } }),
    });
    const dir = await mkdtemp(join(tmpdir(), "janus-"));
    const janus = janusOver(pipe, { contractDir: dir });
    await janus.execute(janus.interaction("x"), noop);
    const { contractFile } = await janus.finalise();
    expect(contractFile).toBe(join(dir, "web-app-orders-api.janus.json"));
    expect(await readFile(contractFile ?? "", "utf8")).toBe(
      '{"$format":"janus-contract/1","consumer":{"name":"web-app"},"zeta":1,"alpha":[2,1]}\n',
    );
  });

  it("fails naming every unverified variant, and writes nothing, when the engine withholds the contract (session.finalise.withheld-contract-fails-build)", async () => {
    const pipe = new ScriptedPipe({
      "consumer-session/finalise": () => ({
        ok: {
          results: [
            {
              handle: "i-1",
              status: "failed",
              variants: [
                { variant: "base", status: "verified" },
                {
                  variant: "response.body.shippedAt#presence=absent",
                  status: "failed",
                  mismatches: [{ path: "$.request.headers.accept", message: "Expected <absent> to equal [\"application/json\"]" }],
                },
                { variant: "response.body.status#value=SHIPPED", status: "not-exercised" },
              ],
            },
          ],
        },
      }),
    });
    const dir = await mkdtemp(join(tmpdir(), "janus-"));
    const janus = janusOver(pipe, { contractDir: dir });
    await janus.execute(janus.interaction("get an order"), noop);
    const error = await janus.finalise().catch((e: unknown) => e);
    expect(error).toBeInstanceOf(ContractWithheldError);
    const message = (error as Error).message;
    expect(message).toContain("get an order: failed");
    expect(message).toContain("✗ response.body.shippedAt#presence=absent: failed");
    expect(message).toContain("$.request.headers.accept Expected <absent>");
    expect(message).toContain("✗ response.body.status#value=SHIPPED: not-exercised");
    expect(message).not.toContain("✗ base");
    await expect(readFile(join(dir, "web-app-orders-api.janus.json"))).rejects.toThrow();
    expect(pipe.closed).toBe(true);
  });

  it("does nothing but end the engine when nothing was executed", async () => {
    const pipe = new ScriptedPipe();
    const janus = janusOver(pipe);
    expect(await janus.finalise()).toEqual({ results: [] });
    expect(pipe.ops).toEqual([]);
  });
});
