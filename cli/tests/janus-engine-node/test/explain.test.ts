// Staged plan S1.12, ADR 0050: `verification/explain` compiles what a run would compile, and runs
// it against captured values, over the protocol alone. The CSV component (third-party/janus-csv) is
// declared in the request exactly as a run declares it, so the plan carries its fragment, and the
// fragment's own action, `csv:integer`, is applied by the component when the values run. Where there
// is no Rust toolchain, JANUS_CSV_COMPONENT names a janus_csv.wasm built elsewhere.

import { beforeAll, describe, expect, it } from "vitest";
import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { EngineClient } from "./engine-client.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../../../..");
const binaryName = process.platform === "win32" ? "janus-engine.exe" : "janus-engine";
const binaryPath = path.join(repoRoot, "target", "debug", binaryName);
const componentDir = path.join(repoRoot, "third-party", "janus-csv");

/** A contract whose response body is `text/csv` rows, each with an integer `items`. */
function csvContract() {
  return {
    $format: "janus-contract/1",
    consumer: { name: "reporting" },
    provider: { name: "order-service" },
    interactions: [
      {
        description: "the orders export",
        transport: { kind: "http", mode: "passive" },
        parts: {
          request: {
            method: { shape: "equality", example: "GET" },
            path: { shape: "equality", example: "/orders.csv" },
          },
          response: {
            body: {
              shape: "each-like",
              min: 1,
              items: {
                shape: "object",
                members: { id: { shape: "string", example: "66" }, items: { shape: "integer", example: 1 } },
              },
            },
          },
        },
        "content-types": { response: { body: "text/csv" } },
        requires: [{ component: "content/csv", "min-version": 1 }],
        selection: {
          variants: [
            {
              id: "base",
              origin: "base",
              assignment: [],
              parts: { request: { method: { content: "GET" }, path: { content: "/orders.csv" } } },
            },
          ],
          report: {},
        },
      },
    ],
  };
}

describe("janus-engine subprocess: verification/explain with components and values", () => {
  let csv: Record<string, unknown>;

  beforeAll(() => {
    execFileSync("cargo", ["build", "-p", "pact_janus_cli", "--bin", "janus-engine"], { cwd: repoRoot, stdio: "inherit" });
    let built = process.env.JANUS_CSV_COMPONENT;
    if (!built) {
      execFileSync("cargo", ["build", "--release", "--target", "wasm32-wasip2"], { cwd: componentDir, stdio: "inherit" });
      built = path.join(componentDir, "target", "wasm32-wasip2", "release", "janus_csv.wasm");
    }
    csv = { name: "csv", source: { kind: "file", reference: path.resolve(built) } };
  }, 300_000);

  it("compiles the component's fragment and runs captured values through its action", async () => {
    const engine = new EngineClient(binaryPath);
    try {
      await engine.send("engine/hello", {
        "protocol-versions": [1],
        host: { name: "janus-engine-node-test", version: "0.0.0" },
        capabilities: {},
      });
      const interaction = { kind: "contract-interaction", contract: csvContract() };

      // With nothing declared, explain refuses the interaction, as a run would.
      const refused = await engine.send("verification/explain", { interaction });
      expect((refused.error as { code: string }).code).toBe("component-unavailable");

      const values = {
        "$.request.method": "GET",
        "$.request.path": "/orders.csv",
        "$.response.body": [{ id: "66", items: "x" }],
      };
      const explained = await engine.send("verification/explain", { interaction, components: [csv], values });
      const ok = explained.ok as { text: string; executed: { status: string; mismatches: { action: string }[]; text: string } };
      expect(ok.text).toContain("csv:integer");
      expect(ok.executed.status).toBe("mismatched");
      expect(ok.executed.mismatches.map((mismatch) => mismatch.action)).toEqual(["csv:integer"]);
      expect(ok.executed.text).toContain("expected text that spells an integer");

      values["$.response.body"] = [{ id: "66", items: "12" }];
      const matched = await engine.send("verification/explain", { interaction, components: [csv], values });
      expect((matched.ok as { executed: { status: string } }).executed.status).toBe("matched");
    } finally {
      await engine.close();
    }
  }, 60_000);
});
