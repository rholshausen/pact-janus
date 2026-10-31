// The engine this SDK runs, and where a user gets one when it cannot be started (staged plan S1.36;
// ADR 0029 decision 3, ADR 0060 decision 6). Unless it is told which executable to run, the SDK runs
// the janus-engine it was released with: `PIN` names it by version and by digest, and resolving it
// is a download and a hash — the file for this platform, from that version's release, kept in a
// cache. A file whose digest is not the pinned one is never run and never kept.

import { createHash } from "node:crypto";
import { chmod, mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { PIN } from "./pin.js";

export interface EnginePin {
  /** `owner/name` of the repository whose releases hold the engine. */
  repository: string;
  /** The pinned release, or null while there is none to pin. */
  version: string | null;
  /** Release asset name → the SHA-256 it must have, in hex. */
  sha256: Record<string, string>;
}

export { PIN };

/** Where the engine is released. */
export const RELEASES = `https://github.com/${PIN.repository}/releases`;

const TRIPLES: Partial<Record<string, string>> = {
  "linux/x64": "x86_64-unknown-linux-musl",
  "linux/arm64": "aarch64-unknown-linux-musl",
  "darwin/x64": "x86_64-apple-darwin",
  "darwin/arm64": "aarch64-apple-darwin",
  "win32/x64": "x86_64-pc-windows-msvc",
};

/** What a release calls the engine for a platform, or `undefined` when none is released for it. */
export function releasedEngine(
  platform: string = process.platform,
  arch: string = process.arch,
): string | undefined {
  const triple = TRIPLES[`${platform}/${arch}`];
  return triple && `janus-engine-${triple}${platform === "win32" ? ".exe" : ""}`;
}

/** What a release calls the engine for a platform (ADR 0028 decision 9). */
export function engineAsset(platform: string = process.platform, arch: string = process.arch): string {
  const asset = releasedEngine(platform, arch);
  if (!asset) {
    throw new Error(`No janus-engine is released for ${platform}/${arch}`);
  }
  return asset;
}

export interface ResolveOptions {
  /** Where resolved engines are kept. Defaults to `engineCache()`. */
  cacheDir?: string;
  /** What a release's tag and asset name are appended to. Defaults to `engineMirror()`. */
  baseUrl?: string;
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

/**
 * `JANUS_ENGINE_MIRROR`, or the pinned repository's releases: a base URL under which
 * `v<version>/<asset>` is the engine, as GitHub serves a release's files.
 */
export function engineMirror(pin: EnginePin = PIN, env: NodeJS.ProcessEnv = process.env): string {
  return (env.JANUS_ENGINE_MIRROR || `https://github.com/${pin.repository}/releases/download`).replace(/\/+$/, "");
}

function pinnedVersion(pin: EnginePin): string {
  if (!pin.version) {
    throw new Error("No janus-engine release is pinned yet");
  }
  return pin.version;
}

/** Where the pinned engine for this platform is, once resolved. Reads nothing and fetches nothing. */
export function enginePath(pin: EnginePin = PIN, cacheDir: string = engineCache()): string {
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
export async function resolveEngine(pin: EnginePin = PIN, options: ResolveOptions = {}): Promise<string> {
  const version = pinnedVersion(pin);
  const asset = engineAsset();
  const expected = pin.sha256[asset];
  if (!expected) {
    throw new Error(`janus-engine ${version} is pinned with no digest for ${asset}`);
  }
  const path = enginePath(pin, options.cacheDir);
  const cached = await readFile(path).catch(() => undefined);
  if (cached && sha256(cached) === expected) {
    return path;
  }

  const url = `${options.baseUrl ?? engineMirror(pin)}/v${version}/${asset}`;
  // Once per machine and version, and long enough to look like a hang if it said nothing.
  process.stderr.write(`pact-janus: fetching janus-engine ${version} from ${url}\n`);
  const response = await fetch(url).catch((error: unknown) => {
    const cause = error instanceof Error && error.cause instanceof Error ? error.cause.message : String(error);
    throw new Error(`janus-engine ${version} could not be fetched from ${url}: ${cause}`);
  });
  if (!response.ok) {
    throw new Error(`janus-engine ${version} could not be fetched from ${url}: HTTP ${response.status}`);
  }
  const bytes = new Uint8Array(await response.arrayBuffer());
  const actual = sha256(bytes);
  if (actual !== expected) {
    throw new Error(
      `${url} is not the janus-engine that was pinned: its SHA-256 is ${actual}, and the pin says ${expected}`,
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

const LOG_LEVELS: Record<string, string> = {
  trace: "trace",
  debug: "debug",
  info: "info",
  warn: "warn",
  error: "error",
  // pact-js's, and pino's: nothing the engine logs is fatal, so its errors are as close as it gets.
  fatal: "error",
  silent: "off",
  // As `janus verify --log-level` takes it.
  none: "off",
  off: "off",
};

/**
 * The engine's environment (ADR 0061): `env` with `RUST_LOG` as given, or else the level a test asked
 * for as pact-js spells it, in any case — its `logLevel` option, else `LOG_LEVEL` — or else `warn`.
 */
export function engineEnvironment(env: NodeJS.ProcessEnv, logLevel: string | undefined): NodeJS.ProcessEnv {
  if (env.RUST_LOG) {
    return env;
  }
  const [level, source] = logLevel?.trim() ? [logLevel, "logLevel"] : [env.LOG_LEVEL, "LOG_LEVEL"];
  if (!level?.trim()) {
    return { ...env, RUST_LOG: "warn" };
  }
  const known = LOG_LEVELS[level.trim().toLowerCase()];
  if (!known) {
    throw new TypeError(`${source} '${level}' is not one of ${Object.keys(LOG_LEVELS).join(", ")}`);
  }
  return { ...env, RUST_LOG: known };
}

/** What pointed the SDK at the executable it tried, as the error names it. */
export type EngineSource = "engine.command" | "JANUS_ENGINE" | "release";

const BUILD = "in a pact-janus checkout, build it with `cargo build -p pact_janus_cli --bin janus-engine` " +
  "and set JANUS_ENGINE to target/debug/janus-engine";

/**
 * The SDK could not start `janus-engine`. Its message is the reason, then what to do, wrapped to the
 * terminal when there is one. It has no stack: where the SDK noticed is no help to a reader, and a
 * test runner would print that code as if it were the test's.
 */
export class EngineNotStartedError extends Error {
  override readonly name = "EngineNotStartedError";

  constructor(message: string) {
    super(message);
    this.stack = `${this.name}: ${this.message}`;
  }
}

/** The engine could not be started: why, and how to get one. */
export function engineNotStarted(
  command: string,
  source: EngineSource,
  error: { code?: string; message: string },
  platform: string = process.platform,
  arch: string = process.arch,
  width: number | undefined = terminalWidth(),
): EngineNotStartedError {
  let reason: string;
  if (error.code === "ENOENT") {
    reason = source === "release" ? `${command} does not exist` : `${source} names ${command}, which does not exist`;
  } else if (error.code === "EACCES") {
    reason = `${command} is not executable${platform === "win32" ? "" : " (chmod +x it)"}`;
  } else {
    reason = `${command} could not be run: ${error.message}`;
  }
  const fixes =
    source === "release"
      ? []
      : [`${source === "JANUS_ENGINE" ? "unset" : "leave out"} ${source}, and the SDK fetches the janus-engine it was released with; or`];
  return laidOut(reason, "The SDK runs the Janus engine as its own executable.", fixes, platform, arch, width);
}

/** The pinned engine could not be fetched (`reason`, from `resolveEngine`): why, and how to get one. */
export function engineNotFetched(
  reason: string,
  platform: string = process.platform,
  arch: string = process.arch,
  width: number | undefined = terminalWidth(),
): EngineNotStartedError {
  const fixes = releasedEngine(platform, arch)
    ? ["set JANUS_ENGINE_MIRROR to a server holding janus-engine's release files, as `<mirror>/v<version>/<file>`; or"]
    : [];
  return laidOut(
    reason,
    "The SDK fetches the Janus engine it was released with when it first runs.",
    fixes,
    platform,
    arch,
    width,
  );
}

function laidOut(
  reason: string,
  intro: string,
  fixes: string[],
  platform: string,
  arch: string,
  width: number | undefined,
): EngineNotStartedError {
  const asset = releasedEngine(platform, arch);
  const all = asset
    ? [
        ...fixes,
        `download ${asset} from ${RELEASES}/tag/v${PIN.version}, make it executable, and set JANUS_ENGINE to its path; or`,
        `${BUILD}.`,
      ]
    : [...fixes, `${BUILD}. No janus-engine is released for ${platform}/${arch}.`];
  const lines = [
    ...wrap(`${reason}.`, width, "", ""),
    "",
    ...wrap(`${intro} ${all.length > 1 ? "Either" : "To get one"}:`, width, "", ""),
    ...all.flatMap((fix) => wrap(fix, width, "  - ", "    ")),
  ];
  return new EngineNotStartedError(lines.join("\n"));
}

/** The terminal's width less a test runner's indent, or `undefined` when output is not a terminal. */
function terminalWidth(): number | undefined {
  const columns = process.stderr.columns || process.stdout.columns || Number(process.env.COLUMNS);
  return columns ? Math.min(columns, 120) - 8 : undefined;
}

/**
 * Wraps at spaces only, and never inside a `quoted` command, so a URL or a command is never broken;
 * `undefined` width leaves it whole.
 */
function wrap(text: string, width: number | undefined, first: string, rest: string): string[] {
  const lines: string[] = [];
  let line = first;
  let words = 0;
  for (const word of text.match(/`[^`]*`\S*|\S+/g) ?? []) {
    if (width && words > 0 && line.length + 1 + word.length > width) {
      lines.push(line);
      line = rest;
      words = 0;
    }
    line += (words > 0 ? " " : "") + word;
    words += 1;
  }
  return [...lines, line];
}
