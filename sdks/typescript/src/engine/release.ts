// Where a user gets a `janus-engine` when the SDK cannot start one. The SDK runs the engine as its own
// executable and does not ship it, so the error says which release asset fits this platform (ADR 0028
// decision 9) and how to point the SDK at it.

/** Where the engine is released. */
export const RELEASES = "https://github.com/rholshausen/pact-janus/releases";

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

/** What pointed the SDK at the executable it tried, as the error names it. */
export type EngineSource = "engine.command" | "JANUS_ENGINE" | "PATH";

const BUILD = "In a pact-janus checkout, `cargo build -p pact_janus_cli --bin janus-engine` builds target/debug/janus-engine.";

/** The engine could not be started: why, and how to get one. */
export function engineNotStarted(
  command: string,
  source: EngineSource,
  error: { code?: string; message: string },
  platform: string = process.platform,
  arch: string = process.arch,
): Error {
  let reason: string;
  if (error.code === "ENOENT") {
    reason = source === "PATH"
      ? "there is no janus-engine on the PATH, and JANUS_ENGINE is not set"
      : `${source} names ${command}, which does not exist`;
  } else if (error.code === "EACCES") {
    reason = `${command} is not executable${platform === "win32" ? "" : " (chmod +x it)"}`;
  } else {
    reason = `${command}: ${error.message}`;
  }
  const asset = releasedEngine(platform, arch);
  const help = asset
    ? `The SDK runs the engine as its own executable, janus-engine, and does not ship it. Download ${asset} ` +
      `from ${RELEASES}, make it executable, and put it on the PATH as janus-engine or set JANUS_ENGINE to ` +
      `its path. ${BUILD}`
    : `No janus-engine is released for ${platform}/${arch}. ${BUILD} Set JANUS_ENGINE to its path.`;
  return new Error(`The Janus engine could not be started: ${reason}.\n${help}`, { cause: error });
}
