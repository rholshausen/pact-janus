// Runs a recorder case (README §3): a shape recorder built from the case's data, each answer
// observed in order and the recorder finished, against the case's scripted engine or, for a live
// case, the real engine. What this file knows is how TypeScript spells the primitives; what they must
// do is the case's.

import { existsSync } from "node:fs";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { JanusError, ShapeRecorder } from "../../src/index.js";
import type { Case, Outcome } from "./cases.js";
import { equal, show } from "./compare.js";
import { template } from "./dsl.js";
import { checkEngine } from "./provider.js";
import { PROVIDER, ScriptedPipe } from "./scripted-engine.js";

export async function runRecorder(testCase: Case, failures: string[]): Promise<void> {
  const live = testCase.category === "live";
  const pipe = live ? undefined : new ScriptedPipe(testCase.engine ?? {});
  const script = testCase.recorder ?? {};
  const expected = testCase.expect ?? {};
  const dir = await mkdtemp(join(tmpdir(), "janus-recorder-"));
  const shapeDir = join(dir, "shapes");
  const evidenceDir = join(dir, "evidence");
  const recorder = new ShapeRecorder({
    provider: PROVIDER,
    shapeDir,
    evidenceDir,
    ...(script.source !== undefined ? { source: script.source } : {}),
    ...(script.policy !== undefined ? { policy: script.policy } : {}),
    ...(pipe ? { engine: () => pipe } : {}),
  });

  const observed: ("ok" | Outcome)[] = [];
  for (const observation of script.observations ?? []) {
    const { status, headers, body } = observation.response;
    const operation = Object.fromEntries(Object.entries(observation.operation).map(([slot, value]) => [slot, template(value)]));
    const octets = body?.bytes !== undefined ? new Uint8Array(Buffer.from(body.bytes, "base64")) : body?.text;
    observed.push(
      await outcome(
        () =>
          recorder.observe(
            operation,
            { status, ...(headers ? { headers } : {}), ...(octets !== undefined ? { body: octets } : {}) },
            {
              ...(observation.description !== undefined ? { description: observation.description } : {}),
              ...(observation.states !== undefined ? { states: observation.states } : {}),
              ...(observation.request !== undefined ? { request: observation.request } : {}),
            },
          ),
        pipe,
      ),
    );
  }
  const written = script.finish === false ? undefined : await recorder.finish();

  if (expected.observed !== undefined && !equal(expected.observed, observed)) {
    failures.push(`observe ended differently\n  expected: ${show(expected.observed)}\n  actual:   ${show(observed)}`);
  }
  await checkEngine(expected, pipe, failures);
  const shapeFile = join(shapeDir, `${PROVIDER}.provider-shape.json`);
  if (expected.written !== undefined) {
    if (expected.written !== (written !== undefined)) {
      failures.push(`finish answered ${show(written)}, and the case expects ${expected.written ? "a path" : "none"}`);
    } else if (expected.written && (written !== shapeFile || !existsSync(shapeFile))) {
      failures.push(`finish answered ${show(written)}, and the case expects the file ${show(shapeFile)}`);
    }
    const evidence = (pipe?.bodyOf("provider-shape-session/finalise")?.write as { evidence?: string } | undefined)?.evidence;
    if (expected.written && pipe && evidence !== join(evidenceDir, `${PROVIDER}.json`)) {
      failures.push(`finalise named the evidence ${show(evidence)}, and the case expects ${show(join(evidenceDir, `${PROVIDER}.json`))}`);
    }
  }
  if (expected.shape !== undefined) {
    const shape = existsSync(shapeFile) ? (JSON.parse(await readFile(shapeFile, "utf8")) as unknown) : undefined;
    if (!equal(expected.shape, shape)) {
      failures.push(`the shape written differs\n  expected: ${show(expected.shape)}\n  actual:   ${show(shape)}`);
    }
  }
}

/** This SDK's ways of ending `observe`, as the case's outcomes (README §4). */
async function outcome(run: () => Promise<unknown>, pipe: ScriptedPipe | undefined): Promise<"ok" | Outcome> {
  const sent = pipe?.ops.length ?? 0;
  try {
    await run();
    return "ok";
  } catch (error) {
    if (error instanceof JanusError) {
      return { error: "engine-error", code: error.code };
    }
    if (error instanceof TypeError && (pipe?.ops.length ?? 0) === sent) {
      return { error: "refused" };
    }
    throw error;
  }
}
