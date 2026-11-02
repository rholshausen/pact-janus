// The SDK built into a scratch project's node_modules, as a user's project installs it: resolved
// through package.json's own exports, never through this directory's sources.

import { execFileSync } from "node:child_process";
import { cp } from "node:fs/promises";
import { join } from "node:path";

const tsc = join("node_modules", "typescript", "bin", "tsc");

/** Builds the package into `<root>/node_modules/pact-janus`: its CommonJS build, and with `esm` its ES modules too. */
export async function buildPackage(root: string, { esm }: { esm: boolean }): Promise<string> {
  const pkg = join(root, "node_modules", "pact-janus");
  if (esm) {
    execFileSync(process.execPath, [tsc, "-p", "tsconfig.build.json", "--outDir", join(pkg, "dist")]);
  }
  execFileSync(process.execPath, [tsc, "-p", "tsconfig.cjs.json", "--outDir", join(pkg, "dist", "cjs")]);
  execFileSync(process.execPath, ["--experimental-strip-types", "--disable-warning=ExperimentalWarning",
    join("scripts", "commonjs-package.ts"), join(pkg, "dist", "cjs")]);
  await cp("package.json", join(pkg, "package.json"));
  return pkg;
}
