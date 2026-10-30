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
    reason = source === "PATH"
      ? "there is no janus-engine on the PATH, and JANUS_ENGINE is not set"
      : `${source} names ${command}, which does not exist`;
  } else if (error.code === "EACCES") {
    reason = `${command} is not executable${platform === "win32" ? "" : " (chmod +x it)"}`;
  } else {
    reason = `${command} could not be run: ${error.message}`;
  }
  const asset = releasedEngine(platform, arch);
  const fixes = asset
    ? [`download ${asset} from ${RELEASES}, make it executable, and put it on the PATH as janus-engine or ` +
        "set JANUS_ENGINE to its path; or", `${BUILD}.`]
    : [`${BUILD}. No janus-engine is released for ${platform}/${arch}.`];
  const lines = [
    ...wrap(`${reason}.`, width, "", ""),
    "",
    ...wrap(`The SDK runs the Janus engine as its own executable, and does not ship it. ${asset ? "Either" : "To get one"}:`, width, "", ""),
    ...fixes.flatMap((fix) => wrap(fix, width, "  - ", "    ")),
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
