// When the SDK cannot start `janus-engine`, the error says why and what to do, and is laid out for a
// reader: someone moving from pact-js has no checkout to build one in (ADR 0029's release assets). By
// default the SDK fetches the engine it was released with (ADR 0060 decision 6), and a fetch that
// fails is laid out the same way.

import { mkdtemp, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { EngineNotStartedError } from "../src/index.js";
import { Engine } from "../src/engine/engine.js";
import { engineNotFetched, engineNotStarted, PIN, RELEASES } from "../src/engine/release.js";
import { SubprocessPipe } from "../src/engine/subprocess.js";

const missing = { code: "ENOENT", message: "spawn janus-engine ENOENT" };

describe("an engine that cannot be started", () => {
  it("says why, then each way to get one, unwrapped when there is no terminal", () => {
    const error = engineNotStarted("/opt/janus-engine", "JANUS_ENGINE", missing, "linux", "x64", undefined);
    expect(error).toBeInstanceOf(EngineNotStartedError);
    expect(error.message).toBe(
      [
        "JANUS_ENGINE names /opt/janus-engine, which does not exist.",
        "",
        "The SDK runs the Janus engine as its own executable. Either:",
        "  - unset JANUS_ENGINE, and the SDK fetches the janus-engine it was released with; or",
        `  - download janus-engine-x86_64-unknown-linux-musl from ${RELEASES}/tag/v${PIN.version}, make it ` +
          "executable, and set JANUS_ENGINE to its path; or",
        "  - in a pact-janus checkout, build it with `cargo build -p pact_janus_cli --bin janus-engine` " +
          "and set JANUS_ENGINE to target/debug/janus-engine.",
      ].join("\n"),
    );
  });

  it("is wrapped to the terminal at spaces only, under its bullet", () => {
    const lines = engineNotStarted("/opt/janus-engine", "JANUS_ENGINE", missing, "linux", "x64", 60).message.split("\n");
    for (const line of lines) {
      expect(line.length <= 60 || line.includes("`") || line.includes(RELEASES), line).toBe(true);
    }
    expect(lines.some((line) => line.startsWith(`    ${RELEASES}/tag/v${PIN.version},`)), "the URL is whole").toBe(true);
    expect(lines.some((line) => line.includes("`cargo build -p pact_janus_cli --bin janus-engine`")), "the command is whole").toBe(true);
    expect(lines.filter((line) => line.startsWith("  - "))).toHaveLength(3);
    expect(lines.filter((line) => line.startsWith("    ")).length).toBeGreaterThan(2);
  });

  it("names the error and carries no stack for a test runner to print as the test's code", () => {
    const error = engineNotStarted("/opt/janus-engine", "JANUS_ENGINE", missing, "linux", "x64", undefined);
    expect(error.name).toBe("EngineNotStartedError");
    expect(error.stack).toBe(`EngineNotStartedError: ${error.message}`);
    expect(error.cause).toBeUndefined();
  });

  it("names what pointed at a path that does not exist", () => {
    expect(engineNotStarted("/opt/janus-engine", "engine.command", missing, "darwin", "arm64", undefined).message).toMatch(
      /^engine\.command names \/opt\/janus-engine, which does not exist\.\n[\s\S]*leave out engine\.command[\s\S]*janus-engine-aarch64-apple-darwin from/,
    );
  });

  it("is not executable: says to make it so", () => {
    const error = engineNotStarted("/opt/janus-engine", "engine.command", { code: "EACCES", message: "" }, "linux", "x64", undefined);
    expect(error.message).toMatch(/^\/opt\/janus-engine is not executable \(chmod \+x it\)\.\n/);
  });

  it("on a platform with no release, says to build one", () => {
    expect(engineNotFetched("No janus-engine is released for freebsd/x64", "freebsd", "x64", undefined).message).toBe(
      [
        "No janus-engine is released for freebsd/x64.",
        "",
        "The SDK fetches the Janus engine it was released with when it first runs. To get one:",
        "  - in a pact-janus checkout, build it with `cargo build -p pact_janus_cli --bin janus-engine` " +
          "and set JANUS_ENGINE to target/debug/janus-engine. No janus-engine is released for freebsd/x64.",
      ].join("\n"),
    );
  });

  it("when the release cannot be fetched, says to name a mirror, or to fetch it by hand", () => {
    const error = engineNotFetched("janus-engine 0.4.0 could not be fetched from https://example.com: HTTP 404", "linux", "arm64", undefined);
    expect(error.message.split("\n").slice(0, 4)).toEqual([
      "janus-engine 0.4.0 could not be fetched from https://example.com: HTTP 404.",
      "",
      "The SDK fetches the Janus engine it was released with when it first runs. Either:",
      "  - set JANUS_ENGINE_MIRROR to a server holding janus-engine's release files, as " +
        "`<mirror>/v<version>/<file>`; or",
    ]);
    expect(error.message).toContain(`download janus-engine-aarch64-unknown-linux-musl from ${RELEASES}/tag/v${PIN.version}`);
  });

  it("fails the first request with that error", async () => {
    const dir = await mkdtemp(join(tmpdir(), "pact-janus-engine-"));
    const command = join(dir, "janus-engine");
    const pipe = () => SubprocessPipe.start({ command }).then((started) => Engine.start(started));
    await expect(pipe()).rejects.toThrow(EngineNotStartedError);
    await expect(pipe()).rejects.toThrow(`engine.command names ${command}, which does not exist.`);
    if (process.platform !== "win32") {
      await writeFile(command, "not an executable");
      await expect(pipe()).rejects.toThrow(`${command} is not executable`);
    }
  });

  it("named by nothing, is the pinned release, and a fetch that fails is that error", async () => {
    const server = createServer((_, response) => response.writeHead(404).end());
    await new Promise<void>((listening) => server.listen(0, "127.0.0.1", listening));
    const mirror = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
    vi.stubEnv("JANUS_ENGINE", "");
    vi.stubEnv("JANUS_ENGINE_MIRROR", mirror);
    vi.stubEnv("JANUS_ENGINE_CACHE", await mkdtemp(join(tmpdir(), "janus-engine-cache-")));
    const stderr = vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    try {
      const started = SubprocessPipe.start();
      await expect(started).rejects.toThrow(EngineNotStartedError);
      await expect(started).rejects.toThrow(
        new RegExp(`^janus-engine ${PIN.version} could not be fetched from ${mirror}/v${PIN.version}/janus-engine-\\S+: HTTP 404\\.\\n`),
      );
    } finally {
      stderr.mockRestore();
      vi.unstubAllEnvs();
      await new Promise<void>((closed) => server.close(() => closed()));
    }
  });
});
