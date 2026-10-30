// What the ported pact-js provider examples share (ADR 0056 decision 5): where their pacts are in
// S1.1's corpus, and a provider on node:http in place of the examples' Express apps, answering as
// they answer. Every example's pact is a fixture there, so `verifier-diff` holds today's verifier
// and `janus verify` to the verdicts these tests hold the facade to.

import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";
import { join } from "node:path";
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
