// The loopback endpoint a provider verification's handlers are served from (ADR 0055 decision 2):
// one HTTP server on 127.0.0.1, at a port the system chooses, for one run. The engine reaches it as
// it reaches any `http` hook, POSTing a hook context and reading an invoke result. Every request
// must carry the run's bearer token, so no other process on the machine can drive the handlers.
// This file decides nothing about what a handler means; it only carries documents to and from one.

import { randomBytes, timingSafeEqual } from "node:crypto";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";

/** A handler's route: the hook context in, the invoke result out. It does not throw. */
export type Route = (context: Record<string, unknown>) => Promise<Record<string, unknown>>;

export class Loopback {
  /** 128 random bits, as hex: the run's token. */
  readonly #token = randomBytes(16).toString("hex");
  readonly #routes = new Map<string, Route>();
  #server: Server | undefined;
  #base = "";

  /** The `authorization` header every entry carries. */
  get authorization(): string {
    return `Bearer ${this.#token}`;
  }

  route(path: string, route: Route): void {
    this.#routes.set(path, route);
  }

  /** Where `path` is served, once started. */
  url(path: string): string {
    return `${this.#base}${path}`;
  }

  async start(): Promise<void> {
    const server = createServer((request, response) => void this.#serve(request, response));
    await new Promise<void>((resolve, reject) => {
      server.once("error", reject);
      server.listen(0, "127.0.0.1", () => resolve());
    });
    this.#server = server;
    this.#base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  }

  async stop(): Promise<void> {
    const server = this.#server;
    this.#server = undefined;
    if (server) {
      server.closeAllConnections();
      await new Promise<void>((resolve) => server.close(() => resolve()));
    }
  }

  async #serve(request: IncomingMessage, response: ServerResponse): Promise<void> {
    if (!this.#authorised(request.headers.authorization)) {
      response.writeHead(401).end();
      return;
    }
    const route = request.method === "POST" ? this.#routes.get(request.url ?? "") : undefined;
    if (!route) {
      response.writeHead(404).end();
      return;
    }
    const chunks: Buffer[] = [];
    for await (const chunk of request) {
      chunks.push(chunk as Buffer);
    }
    let context: Record<string, unknown>;
    try {
      context = JSON.parse(Buffer.concat(chunks).toString("utf8")) as Record<string, unknown>;
    } catch {
      response.writeHead(400).end();
      return;
    }
    const body = JSON.stringify(await route(context));
    response.writeHead(200, { "content-type": "application/json", "content-length": Buffer.byteLength(body) }).end(body);
  }

  #authorised(header: string | undefined): boolean {
    const expected = Buffer.from(this.authorization);
    const given = Buffer.from(header ?? "");
    return given.length === expected.length && timingSafeEqual(given, expected);
  }
}
