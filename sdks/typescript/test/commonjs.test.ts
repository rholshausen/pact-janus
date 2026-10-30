// The CommonJS build (tsconfig.cjs.json): a pact-js user's Jest tests `require()` the SDK, and Jest
// cannot load an ES module that way before Node 24.9. The build is compiled to a scratch package here
// and required through package.json's own exports, as such a project resolves it.

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { cp, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

type Conditions = { import?: { default: string }; require?: { default: string } };

describe("the CommonJS build", () => {
  it("is what require() resolves for every export but pact-janus/vitest", async () => {
    const root = await mkdtemp(join(tmpdir(), "pact-janus-cjs-"));
    const pkg = join(root, "node_modules", "pact-janus");
    const manifest = JSON.parse(await readFile("package.json", "utf8")) as {
      exports: Record<string, Conditions>;
    };
    execFileSync(process.execPath, [join("node_modules", "typescript", "bin", "tsc"), "-p", "tsconfig.cjs.json",
      "--outDir", join(pkg, "dist", "cjs")]);
    execFileSync(process.execPath, ["--experimental-strip-types", "--disable-warning=ExperimentalWarning",
      join("scripts", "commonjs-package.ts"), join(pkg, "dist", "cjs")]);
    await cp("package.json", join(pkg, "package.json"));

    for (const [entry, conditions] of Object.entries(manifest.exports)) {
      if (entry === "./vitest") {
        expect(conditions.require, "Vitest is ES modules only").toBeUndefined();
        continue;
      }
      const required = conditions.require?.default ?? "";
      expect(required, entry).toMatch(/^\.\/dist\/cjs\//);
      expect(existsSync(join(pkg, required)), entry).toBe(true);
    }

    const script = join(root, "require.cjs");
    await writeFile(
      script,
      `const { Verifier, MessageProviderPact, providerWithMetadata } = require("pact-janus/verifier");
       const janus = require("pact-janus");
       require("pact-janus/bindings");
       console.log(JSON.stringify([typeof Verifier, typeof MessageProviderPact, typeof providerWithMetadata,
         typeof janus.Verifier, typeof janus.VerificationFailedError]));`,
    );
    const loaded = execFileSync(process.execPath, [script], { cwd: root, encoding: "utf8" });
    expect(JSON.parse(loaded)).toEqual(["function", "function", "function", "function", "function"]);
  }, 60_000);
});
