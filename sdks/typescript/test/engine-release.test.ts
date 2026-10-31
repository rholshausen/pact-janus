// Resolving a released engine (staged plan S0.4, X.1, S1.36; ADR 0029 decision 3, ADR 0060 decision 6),
// against a release this test serves itself: what matters is that the pinned digest decides, whatever the server sends.

import { createHash } from "node:crypto";
import { access, constants, mkdtemp, readFile, readdir } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { engineAsset, enginePath, resolveEngine, type EnginePin } from "../src/engine/release.js";

const ENGINE = Buffer.from("#!/bin/sh\n# not an engine, but these are the bytes that were pinned\n");
const digest = createHash("sha256").update(ENGINE).digest("hex");
const asset = engineAsset();

/** A release host: serves `served` for the one asset of the one version, and counts what it is asked. */
let server: Server;
let baseUrl: string;
let served: Buffer = ENGINE;
const requests: string[] = [];

beforeAll(async () => {
  server = createServer((request, response) => {
    requests.push(request.url ?? "");
    if (request.url === `/v1.2.3/${asset}`) {
      response.writeHead(200).end(served);
    } else {
      response.writeHead(404).end();
    }
  });
  await new Promise<void>((listening) => server.listen(0, "127.0.0.1", listening));
  baseUrl = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
});

afterAll(() => new Promise<void>((closed) => server.close(() => closed())));

function pin(overrides: Partial<EnginePin> = {}): EnginePin {
  return { repository: "example/janus", version: "1.2.3", sha256: { [asset]: digest }, ...overrides };
}

const cache = () => mkdtemp(join(tmpdir(), "janus-engine-cache-"));

describe("resolving a released engine", () => {
  it("names the engine as the release does, per platform", () => {
    expect(engineAsset("linux", "x64")).toBe("janus-engine-x86_64-unknown-linux-musl");
    expect(engineAsset("linux", "arm64")).toBe("janus-engine-aarch64-unknown-linux-musl");
    expect(engineAsset("darwin", "arm64")).toBe("janus-engine-aarch64-apple-darwin");
    expect(engineAsset("darwin", "x64")).toBe("janus-engine-x86_64-apple-darwin");
    expect(engineAsset("win32", "x64")).toBe("janus-engine-x86_64-pc-windows-msvc.exe");
    expect(() => engineAsset("freebsd", "x64")).toThrow(/No janus-engine is released for freebsd\/x64/);
  });

  it("fetches the pinned version's engine once, and keeps it", async () => {
    served = ENGINE;
    const cacheDir = await cache();
    const before = requests.length;

    const path = await resolveEngine(pin(), { cacheDir, baseUrl });

    expect(path).toBe(enginePath(pin(), cacheDir));
    expect(await readFile(path)).toEqual(ENGINE);
    if (process.platform !== "win32") {
      await access(path, constants.X_OK);
    }
    expect(await resolveEngine(pin(), { cacheDir, baseUrl })).toBe(path);
    expect(requests.slice(before)).toEqual([`/v1.2.3/${asset}`]);
  });

  it("refuses a download that is not what was pinned, and keeps nothing of it", async () => {
    served = Buffer.from("#!/bin/sh\n# something else\n");
    const cacheDir = await cache();

    await expect(resolveEngine(pin(), { cacheDir, baseUrl })).rejects.toThrow(
      new RegExp(`is not the janus-engine that was pinned.*the pin says ${digest}`),
    );
    expect(await readdir(cacheDir)).toEqual([]);
  });

  it("fetches again when what is in the cache is not what was pinned", async () => {
    served = ENGINE;
    const cacheDir = await cache();
    const path = await resolveEngine(pin(), { cacheDir, baseUrl });
    const other = Buffer.from("tampered with since");
    const moved = pin({ sha256: { [asset]: createHash("sha256").update(other).digest("hex") } });
    served = other;

    expect(await resolveEngine(moved, { cacheDir, baseUrl })).toBe(path);
    expect(await readFile(path)).toEqual(other);
    expect(await readdir(dirname(path))).toEqual([asset]);
  });

  it("says so when nothing is pinned, or nothing for this platform", async () => {
    const cacheDir = await cache();
    await expect(resolveEngine(pin({ version: null }), { cacheDir, baseUrl })).rejects.toThrow(
      /No janus-engine release is pinned yet/,
    );
    await expect(resolveEngine(pin({ sha256: {} }), { cacheDir, baseUrl })).rejects.toThrow(
      new RegExp(`pinned with no digest for ${asset.replaceAll(".", "\\.")}`),
    );
  });

  it("says where it looked when the release does not have the file", async () => {
    const cacheDir = await cache();
    await expect(resolveEngine(pin({ version: "9.9.9" }), { cacheDir, baseUrl })).rejects.toThrow(
      new RegExp(`could not be fetched from ${baseUrl}/v9\\.9\\.9/.*HTTP 404`),
    );
  });
});
