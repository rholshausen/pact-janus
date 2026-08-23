// Core-WASM path: hand-built C-ABI shim over Node's built-in WebAssembly.
// This is what "no component model" costs in host code: ~30 lines of pointer math.
import { readFile } from "node:fs/promises";
import { FRAMES, bench, checkResponse, stats } from "./bench-common.mjs";

const wasmBytes = await readFile(new URL("../../engine-toy/target/wasm32-unknown-unknown/release/toy_engine_core.wasm", import.meta.url));

function makePipe(instance) {
  const { alloc, dealloc, call, memory } = instance.exports;
  return {
    call(request) {
      const reqPtr = alloc(request.length);
      new Uint8Array(memory.buffer, reqPtr, request.length).set(request);
      const packed = call(reqPtr, request.length);
      const respPtr = Number(packed >> 32n);
      const respLen = Number(packed & 0xffffffffn);
      const response = new Uint8Array(memory.buffer, respPtr, respLen).slice();
      dealloc(respPtr, respLen);
      dealloc(reqPtr, request.length);
      return response;
    },
  };
}

const cold = [];
let pipe;
for (let i = 0; i < 20; i++) {
  const t0 = process.hrtime.bigint();
  const module = await WebAssembly.compile(wasmBytes);
  const instance = await WebAssembly.instantiate(module, {});
  pipe = makePipe(instance);
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
