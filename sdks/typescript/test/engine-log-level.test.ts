// How much the engine logs (staged plan S1.37, ADR 0061): set as a pact-js user sets it, by a test's
// `logLevel` option or `LOG_LEVEL`, and passed to the engine as `RUST_LOG` — which, when it is given
// already, is passed as it is.

import { describe, expect, it } from "vitest";
import { Janus } from "../src/index.js";
import { engineEnvironment } from "../src/engine/release.js";
import { Verifier } from "../src/facade/verifier.js";

describe("the engine's log level", () => {
  it("is the logLevel option, in any case", () => {
    expect(engineEnvironment({}, "debug").RUST_LOG).toBe("debug");
    expect(engineEnvironment({}, "TRACE").RUST_LOG).toBe("trace");
    expect(engineEnvironment({ LOG_LEVEL: "error" }, "info").RUST_LOG).toBe("info");
  });

  it("else LOG_LEVEL, else warn", () => {
    expect(engineEnvironment({ LOG_LEVEL: "Debug" }, undefined).RUST_LOG).toBe("debug");
    expect(engineEnvironment({}, undefined).RUST_LOG).toBe("warn");
    expect(engineEnvironment({ LOG_LEVEL: "" }, "").RUST_LOG).toBe("warn");
  });

  it("takes pact-js's fatal and silent", () => {
    expect(engineEnvironment({}, "fatal").RUST_LOG).toBe("error");
    expect(engineEnvironment({}, "silent").RUST_LOG).toBe("off");
  });

  it("is RUST_LOG when it is given, directives and all", () => {
    const env = { RUST_LOG: "warn,pact_janus_component_http=trace", LOG_LEVEL: "debug" };
    expect(engineEnvironment(env, "info")).toEqual(env);
  });

  it("refuses a level it does not know, naming where it came from", () => {
    expect(() => engineEnvironment({}, "verbose")).toThrow(/^logLevel 'verbose' is not one of trace, debug/);
    expect(() => engineEnvironment({ LOG_LEVEL: "loud" }, undefined)).toThrow(/^LOG_LEVEL 'loud' is not one of/);
  });

  it("refuses an unknown logLevel at the call that starts the engine", async () => {
    const janus = new Janus({ consumer: "c", provider: "p", logLevel: "verbose", engine: { command: "janus-engine" } });
    const interaction = janus.interaction("x").request({ method: "GET", path: "/" }).response({ status: 200 });
    await expect(janus.execute(interaction, () => undefined)).rejects.toThrow(/logLevel 'verbose'/);
  });

  it("is pact-js's logLevel, through the facade", async () => {
    const verifying = new Verifier({
      providerBaseUrl: "http://127.0.0.1:1",
      pactUrls: ["https://broker.example/pacts/1"],
      logLevel: "verbose",
      engine: { command: "janus-engine" },
    }).verifyProvider();
    await expect(verifying).rejects.toThrow(/logLevel 'verbose'/);
  });
});
