// Plan task 8.1 from the consumer's side: a consumer that reads orders as CSV, tested through this
// SDK against the real engine, with the body handled by a third-party content component the
// project declares — `third-party/janus-csv`, built here to wasm32-wasip2 and loaded by the engine
// from its path. Nothing in the SDK knows what CSV is: it declares the body's type and the
// component, and the engine does the rest.

import { execFileSync } from "node:child_process";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, relative, resolve } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import { Janus, JanusError, content, eachLike, regex, string, type ComponentDeclaration } from "../src/index.js";

const repoRoot = resolve(import.meta.dirname, "../../..");
const componentDir = join(repoRoot, "third-party/janus-csv");

/** The consumer's own code: a report built from the provider's CSV export. */
class OrderReport {
  readonly baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  /** How many items the orders in the export hold between them. */
  async totalItems(): Promise<number> {
    const response = await fetch(`${this.baseUrl}/orders.csv`, { headers: { Accept: "text/csv" } });
    if (!response.ok) {
      throw new Error(`GET /orders.csv: ${response.status} ${await response.text()}`);
    }
    if (!response.headers.get("content-type")?.startsWith("text/csv")) {
      throw new Error(`expected CSV, got ${response.headers.get("content-type")}`);
    }
    const [header = [], ...rows] = (await response.text()).trim().split(/\r?\n/).map((line) => line.split(","));
    const items = header.indexOf("items");
    if (items < 0) {
      throw new Error(`no 'items' column in ${header.join(",")}`);
    }
    return rows.reduce((total, row) => total + Number.parseInt(row[items] ?? "", 10), 0);
  }
}

const exportInteraction = (janus: Janus) =>
  janus
    .interaction("the orders export")
    .request({ method: "GET", path: "/orders.csv" })
    .response({
      status: 200,
      // Strings, every field: CSV has no other type, and the component says so (its `string-only`
      // degradation). A count is the text that spells one. One row, because this consumer's test
      // sets up no state that would make the provider export more.
      body: content(
        "text/csv",
        eachLike({ id: string("66"), status: string("PENDING"), items: regex("^[0-9]+$", "2") }, { min: 1, max: 1 }),
      ),
    });

describe("a CSV consumer, through a third-party content component", () => {
  let csv: ComponentDeclaration;

  beforeAll(() => {
    execFileSync("cargo", ["build", "--release", "--target", "wasm32-wasip2"], { cwd: componentDir, stdio: "inherit" });
    // Relative, as a project would write it: the SDK resolves it against the working directory.
    csv = {
      name: "csv",
      source: { kind: "file", reference: relative(process.cwd(), join(componentDir, "target/wasm32-wasip2/release/janus_csv.wasm")) },
    };
  }, 300_000);

  it("serves every variant as CSV and writes a contract that declares the type", async () => {
    const contractDir = await mkdtemp(join(tmpdir(), "janus-csv-"));
    const janus = new Janus({ consumer: "reporting", provider: "order-service", contractDir, components: [csv] });

    await janus.execute(exportInteraction(janus), async (mock) => {
      expect(await new OrderReport(mock.url).totalItems()).toBeGreaterThan(0);
    });
    const { contractFile } = await janus.finalise();

    const contract = JSON.parse(await readFile(contractFile as string, "utf8")) as {
      interactions: { "content-types": unknown; selection: { variants: { parts: { response: { body: unknown } } }[] } }[];
    };
    const interaction = contract.interactions[0];
    if (!interaction) {
      throw new Error("the contract records no interaction");
    }
    expect(interaction["content-types"]).toEqual({ response: { body: "text/csv" } });
    for (const variant of interaction.selection.variants) {
      expect(variant.parts.response.body).toMatchObject({ "content-type": "text/csv" });
    }
  });

  it("without the component, the engine refuses the interaction by name", async () => {
    const janus = new Janus({ consumer: "reporting", provider: "order-service", contractDir: await mkdtemp(join(tmpdir(), "janus-csv-")) });
    const refused = janus.execute(exportInteraction(janus), () => undefined);
    await expect(refused).rejects.toBeInstanceOf(JanusError);
    await expect(refused).rejects.toMatchObject({ code: "component-unavailable" });
    await janus.finalise().catch(() => undefined);
  });
});
