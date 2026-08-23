// Node host leg: spawn pact-engine-toy, drive LSP-framed stdio, prove the
// lifecycle scenarios, and run the 1.2 benchmark protocol over the pipe.
//
// `node main.mjs`          — run all scenarios
// `node main.mjs --runner` — orphan-test helper: spawn engine, print its pid, hold
import { spawn, execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { setTimeout as sleep } from "node:timers/promises";

const ENGINE = new URL("../../engine-bin/target/release/pact-engine-toy", import.meta.url).pathname;
const DOC = readFileSync(new URL("../../../1.2-wasm-embedding/payloads/order-100kb.json", import.meta.url), "utf8");
const enc = new TextEncoder();
const FRAMES = {
  handshake: JSON.stringify({ op: "handshake", "protocol-versions": [1] }),
  handshakeBad: JSON.stringify({ op: "handshake", "protocol-versions": [99] }),
  echoSmall: JSON.stringify({ op: "echo", payload: { ping: 1 } }),
  echo100k: `{"op":"echo","payload":${DOC}}`,
  match100k: `{"op":"match-type","expected":${DOC},"actual":${DOC}}`,
  shutdown: JSON.stringify({ op: "shutdown" }),
};

/** Minimal LSP-framing client over a child process's stdio. */
class EngineClient {
  constructor(child) {
    this.child = child;
    this.buf = Buffer.alloc(0);
    this.waiters = [];
    child.stdout.on("data", (chunk) => {
      this.buf = Buffer.concat([this.buf, chunk]);
      this.drain();
    });
  }
  drain() {
    for (;;) {
      const headerEnd = this.buf.indexOf("\r\n\r\n");
      if (headerEnd < 0) return;
      const header = this.buf.subarray(0, headerEnd).toString();
      const m = /Content-Length:\s*(\d+)/i.exec(header);
      if (!m) throw new Error(`bad header: ${header}`);
      const len = Number(m[1]);
      const start = headerEnd + 4;
      if (this.buf.length < start + len) return;
      const body = this.buf.subarray(start, start + len);
      this.buf = this.buf.subarray(start + len);
      this.waiters.shift()?.(JSON.parse(body.toString()));
    }
  }
  request(frame) {
    const body = enc.encode(frame);
    return new Promise((resolve) => {
      this.waiters.push(resolve);
      this.child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
      this.child.stdin.write(body);
    });
  }
}

function spawnEngine() {
  const child = spawn(ENGINE, [], { stdio: ["pipe", "pipe", "pipe"] });
  return { child, client: new EngineClient(child) };
}

const exited = (child) => new Promise((r) => child.once("exit", (code, signal) => r({ code, signal })));

function assert(cond, msg) {
  if (!cond) throw new Error(`FAIL: ${msg}`);
  console.log(`  ok: ${msg}`);
}

const pidAlive = (pid) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
};

function stats(name, samples) {
  const s = [...samples].sort((a, b) => a - b);
  const at = (q) => s[Math.min(s.length - 1, Math.floor(q * s.length))] / 1e3;
  console.log(`  ${name}: median ${at(0.5).toFixed(1)}µs  p95 ${at(0.95).toFixed(1)}µs  min ${(s[0] / 1e3).toFixed(1)}µs`);
}

async function bench(client, name, frame, warmup, iters) {
  for (let i = 0; i < warmup; i++) await client.request(frame);
  const samples = [];
  for (let i = 0; i < iters; i++) {
    const t0 = process.hrtime.bigint();
    await client.request(frame);
    samples.push(Number(process.hrtime.bigint() - t0));
  }
  stats(name, samples);
}

// --- runner mode (orphan-test helper): spawn engine, report pid, hold forever
if (process.argv.includes("--runner")) {
  const { child, client } = spawnEngine();
  await client.request(FRAMES.handshake);
  console.log(child.pid); // parent reads this
  await sleep(60_000); // never reached: parent SIGKILLs us
  process.exit(1);
}

// --- scenario 1: handshake, including version rejection
console.log("scenario: handshake");
{
  const { child, client } = spawnEngine();
  const bad = await client.request(FRAMES.handshakeBad);
  assert(bad.err?.code === "protocol-version-unsupported", `unsupported version rejected: ${JSON.stringify(bad.err)}`);
  const good = await client.request(FRAMES.handshake);
  assert(good.ok?.["protocol-version"] === 1, `version 1 negotiated, capabilities ${JSON.stringify(good.ok?.capabilities)}`);
  child.kill();
  await exited(child);
}

// --- scenario 2: spawn-to-ready cold start
console.log("scenario: spawn-to-ready");
{
  const samples = [];
  for (let i = 0; i < 10; i++) {
    const t0 = process.hrtime.bigint();
    const { child, client } = spawnEngine();
    await client.request(FRAMES.handshake);
    samples.push(Number(process.hrtime.bigint() - t0));
    child.stdin.end();
    await exited(child);
  }
  stats("spawn-to-ready", samples);
}

// --- scenario 3: benchmark over the pipe
console.log("scenario: pipe benchmark");
{
  const { child, client } = spawnEngine();
  await client.request(FRAMES.handshake);
  await bench(client, "echo-small", FRAMES.echoSmall, 500, 5000);
  await bench(client, "echo-100k", FRAMES.echo100k, 50, 500);
  await bench(client, "match-100k", FRAMES.match100k, 20, 200);
  child.stdin.end();
  await exited(child);
}

// --- scenario 4: clean shutdown
console.log("scenario: clean shutdown");
{
  const { child, client } = spawnEngine();
  await client.request(FRAMES.handshake);
  const ack = await client.request(FRAMES.shutdown);
  assert(ack.ok?.["shutting-down"] === true, "shutdown acknowledged");
  const { code } = await exited(child);
  assert(code === 0, `exit code 0 (got ${code})`);
  assert(!pidAlive(child.pid), "no process left");
}

// --- scenario 5: stdin close without shutdown (EOF path)
console.log("scenario: EOF exit");
{
  const { child, client } = spawnEngine();
  await client.request(FRAMES.handshake);
  child.stdin.end();
  const { code } = await Promise.race([exited(child), sleep(2000).then(() => ({ code: "timeout" }))]);
  assert(code === 0, `engine exited on EOF with code 0 (got ${code})`);
}

// --- scenario 6: orphan test — SIGKILL the runner, engine must die
console.log("scenario: orphan on runner SIGKILL");
{
  const runner = spawn(process.execPath, [new URL(import.meta.url).pathname, "--runner"], { stdio: ["ignore", "pipe", "inherit"] });
  const enginePid = await new Promise((resolve) => {
    let out = "";
    runner.stdout.on("data", (d) => {
      out += d;
      const line = out.trim().split("\n").pop();
      if (/^\d+$/.test(line)) resolve(Number(line));
    });
  });
  assert(pidAlive(enginePid), `engine (pid ${enginePid}) alive under runner (pid ${runner.pid})`);
  process.kill(runner.pid, "SIGKILL");
  let waited = 0;
  while (pidAlive(enginePid) && waited < 5000) {
    await sleep(50);
    waited += 50;
  }
  assert(!pidAlive(enginePid), `engine exited within ${waited}ms of runner SIGKILL`);
  try {
    console.log(`  leftover check: ${execSync(`pgrep -c pact-engine-toy || true`).toString().trim()} pact-engine-toy processes running`);
  } catch {}
}

console.log("all scenarios passed");
