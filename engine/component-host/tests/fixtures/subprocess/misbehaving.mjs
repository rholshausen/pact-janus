// A subprocess component built to break the binding's rules (component-interfaces spec §3.4, §9.3, §11.2;
// ADR 0108), for `tests/subprocess.rs`: a content interface whose content type says how to misbehave, and a
// `line` transport whose instances hold real listening sockets, so what dies with the process can be seen.
// Plain Node, no dependencies. Requests are answered as they complete, out of order, correlated by id.
//
//   content/decode, by content type:
//     x/fine       answers "ok"
//     x/hang       never answers (the event loop stays alive)
//     x/spin       never answers, in a busy loop that never reads stdin again
//     x/exit       exits with status 3 without answering
//     x/malformed  answers with a well-framed body that is not JSON
//     x/stray      writes a line that is not a frame to stdout, then answers "ok"
//     x/env        answers with its environment's variable names, sorted
//     x/cwd        answers with its working directory
//     x/pid        answers with its process id
//   transport `line`: start listens on 127.0.0.1, stop closes; send never answers.
//
// Arguments: `--pid-file <path>` writes the process id there at start; `--changes-version <path>` answers
// the handshake as 1.0.0 the first time and 2.0.0 every time after, which a marker file at <path> records.

import { createServer } from "node:net";
import { existsSync, writeFileSync } from "node:fs";

const argument = (name) => {
  const at = process.argv.indexOf(name);
  return at < 0 ? undefined : process.argv[at + 1];
};
const pidFile = argument("--pid-file");
if (pidFile) writeFileSync(pidFile, String(process.pid));
let version = "1.0.0";
const marker = argument("--changes-version");
if (marker) {
  if (existsSync(marker)) version = "2.0.0";
  else writeFileSync(marker, "started");
}

const servers = new Map();

function write(text) {
  const body = Buffer.from(text, "utf8");
  process.stdout.write(Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`), body]));
}

function respond(id, result) {
  write(JSON.stringify({ type: "response", id, ...result }));
}

async function handle(op, body) {
  switch (op) {
    case "component/hello":
      return {
        ok: {
          "component-protocol-version": 1,
          component: { name: "misbehaving", version },
          interfaces: ["content", "transport"],
          contributes: {
            "content-types": [{ "media-type": "x/*" }],
            transports: [{ kind: "line", roles: ["serve"], "exchange-parts": { initiating: "ask", answering: "answer" } }],
          },
        },
      };
    case "content/decode":
      return decode(body["content-type"]);
    case "transport/start":
      return start(body.instance);
    case "transport/stop": {
      servers.get(body.instance)?.close();
      servers.delete(body.instance);
      return { ok: {} };
    }
    case "transport/poll-inbound":
      await new Promise((resolve) => setTimeout(resolve, body["timeout-ms"] ?? 0));
      return { ok: {} };
    case "transport/send":
      return new Promise(() => {});
    default:
      return { error: { code: "operation-unsupported", category: "protocol", message: op, details: { op } } };
  }
}

async function decode(type) {
  const ok = (document) => ({ ok: { document } });
  switch (type) {
    case "x/hang":
      return new Promise(() => {});
    case "x/spin":
      for (;;) {}
    case "x/exit":
      process.exit(3);
    case "x/malformed":
      write("this is not JSON");
      return new Promise(() => {});
    case "x/stray":
      process.stdout.write("debug: the decode was called\n");
      return ok("ok");
    case "x/env":
      return ok(Object.keys(process.env).sort());
    case "x/cwd":
      return ok(process.cwd());
    case "x/pid":
      return ok(process.pid);
    default:
      return ok("ok");
  }
}

function start(instance) {
  return new Promise((resolve) => {
    const server = createServer((socket) => {
      // Every socket error is survivable (spike 8.3 finding 9): one reset must not end the process.
      socket.on("error", () => {});
      socket.end("hello\n");
    });
    server.on("error", () => {});
    server.listen(0, "127.0.0.1", () => {
      servers.set(instance, server);
      resolve({ ok: { endpoint: { host: "127.0.0.1", port: server.address().port } } });
    });
  });
}

let buffered = Buffer.alloc(0);
process.stdin.on("data", (chunk) => {
  buffered = Buffer.concat([buffered, chunk]);
  for (;;) {
    const end = buffered.indexOf("\r\n\r\n");
    if (end < 0) return;
    const length = Number(buffered.subarray(0, end).toString().match(/content-length:\s*(\d+)/i)[1]);
    if (buffered.length < end + 4 + length) return;
    const frame = JSON.parse(buffered.subarray(end + 4, end + 4 + length).toString());
    buffered = buffered.subarray(end + 4 + length);
    handle(frame.op, frame.body).then((result) => respond(frame.id, result));
  }
});
process.stdin.on("end", () => process.exit(0));
