// Component-model path: jco-transpiled component + preview2-shim WASI, async instantiation.
import { readFile } from "node:fs/promises";
import { instantiate } from "./transpiled/toy_engine_component.js";
import { WASIShim } from "@bytecodealliance/preview2-shim/instantiation";
import { FRAMES, bench, checkResponse, stats } from "./bench-common.mjs";

const coreModules = new Map();
async function getCoreModule(path) {
  // Cache file bytes but recompile per instantiation? For cold start we want the
  // full compile; jco calls this per core module. Compile fresh each time.
  if (!coreModules.has(path)) coreModules.set(path, await readFile(new URL(`./transpiled/${path}`, import.meta.url)));
  return WebAssembly.compile(coreModules.get(path));
}

const wasiImports = new WASIShim().getImportObject();

async function freshInstance() {
  const root = await instantiate(getCoreModule, wasiImports);
  return root.pipe;
}

// Cold start: compile + instantiate, 20 fresh instances
const cold = [];
let pipe;
for (let i = 0; i < 20; i++) {
  const t0 = process.hrtime.bigint();
  pipe = await freshInstance();
  cold.push(Number(process.hrtime.bigint() - t0));
}
const c = stats(cold);
console.log(`cold-start: first ${(cold[0] / 1e6).toFixed(1)}ms  median ${(c.median_us / 1e3).toFixed(1)}ms`);

checkResponse("handshake", pipe.call(FRAMES.handshake));
checkResponse("echo-small", pipe.call(FRAMES.echoSmall));
checkResponse("match-100k", pipe.call(FRAMES.match100k));

bench("echo-small", () => pipe.call(FRAMES.echoSmall), { warmup: 500, iters: 5000 });
bench("echo-100k", () => pipe.call(FRAMES.echo100k), { warmup: 50, iters: 500 });
bench("match-100k", () => pipe.call(FRAMES.match100k), { warmup: 20, iters: 200 });
