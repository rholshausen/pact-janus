// Session lifecycle and variant iteration (SDK spec §7 categories 2 and 3), against a scripted
// frame pipe: what matters here is which operations the SDK sends, in what order, and what it does
// with what comes back — not what an engine would decide.

import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { describe, expect, it, vi } from "vitest";
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
    const reply = this.script[frame.op]?.(frame.body) ?? this.defaultReply(frame.op, frame.body);
    return { type: "response", id: frame.id, ...reply };
  }

  /** The variants reported failed, which withhold the contract as the engine would (ADR 0064). */
  private readonly failedReports: string[] = [];

  private defaultReply(op: string, body: Record<string, unknown>): Reply {
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
      case "consumer-session/report-variant": {
        const test = { status: body.status, ...(body.reason === undefined ? {} : { reason: body.reason }) };
        if (body.status !== "passed") {
          this.failedReports.push(String(body.variant));
        }
        return { ok: { result: { variant: body.variant, status: body.status === "passed" ? "verified" : "failed", test } } };
      }
      case "consumer-session/finalise":
        if (this.failedReports.length > 0) {
          return {
            ok: {
              results: [{ handle: "i-1", status: "failed", variants: this.failedReports.map((variant) => ({ variant, status: "failed" })) }],
              refused: [],
            },
          };
        }
        return { ok: { results: [{ handle: "i-1", status: "verified" }], "contract-text": '{"$format":"janus-contract/1"}\n' } };
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
      "consumer-session/report-variant",
    ]);
    expect(pipe.bodies[0]).toMatchObject({ "protocol-versions": [1], capabilities: { "contract-text": {}, "test-verdicts": {} } });
    expect(pipe.bodies[1]).toEqual({ events: true, config: { consumer: { name: "web-app" }, provider: { name: "orders-api" } } });
    expect(pipe.bodies[5]).toEqual({ session: "s-1", handle: "i-1", variant: "base" });
    expect(pipe.bodies[6]).toEqual({ session: "s-1", handle: "i-1", variant: "base", status: "passed" });
  });

  it("drains the session's stream while a closure runs, and logs each exchange (session.execute.logs-exchange-events)", async () => {
    const told = [
      { kind: "consumer-session/exchange", payload: { transport: "t-1", handle: "i-1", variant: "a", status: "mismatched", mismatches: [{ path: "$.request.path", message: "Expected '/orders/7' to equal '/orders/42'" }] } },
      { kind: "consumer-session/exchange", payload: { transport: "t-1", handle: "i-1", variant: "a", status: "matched" } },
      { kind: "consumer-session/exchange", payload: { transport: "t-1", status: "unmatched" } },
    ];
    let polls = 0;
    const pipe = new ScriptedPipe(
      {
        "consumer-session/create": () => ({ ok: { session: "s-1", stream: "s-9" } }),
        "events/poll": (body) => {
          expect(body).toEqual({ streams: ["s-9"], "wait-ms": 0 });
          polls += 1;
          const events = told.splice(0, 1).map((event, i) => ({ stream: "s-9", seq: polls + i, ...event }));
          return { ok: { events } };
        },
      },
      ["a"],
    );
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const debug = vi.spyOn(console, "debug").mockImplementation(() => undefined);
    try {
      await janusOver(pipe, { logLevel: "debug" }).execute(janusOver(pipe).interaction("get an order"), async () => {
        // Long enough for the polls the closure's run makes to say what the mock did, before it ends.
        await new Promise((resolve) => setTimeout(resolve, 250));
        expect(warn).toHaveBeenCalledWith(
          "pact-janus: 'get an order' [label of a [a]]: the request did not match\n    $.request.path: Expected '/orders/7' to equal '/orders/42'",
        );
      });
      expect(debug).toHaveBeenCalledWith("pact-janus: 'get an order' [label of a [a]]: the request matched");
      expect(warn).toHaveBeenCalledWith("pact-janus: a request arrived with no variant armed for it, and the mock refused it");
      // The last drain comes before the variant is reported.
      expect(pipe.ops.lastIndexOf("events/poll")).toBeLessThan(pipe.ops.indexOf("consumer-session/report-variant"));
    } finally {
      warn.mockRestore();
      debug.mockRestore();
    }
  });

  it("asks for the session's stream, and polls nothing when the engine answers with none (session.execute.asks-for-exchange-events)", async () => {
    const pipe = new ScriptedPipe();
    await janusOver(pipe).execute(janusOver(pipe).interaction("x"), () => new Promise((resolve) => setTimeout(resolve, 150)));
    expect(pipe.bodies[pipe.ops.indexOf("consumer-session/create")]).toMatchObject({ events: true });
    expect(pipe.ops).not.toContain("events/poll");
  });

  it("logs nothing below the test's log level, and a failed poll is not the closure's verdict", async () => {
    const pipe = new ScriptedPipe({
      "consumer-session/create": () => ({ ok: { session: "s-1", stream: "s-9" } }),
      "events/poll": () => ({ error: { code: "stream-not-found", category: "request", message: "no stream 's-9'" } }),
    });
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    try {
      await janusOver(pipe, { logLevel: "error" }).execute(janusOver(pipe).interaction("x"), noop);
      expect(warn).not.toHaveBeenCalled();
      await janusOver(pipe).execute(janusOver(pipe).interaction("x"), noop);
      expect(warn).toHaveBeenCalledWith(expect.stringContaining("could not read the mock's exchanges"));
      expect(pipe.ops.filter((op) => op === "events/poll")).toHaveLength(2);
    } finally {
      warn.mockRestore();
    }
  });

  it("reports each closure's verdict once it ends, with what a failing one threw (session.execute.reports-each-variant)", async () => {
    const pipe = new ScriptedPipe({}, ["base", "a"]);
    const janus = janusOver(pipe);
    await janus
      .execute(janus.interaction("x"), (_mock, variant) => {
        if (variant.id === "a") {
          throw new TypeError("shippedAt is undefined");
        }
      })
      .catch(() => undefined);
    const ops = pipe.ops.slice(pipe.ops.indexOf("consumer-session/serve-variant"));
    expect(ops).toEqual([
      "consumer-session/serve-variant",
      "consumer-session/report-variant",
      "consumer-session/serve-variant",
      "consumer-session/report-variant",
    ]);
    const reports = pipe.bodies.filter((_, i) => pipe.ops[i] === "consumer-session/report-variant");
    expect(reports).toEqual([
      { session: "s-1", handle: "i-1", variant: "base", status: "passed" },
      { session: "s-1", handle: "i-1", variant: "a", status: "failed", reason: "TypeError: shippedAt is undefined" },
    ]);
  });

  it("fails the variant the engine did not verify, though its closure passed (session.execute.engine-verdict-fails-the-variant)", async () => {
    const pipe = new ScriptedPipe(
      {
        "consumer-session/report-variant": (body) => ({
          ok: {
            result:
              body.variant === "a"
                ? { variant: "a", status: "failed", mismatches: [{ path: "$.request.path", message: "Expected '/orders/7' to equal '/orders/42'" }] }
                : body.variant === "b"
                  ? { variant: "b", status: "not-exercised" }
                  : { variant: body.variant, status: "verified" },
          },
        }),
      },
      ["base", "a", "b"],
    );
    const janus = janusOver(pipe);
    const ran: string[] = [];
    const error = await janus
      .execute(janus.interaction("get an order"), (_mock, variant) => {
        ran.push(variant.id);
      })
      .catch((e: unknown) => e);
    expect(ran).toEqual(["base", "a", "b"]);
    expect(error).toBeInstanceOf(VariantsFailedError);
    const failed = error as VariantsFailedError;
    expect(failed.failures.map((f) => [f.variant.id, f.result.status, f.cause])).toEqual([
      ["a", "failed", undefined],
      ["b", "not-exercised", undefined],
    ]);
    expect(failed.message).toContain("$.request.path Expected '/orders/7' to equal '/orders/42'");
    expect(failed.message).toContain("nothing exercised the mock under this variant");
  });

  it("ends the loop at once when the engine cannot take a report (session.execute.engine-error-aborts-the-loop)", async () => {
    const pipe = new ScriptedPipe(
      {
        "consumer-session/report-variant": () => ({ error: { code: "session-not-found", category: "session", message: "no such session: s-1" } }),
      },
      ["base", "a"],
    );
    const janus = janusOver(pipe);
    const ran: string[] = [];
    const error = await janus
      .execute(janus.interaction("x"), (_mock, variant) => {
        ran.push(variant.id);
      })
      .catch((e: unknown) => e);
    expect(ran).toEqual(["base"]);
    expect((error as JanusError).code).toBe("session-not-found");
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

  it("writes no contract when a test in the suite failed, because the engine withheld it (session.finalise.failed-test-withholds-contract)", async () => {
    // Every request matched. But the closure crashed on what the mock sent: the SDK reported it, and
    // the engine withholds a contract claiming a variant the consumer's own test failed on.
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
    expect((error as ContractWithheldError).interactions).toEqual(["get an order"]);
    expect((error as Error).message).toContain("get an order: failed");
    await expect(readFile(join(dir, "web-app-orders-api.janus.json"))).rejects.toThrow();
  });

  it("names the contract file, which the engine writes, merging into it (session.finalise.writes-contract-when-verified, session.finalise.names-contract-file)", async () => {
    const pipe = new ScriptedPipe({
      "consumer-session/finalise": (body) => ({
        ok: { results: [{ handle: "i-1", status: "verified" }], written: (body.write as { path: string }).path },
      }),
    });
    const dir = await mkdtemp(join(tmpdir(), "janus-"));
    const janus = janusOver(pipe, { contractDir: dir });
    await janus.execute(janus.interaction("x"), noop);
    const { contractFile } = await janus.finalise();
    const path = join(dir, "web-app-orders-api.janus.json");
    expect(pipe.bodies[pipe.ops.indexOf("consumer-session/finalise")]?.write).toEqual({ path, mode: "merge" });
    expect(contractFile).toBe(path);
    // The engine's host writes the file (ADR 0065); this engine wrote none, and nor did the SDK.
    await expect(readFile(path)).rejects.toThrow();
    expect(pipe.bodies[pipe.ops.indexOf("engine/hello")]?.capabilities).toEqual({ "contract-text": {}, "test-verdicts": {} });
  });

  it("asks the engine to overwrite the file when told to (session.finalise.names-contract-file)", async () => {
    const pipe = new ScriptedPipe({
      "consumer-session/finalise": (body) => ({
        ok: { results: [{ handle: "i-1", status: "verified" }], written: (body.write as { path: string }).path },
      }),
    });
    const janus = janusOver(pipe, { contractDir: "relative/contracts", overwriteContract: true });
    await janus.execute(janus.interaction("x"), noop);
    await janus.finalise();
    expect(pipe.bodies[pipe.ops.indexOf("consumer-session/finalise")]?.write).toEqual({
      path: resolve("relative/contracts", "web-app-orders-api.janus.json"),
      mode: "overwrite",
    });
  });

  it("fails, saying the engine is too old, when the engine answers the contract instead of writing it", async () => {
    const pipe = new ScriptedPipe({
      "consumer-session/finalise": () => ({ ok: { results: [{ handle: "i-1", status: "verified" }], "contract-text": "{}\n" } }),
    });
    const dir = await mkdtemp(join(tmpdir(), "janus-"));
    const janus = janusOver(pipe, { contractDir: dir });
    await janus.execute(janus.interaction("x"), noop);
    const error = await janus.finalise().catch((e: unknown) => e);
    expect(error).not.toBeInstanceOf(ContractWithheldError);
    expect(String(error)).toContain("predates 'contract-writing'");
    await expect(readFile(join(dir, "web-app-orders-api.janus.json"), "utf8")).rejects.toThrow();
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

  it("names the interactions the engine refused, and writes nothing (session.finalise.refused-interaction-withholds-contract)", async () => {
    const pipe = new ScriptedPipe({
      "consumer-session/finalise": () => ({
        ok: {
          results: [{ handle: "i-1", status: "verified" }],
          refused: [{ description: "a broken one", code: "interaction-invalid", message: "interaction specification is not valid" }],
        },
      }),
    });
    const dir = await mkdtemp(join(tmpdir(), "janus-"));
    const janus = janusOver(pipe, { contractDir: dir });
    await janus.execute(janus.interaction("get an order"), noop);
    const error = await janus.finalise().catch((e: unknown) => e);
    expect(error).toBeInstanceOf(ContractWithheldError);
    expect((error as ContractWithheldError).interactions).toEqual(["a broken one"]);
    expect((error as Error).message).toContain("a broken one: refused, interaction specification is not valid [interaction-invalid]");
    await expect(readFile(join(dir, "web-app-orders-api.janus.json"))).rejects.toThrow();
  });

  it("does nothing but end the engine when nothing was executed", async () => {
    const pipe = new ScriptedPipe();
    const janus = janusOver(pipe);
    expect(await janus.finalise()).toEqual({ results: [] });
    expect(pipe.ops).toEqual([]);
  });
});
