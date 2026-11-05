// What the ported pact-js provider examples share (ADR 0056 decision 5): where their pacts are in
// S1.1's corpus, and a provider on node:http in place of the examples' Express apps, answering as
// they answer. Every example's pact is a fixture there, so `verifier-diff` holds today's verifier
// and `janus verify` to the verdicts these tests hold the facade to.

import { mkdtempSync, readFileSync } from "node:fs";
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { SubprocessPipe } from "../../src/engine/subprocess.js";
import { specRoot } from "../conformance/cases.js";

/** A pact in S1.1's corpus, by its path below corpora/verifier. */
export function corpusPact(path: string): string {
  return join(specRoot, "corpora", "verifier", path);
}

export type Handler = (request: IncomingMessage, body: string, response: ServerResponse) => void;

export interface Provider {
  url: string;
  close(): Promise<void>;
}

/** A provider on the loopback interface, at a port the system chooses. */
export async function serve(handler: Handler): Promise<Provider> {
  const server = createServer((request, response) => {
    const chunks: Buffer[] = [];
    request.on("data", (chunk: Buffer) => chunks.push(chunk));
    request.on("end", () => handler(request, Buffer.concat(chunks).toString("utf8"), response));
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

/** Express's `res.json`. */
export function json(response: ServerResponse, status: number, body: unknown): void {
  response.writeHead(status, { "Content-Type": "application/json; charset=utf-8" }).end(JSON.stringify(body));
}

// The consumer side (ADR 0066): a ported consumer test records its contract into a scratch
// directory, and the exit (staged plan S2.5a) holds it to what `janus upgrade` makes of the pact
// pact-js wrote for the same test, which is the pact S1.1's corpus already has.

/** A fresh directory for a suite's contract. */
export function contractDir(): string {
  return mkdtempSync(join(tmpdir(), "pact-janus-facade-"));
}

/** The interaction a facade suite recorded, by description. */
export function recorded(dir: string, file: string, description: string): Record<string, unknown> {
  const contract = JSON.parse(readFileSync(join(dir, file), "utf8")) as { interactions: Record<string, unknown>[] };
  const found = contract.interactions.find((interaction) => interaction.description === description);
  if (!found) {
    throw new Error(`${file} records no '${description}'`);
  }
  return found;
}

/** The interaction `upgrade/pact` makes of a corpus pact's interaction, by description. */
export async function upgraded(pact: string, description: string): Promise<Record<string, unknown>> {
  const pipe = await SubprocessPipe.start();
  try {
    const hello = { "protocol-versions": [1], host: { name: "examples", version: "0" } };
    await pipe.exchange({ type: "request", id: "1", op: "engine/hello", body: hello });
    const body = { pact: JSON.parse(readFileSync(pact, "utf8")) as unknown };
    const response = await pipe.exchange({ type: "request", id: "2", op: "upgrade/pact", body });
    const contract = response.ok?.contract as { interactions: Record<string, unknown>[] };
    const found = contract.interactions.find((interaction) => interaction.description === description);
    if (!found) {
      throw new Error(`${pact} upgrades to no '${description}'`);
    }
    return found;
  } finally {
    await pipe.close();
  }
}

/**
 * A contract interaction with each recorded JSON body's text parsed: pact_ffi wrote pact-js's pacts
 * with their members sorted, and the engine keeps them in the order the test wrote them (ADR 0037),
 * which decides nothing.
 */
export function documents(interaction: Record<string, unknown>): Record<string, unknown> {
  const copy = structuredClone(interaction) as { selection: { variants: { parts: Record<string, Record<string, Record<string, unknown>>> }[] } };
  for (const variant of copy.selection.variants) {
    for (const part of Object.values(variant.parts)) {
      const body = part.body;
      if (body?.encoded === "text" && typeof body.content === "string") {
        try {
          body.content = JSON.parse(body.content) as unknown;
        } catch {
          // Not JSON: compared as the text it is.
        }
      }
    }
  }
  return copy as unknown as Record<string, unknown>;
}

/** What a contract says about an interaction, without the variants that recorded it. */
export function shapes(interaction: Record<string, unknown>): Record<string, unknown> {
  return Object.fromEntries(Object.entries(interaction).filter(([member]) => member !== "selection"));
}
