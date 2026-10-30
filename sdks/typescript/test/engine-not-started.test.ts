// When the SDK cannot start `janus-engine`, the error says why and what to do, and is laid out for a
// reader: someone moving from pact-js has no checkout to build one in (ADR 0029's release assets).

import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { EngineNotStartedError } from "../src/index.js";
import { Engine } from "../src/engine/engine.js";
import { engineNotStarted, RELEASES } from "../src/engine/release.js";
import { SubprocessPipe } from "../src/engine/subprocess.js";

const missing = { code: "ENOENT", message: "spawn janus-engine ENOENT" };

describe("an engine that cannot be started", () => {
  it("says why, then each way to get one, unwrapped when there is no terminal", () => {
    const error = engineNotStarted("janus-engine", "PATH", missing, "linux", "x64", undefined);
    expect(error).toBeInstanceOf(EngineNotStartedError);
    expect(error.message).toBe(
      [
        "there is no janus-engine on the PATH, and JANUS_ENGINE is not set.",
        "",
        "The SDK runs the Janus engine as its own executable, and does not ship it. Either:",
        `  - download janus-engine-x86_64-unknown-linux-musl from ${RELEASES}, make it executable, and put ` +
          "it on the PATH as janus-engine or set JANUS_ENGINE to its path; or",
        "  - in a pact-janus checkout, build it with `cargo build -p pact_janus_cli --bin janus-engine` " +
          "and set JANUS_ENGINE to target/debug/janus-engine.",
      ].join("\n"),
    );
  });

  it("is wrapped to the terminal at spaces only, under its bullet", () => {
    const lines = engineNotStarted("janus-engine", "PATH", missing, "linux", "x64", 60).message.split("\n");
    for (const line of lines) {
      expect(line.length <= 60 || line.includes("`"), line).toBe(true);
    }
    expect(lines.some((line) => line.startsWith(`    ${RELEASES},`)), "the URL is whole").toBe(true);
    expect(lines.some((line) => line.includes("`cargo build -p pact_janus_cli --bin janus-engine`")), "the command is whole").toBe(true);
    expect(lines.filter((line) => line.startsWith("  - "))).toHaveLength(2);
    expect(lines.filter((line) => line.startsWith("    ")).length).toBeGreaterThan(2);
  });

  it("names the error and carries no stack for a test runner to print as the test's code", () => {
    const error = engineNotStarted("janus-engine", "PATH", missing, "linux", "x64", undefined);
    expect(error.name).toBe("EngineNotStartedError");
    expect(error.stack).toBe(`EngineNotStartedError: ${error.message}`);
    expect(error.cause).toBeUndefined();
  });

  it("names what pointed at a path that does not exist", () => {
    expect(engineNotStarted("/opt/janus-engine", "JANUS_ENGINE", missing, "darwin", "arm64", undefined).message).toMatch(
      /^JANUS_ENGINE names \/opt\/janus-engine, which does not exist\.\n[\s\S]*janus-engine-aarch64-apple-darwin from/,
    );
  });

  it("is not executable: says to make it so", () => {
    const error = engineNotStarted("/opt/janus-engine", "engine.command", { code: "EACCES", message: "" }, "linux", "x64", undefined);
    expect(error.message).toMatch(/^\/opt\/janus-engine is not executable \(chmod \+x it\)\.\n/);
  });

  it("on a platform with no release, says to build one", () => {
    expect(engineNotStarted("janus-engine", "PATH", missing, "freebsd", "x64", undefined).message).toContain(
      "To get one:\n  - in a pact-janus checkout, build it with `cargo build -p pact_janus_cli --bin janus-engine` " +
        "and set JANUS_ENGINE to target/debug/janus-engine. No janus-engine is released for freebsd/x64.",
    );
  });

  it("fails the first request with that error", async () => {
    const dir = await mkdtemp(join(tmpdir(), "pact-janus-engine-"));
    const command = join(dir, "janus-engine");
    await expect(Engine.start(new SubprocessPipe({ command }))).rejects.toThrow(EngineNotStartedError);
    await expect(Engine.start(new SubprocessPipe({ command }))).rejects.toThrow(
      `engine.command names ${command}, which does not exist.`,
    );
    if (process.platform !== "win32") {
      await writeFile(command, "not an executable");
      await expect(Engine.start(new SubprocessPipe({ command }))).rejects.toThrow(`${command} is not executable`);
    }
  });
});
