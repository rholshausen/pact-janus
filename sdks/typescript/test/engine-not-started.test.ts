// When the SDK cannot start `janus-engine`, the error says why, which release asset fits this
// platform and how to point the SDK at it: someone moving from pact-js has no checkout to build one in.

import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { Engine } from "../src/engine/engine.js";
import { engineNotStarted, RELEASES } from "../src/engine/release.js";
import { SubprocessPipe } from "../src/engine/subprocess.js";

const missing = { code: "ENOENT", message: "spawn janus-engine ENOENT" };

describe("an engine that cannot be started", () => {
  it("is not on the PATH: says so, and names this platform's release asset and where it is", () => {
    const error = engineNotStarted("janus-engine", "PATH", missing, "linux", "x64");
    expect(error.message).toBe(
      "The Janus engine could not be started: there is no janus-engine on the PATH, and JANUS_ENGINE is not set.\n" +
        "The SDK runs the engine as its own executable, janus-engine, and does not ship it. Download " +
        `janus-engine-x86_64-unknown-linux-musl from ${RELEASES}, make it executable, and put it on the PATH ` +
        "as janus-engine or set JANUS_ENGINE to its path. In a pact-janus checkout, " +
        "`cargo build -p pact_janus_cli --bin janus-engine` builds target/debug/janus-engine.",
    );
  });

  it("names what pointed at a path that does not exist", () => {
    expect(engineNotStarted("/opt/janus-engine", "JANUS_ENGINE", missing, "darwin", "arm64").message).toMatch(
      /^The Janus engine could not be started: JANUS_ENGINE names \/opt\/janus-engine, which does not exist\.\n.*janus-engine-aarch64-apple-darwin from/,
    );
  });

  it("is not executable: says to make it so", () => {
    const error = engineNotStarted("/opt/janus-engine", "engine.command", { code: "EACCES", message: "" }, "linux", "x64");
    expect(error.message).toMatch(/^The Janus engine could not be started: \/opt\/janus-engine is not executable \(chmod \+x it\)\./);
  });

  it("on a platform with no release, says to build one", () => {
    expect(engineNotStarted("janus-engine", "PATH", missing, "freebsd", "x64").message).toContain(
      "No janus-engine is released for freebsd/x64. In a pact-janus checkout, `cargo build -p pact_janus_cli --bin janus-engine` builds target/debug/janus-engine. Set JANUS_ENGINE to its path.",
    );
  });

  it("fails the first request with that error", async () => {
    const dir = await mkdtemp(join(tmpdir(), "pact-janus-engine-"));
    const command = join(dir, "janus-engine");
    await expect(Engine.start(new SubprocessPipe({ command }))).rejects.toThrow(
      `The Janus engine could not be started: engine.command names ${command}, which does not exist.`,
    );
    if (process.platform !== "win32") {
      await writeFile(command, "not an executable");
      await expect(Engine.start(new SubprocessPipe({ command }))).rejects.toThrow(`${command} is not executable`);
    }
  });
});
