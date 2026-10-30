// Runs a provider case (README §3): a verifier built from the case's data, its handlers recording
// each call, `verify` against the case's scripted engine — or, for a live case, the real engine and
// a provider this driver serves. What this file knows is how TypeScript spells the primitives;
// what they must do is the case's.

import { mkdtemp, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";
import {
  JanusError,
  StateUnsupportedError,
  VerificationFailedError,
  Verifier,
  type Message,
  type VerificationResult,
} from "../../src/index.js";
import type { Behaviour, Case, HandlerCall, LiveProvider, MessageScript, Outcome } from "./cases.js";
import { equal, show, subset } from "./compare.js";
import { ScriptedPipe } from "./scripted-engine.js";

export async function runProvider(testCase: Case, failures: string[]): Promise<void> {
  const live = testCase.category === "live";
  const pipe = live ? undefined : new ScriptedPipe(testCase.engine ?? {});
  const script = testCase.verifier ?? {};
  const expected = testCase.expect ?? {};
  const dir = await mkdtemp(join(tmpdir(), "janus-provider-"));
  const provider = live && testCase.provider ? await serve(testCase.provider) : undefined;
  const calls: HandlerCall[] = [];
  try {
    const pacts = await Promise.all(
      (script.pacts ?? []).map(async (document, i) => {
        const file = join(dir, `pact-${i + 1}.json`);
        await writeFile(file, JSON.stringify(document), "utf8");
        return file;
      }),
    );
    const configFile = join(dir, "verifier.janus.yaml");
    if (script.config !== undefined) {
      await writeFile(configFile, script.config, "utf8");
    }
    const verifier = new Verifier({
      providerUrl: script["provider-url"] ?? provider?.url ?? "",
      ...(script.pacts ? { pacts } : {}),
      ...(script.broker ? { broker: script.broker } : {}),
      ...(script.urls ? { urls: script.urls } : {}),
      ...(script.options ? { options: script.options } : {}),
      ...(script.transport ? { transport: script.transport } : {}),
      // Relative to the working directory, as a test would write it (`config-loaded`).
      ...(script.config !== undefined ? { config: relative(process.cwd(), configFile) } : {}),
      ...(script.components ? { components: script.components } : {}),
      ...(pipe ? { engine: () => pipe } : {}),
    });
    (script["state-handlers"] ?? []).forEach(({ state, setup, teardown }, i) => {
      const handler = `state-${i + 1}`;
      verifier.state(state, {
        setup: (params) => act(setup, calls, { handler, action: "setup", params }),
        ...(teardown ? { teardown: (params) => act(teardown, calls, { handler, action: "teardown", params }) } : {}),
      });
    });
    (script["request-filters"] ?? []).forEach((filter, i) => {
      const handler = `request-filter-${i + 1}`;
      verifier.requestFilter(
        (request) => {
          calls.push({ handler, request });
          if (filter.throws !== undefined) {
            throw new Error(filter.throws);
          }
          return { ...request, ...(filter.set ?? {}) };
        },
        filter.changes ? { changes: filter.changes } : {},
      );
    });
    (script["message-producers"] ?? []).forEach(({ description, returns, throws }, i) => {
      const handler = `message-${i + 1}`;
      verifier.message(description, (request) => {
        calls.push({ handler, request });
        if (throws !== undefined) {
          throw new Error(throws);
        }
        return message(returns ?? { json: null });
      });
    });

    if (testCase.verify !== false) {
      const printed: string[] = [];
      const { outcome, result } = await printing(printed, () => capture(() => verifier.verify(), pipe));
      if (expected.printed !== undefined && !equal(expected.printed, printed)) {
        failures.push(`what verify printed differs\n  expected: ${show(expected.printed)}\n  actual:   ${show(printed)}`);
      }
      if (expected.outcome !== undefined && !(equal(expected.outcome, outcome) || subset(expected.outcome, outcome))) {
        failures.push(`the outcome differs\n  expected: ${show(expected.outcome)}\n  actual:   ${show(outcome)}`);
      }
      if (expected.result?.status !== undefined && expected.result.status !== result?.status) {
        failures.push(`the result's status is ${show(result?.status)}, and the case expects ${show(expected.result.status)}`);
      }
      if (expected.result?.results !== undefined && !equal(expected.result.results, result?.results)) {
        failures.push(`the results differ\n  expected: ${show(expected.result.results)}\n  actual:   ${show(result?.results)}`);
      }
      if (expected.result?.warnings !== undefined && !equal(expected.result.warnings, result?.warnings)) {
        failures.push(`the warnings differ\n  expected: ${show(expected.result.warnings)}\n  actual:   ${show(result?.warnings)}`);
      }
    }

    const verify = pipe?.bodyOf("verification/verify");
    const hooks = (verify?.target as { hooks?: unknown } | undefined)?.hooks;
    if (expected.ops && pipe && !equal(expected.ops, pipe.ops)) {
      failures.push(`the operations sent differ\n  expected: ${show(expected.ops)}\n  actual:   ${show(pipe.ops)}`);
    }
    for (const [op, body] of Object.entries(expected.frames ?? {})) {
      if (!subset(body, pipe?.bodyOf(op))) {
        failures.push(`the '${op}' frame body\n  expected (a subset of): ${show(body)}\n  actual: ${show(pipe?.bodyOf(op))}`);
      }
    }
    if (expected["engine-closed"] !== undefined && pipe && pipe.closed !== expected["engine-closed"]) {
      failures.push(`the engine was ${pipe.closed ? "" : "not "}ended, and the case expects it ${expected["engine-closed"] ? "" : "not "}to be`);
    }
    if (expected.hooks !== undefined) {
      const actual = hooks === undefined ? null : normalised(hooks);
      if (!equal(expected.hooks, actual)) {
        failures.push(`target.hooks differs\n  expected: ${show(expected.hooks)}\n  actual:   ${show(actual)}`);
      }
    }
    if (expected["config-loaded"]) {
      const path = pipe?.bodyOf("config/load")?.path;
      if (path !== configFile) {
        failures.push(`config/load carried ${show(path)}, and the case expects the file's absolute path ${show(configFile)}`);
      }
    }
    if (expected.invocations !== undefined && !equal(expected.invocations, pipe?.invocations)) {
      failures.push(`the endpoint's answers differ\n  expected: ${show(expected.invocations)}\n  actual:   ${show(pipe?.invocations)}`);
    }
    const wanted = expected["handler-calls"];
    if (wanted !== undefined && (wanted.length !== calls.length || !wanted.every((call, i) => subset(call, calls[i])))) {
      failures.push(`the handlers were called differently\n  expected: ${show(wanted)}\n  actual:   ${show(calls)}`);
    }
    if (expected["endpoint-closed"]) {
      const url = loopbackUrl(hooks);
      if (url === undefined) {
        failures.push("the verify frame names no loopback endpoint to check");
      } else if (await answers(url)) {
        failures.push(`the endpoint at ${url} still answers after verify ended`);
      }
    }
  } finally {
    await provider?.close();
  }
}

/** A handler's behaviour, after recording the call. */
function act(behaviour: Behaviour, calls: HandlerCall[], call: HandlerCall): unknown {
  calls.push(call);
  if (behaviour.throws !== undefined) {
    throw new Error(behaviour.throws);
  }
  if (behaviour.unsupported !== undefined) {
    throw new StateUnsupportedError(behaviour.unsupported);
  }
  return behaviour.returns;
}

/** The case's message as this SDK's: bytes as a Uint8Array, a string as itself, JSON as a value. */
function message(script: MessageScript): Message {
  const body = script.bytes !== undefined ? new Uint8Array(Buffer.from(script.bytes, "base64")) : (script.text ?? script.json);
  return {
    body,
    ...(script["content-type"] !== undefined ? { contentType: script["content-type"] } : {}),
    ...(script.metadata !== undefined ? { metadata: script.metadata } : {}),
  };
}

/** This SDK's ways of ending `verify`, as the case's outcomes (README §4). */
async function capture(
  run: () => Promise<VerificationResult>,
  pipe: ScriptedPipe | undefined,
): Promise<{ outcome: "ok" | Outcome; result?: VerificationResult }> {
  try {
    return { outcome: "ok", result: await run() };
  } catch (error) {
    if (error instanceof VerificationFailedError) {
      const failed = error.failures.map(({ interaction, variant }) => ({ interaction, variant }));
      return { outcome: { error: "verification-failed", failed }, result: error.result };
    }
    if (error instanceof JanusError) {
      return { outcome: { error: "engine-error", code: error.code } };
    }
    if (error instanceof TypeError && (pipe?.ops.length ?? 0) === 0) {
      return { outcome: { error: "refused" } };
    }
    throw error;
  }
}

/** target.hooks with each loopback url as `loopback:<path>` and each run token as `<token>` (README §4). */
function normalised(hooks: unknown): unknown {
  const copy = structuredClone(hooks) as { hooks?: Record<string, { run?: { url?: string; headers?: Record<string, string> } }[]> };
  for (const entries of Object.values(copy.hooks ?? {})) {
    for (const { run } of entries) {
      if (run?.url?.startsWith("http://127.0.0.1:")) {
        run.url = `loopback:${new URL(run.url).pathname}`;
      }
      if (run?.headers?.authorization && /^Bearer [0-9a-f]{32,}$/i.test(run.headers.authorization)) {
        run.headers.authorization = "Bearer <token>";
      }
    }
  }
  return copy;
}

function loopbackUrl(hooks: unknown): string | undefined {
  const points = (hooks as { hooks?: Record<string, { run?: { url?: string } }[]> } | undefined)?.hooks ?? {};
  return Object.values(points)
    .flat()
    .map((entry) => entry.run?.url)
    .find((url) => url?.startsWith("http://127.0.0.1:"));
}

async function answers(url: string): Promise<boolean> {
  try {
    await fetch(url, { method: "POST", body: "{}" });
    return true;
  } catch {
    return false;
  }
}

/** The live case's provider, on the loopback interface. */
async function serve(provider: LiveProvider): Promise<{ url: string; close: () => Promise<void> }> {
  const server = createServer((request, response) => {
    const path = new URL(request.url ?? "/", "http://localhost").pathname;
    const route = provider.routes.find((r) => r.method === request.method && r.path === path);
    if (!route) {
      response.writeHead(404).end();
      return;
    }
    const missing = Object.entries(route["require-headers"] ?? {}).some(
      ([name, value]) => request.headers[name.toLowerCase()] !== value,
    );
    if (missing) {
      response.writeHead(401).end();
      return;
    }
    const body = route.body === undefined ? "" : JSON.stringify(route.body);
    response.writeHead(route.status, route.headers ?? {}).end(body);
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", () => resolve()));
  return {
    url: `http://127.0.0.1:${(server.address() as AddressInfo).port}`,
    close: () =>
      new Promise<void>((resolve) => {
        server.closeAllConnections();
        server.close(() => resolve());
      }),
  };
}

/** Runs `run`, keeping what it writes to standard output as `printed`, line by line (ADR 0059). */
async function printing<T>(printed: string[], run: () => Promise<T>): Promise<T> {
  const write = process.stdout.write.bind(process.stdout);
  let text = "";
  process.stdout.write = ((chunk: string | Uint8Array) => {
    text += typeof chunk === "string" ? chunk : Buffer.from(chunk).toString("utf8");
    return true;
  }) as typeof process.stdout.write;
  try {
    return await run();
  } finally {
    process.stdout.write = write;
    printed.push(...(text === "" ? [] : text.replace(/\n$/, "").split("\n")));
  }
}
