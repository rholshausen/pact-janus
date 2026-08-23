// Shared benchmark protocol for every host leg (mirrored in go/python/jvm):
// - cold-start: fresh compile+instantiate, 20 iterations (first reported separately — compile caches)
// - echo-small: {"op":"echo","payload":{"ping":1}}          5000 iters after 500 warmup
// - echo-100k:  echo of payloads/order-100kb.json           500 iters after 50 warmup
// - match-100k: match-type of the document against itself   200 iters after 20 warmup
import { readFileSync } from "node:fs";

export const DOC = readFileSync(new URL("../../payloads/order-100kb.json", import.meta.url), "utf8");
const enc = new TextEncoder();
export const FRAMES = {
  handshake: enc.encode(JSON.stringify({ op: "handshake", "protocol-versions": [1] })),
  echoSmall: enc.encode(JSON.stringify({ op: "echo", payload: { ping: 1 } })),
  echo100k: enc.encode(`{"op":"echo","payload":${DOC}}`),
  match100k: enc.encode(`{"op":"match-type","expected":${DOC},"actual":${DOC}}`),
};

export function stats(samplesNs) {
  const s = [...samplesNs].sort((a, b) => a - b);
  const at = (q) => s[Math.min(s.length - 1, Math.floor(q * s.length))];
  return { median_us: at(0.5) / 1e3, p95_us: at(0.95) / 1e3, min_us: s[0] / 1e3 };
}

export function bench(name, fn, { warmup, iters }) {
  for (let i = 0; i < warmup; i++) fn();
  const samples = [];
  for (let i = 0; i < iters; i++) {
    const t0 = process.hrtime.bigint();
    fn();
    samples.push(Number(process.hrtime.bigint() - t0));
  }
  const r = stats(samples);
  console.log(`${name}: median ${r.median_us.toFixed(1)}µs  p95 ${r.p95_us.toFixed(1)}µs  min ${r.min_us.toFixed(1)}µs`);
  return r;
}

export function checkResponse(name, bytes, expectKey = "ok") {
  const resp = JSON.parse(new TextDecoder().decode(bytes));
  if (!(expectKey in resp)) throw new Error(`${name}: unexpected response ${JSON.stringify(resp).slice(0, 200)}`);
  return resp;
}
