// Resolving a released janus-engine by version (staged plan S0.4, X.1; ADR 0029 decision 3).
//
// The pin is sdks/engine-release.json: a version, and the SHA-256 of the engine for each target
// triple. Resolving is a download and a hash — the file for this platform, from that version's
// release, kept in a cache — and a file whose digest is not the pinned one is never run and never
// kept. Nothing here looks at PATH.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { chmod, mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";

export interface EnginePin {
  /** `owner/name` of the repository whose releases hold the engine. */
  repository: string;
  /** The pinned release, or null while there is none to pin. */
  version: string | null;
  /** Release asset name → the SHA-256 it must have, in hex. */
  sha256: Record<string, string>;
}

export interface ResolveOptions {
  /** Where resolved engines are kept. Defaults to `engineCache()`. */
  cacheDir?: string;
  /** What a release's tag and asset name are appended to. Defaults to the repository's releases. */
  baseUrl?: string;
}

/** The pin every SDK in this repository shares. */
export function pinnedEngine(): EnginePin {
  const file = resolve(import.meta.dirname, "../../engine-release.json");
  return JSON.parse(readFileSync(file, "utf8")) as EnginePin;
}

const TRIPLES: Partial<Record<string, string>> = {
  "linux/x64": "x86_64-unknown-linux-musl",
  "linux/arm64": "aarch64-unknown-linux-musl",
  "darwin/x64": "x86_64-apple-darwin",
  "darwin/arm64": "aarch64-apple-darwin",
  "win32/x64": "x86_64-pc-windows-msvc",
};

/** What a release calls the engine for a platform (ADR 0028 decision 9). */
export function engineAsset(platform: string = process.platform, arch: string = process.arch): string {
  const triple = TRIPLES[`${platform}/${arch}`];
  if (!triple) {
    throw new Error(
      `No janus-engine is released for ${platform}/${arch}. Build one and name it in JANUS_ENGINE.`,
    );
  }
  return `janus-engine-${triple}${platform === "win32" ? ".exe" : ""}`;
}

/** `JANUS_ENGINE_CACHE`, or `pact-janus/engines` in the user's cache directory. */
export function engineCache(env: NodeJS.ProcessEnv = process.env): string {
  if (env.JANUS_ENGINE_CACHE) {
    return env.JANUS_ENGINE_CACHE;
  }
  const caches =
    process.platform === "win32"
      ? (env.LOCALAPPDATA ?? join(homedir(), "AppData", "Local"))
      : process.platform === "darwin"
        ? join(homedir(), "Library", "Caches")
        : (env.XDG_CACHE_HOME ?? join(homedir(), ".cache"));
  return join(caches, "pact-janus", "engines");
}

function pinnedVersion(pin: EnginePin): string {
  if (!pin.version) {
    throw new Error("No janus-engine release is pinned yet (sdks/engine-release.json).");
  }
  return pin.version;
}

/** Where the pinned engine for this platform is, once resolved. Reads nothing and fetches nothing. */
export function enginePath(pin: EnginePin, cacheDir: string = engineCache()): string {
  return join(cacheDir, pinnedVersion(pin), engineAsset());
}

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

/**
 * The pinned engine for this platform, as an executable on disk: from the cache if it is there with
 * the pinned digest, from the release otherwise. Rejects — and leaves nothing behind — if what the
 * release serves is not what was pinned.
 */
export async function resolveEngine(pin: EnginePin, options: ResolveOptions = {}): Promise<string> {
  const version = pinnedVersion(pin);
  const asset = engineAsset();
  const expected = pin.sha256[asset];
  if (!expected) {
    throw new Error(`janus-engine ${version} is pinned with no digest for ${asset}.`);
  }
  const path = enginePath(pin, options.cacheDir);
  const cached = await readFile(path).catch(() => undefined);
  if (cached && sha256(cached) === expected) {
    return path;
  }

  const baseUrl = options.baseUrl ?? `https://github.com/${pin.repository}/releases/download`;
  const url = `${baseUrl}/v${version}/${asset}`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`janus-engine ${version} could not be fetched from ${url}: HTTP ${response.status}`);
  }
  const bytes = new Uint8Array(await response.arrayBuffer());
  const actual = sha256(bytes);
  if (actual !== expected) {
    throw new Error(
      `${url} is not the janus-engine that was pinned: its SHA-256 is ${actual}, and the pin says ${expected}.`,
    );
  }
  // Written beside its final name and moved into place, so a run that is interrupted, or two that
  // overlap, never leave half an engine where the next one will look.
  await mkdir(dirname(path), { recursive: true });
  const partial = `${path}.${process.pid}.partial`;
  await writeFile(partial, bytes);
  await chmod(partial, 0o755);
  await rm(path, { force: true });
  await rename(partial, path);
  return path;
}
