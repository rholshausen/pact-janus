// The component side of the subprocess binding (component-interfaces spec §9.3): the engine
// protocol's own Content-Length framing over stdio, request frames in, response frames out.
// Dependency-free on purpose — this file is the "30-60 lines per language" claim, measured.
// stdout carries frames and nothing else, so console.log goes to stderr: the one line that stops
// an author's first debug print from corrupting the pipe.
console.log = console.error;

export class ComponentError extends Error {
  constructor(code, category, message, details) {
    super(message);
    Object.assign(this, { code, category, details });
  }
}

export function send(frame) {
  const body = Buffer.from(JSON.stringify(frame));
  process.stdout.write(Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`), body]));
}

export function serve(handlers) {
  let buffer = Buffer.alloc(0);
  let greeted = false;
  const answer = async ({ id, op, body }) => {
    try {
      if (op !== "component/hello" && !greeted)
        throw new ComponentError("handshake-required", "protocol", "component/hello comes first");
      const handler = handlers[op];
      if (!handler)
        throw new ComponentError("operation-unsupported", "protocol", `'${op}' is not answered here`, { op });
      const ok = await handler(body ?? {});
      if (op === "component/hello") greeted = true;
      if (ok !== undefined) send({ type: "response", id, ok });
    } catch (e) {
      const { code = "internal", category = "internal", message = String(e), details } = e;
      send({ type: "response", id, error: { code, category, message, details } });
    }
  };
  process.stdin.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    for (;;) {
      const end = buffer.indexOf("\r\n\r\n");
      if (end < 0) return;
      const length = /content-length:\s*(\d+)/i.exec(buffer.subarray(0, end).toString());
      const start = end + 4;
      if (!length) { buffer = buffer.subarray(start); continue; } // no length, no frame: resync
      if (buffer.length < start + Number(length[1])) return;
      const body = buffer.subarray(start, start + Number(length[1]));
      buffer = buffer.subarray(start + Number(length[1]));
      let frame;
      try { frame = JSON.parse(body); } catch {
        send({ type: "response", id: null, error: { code: "malformed-frame", category: "protocol", message: "not JSON" } });
        continue;
      }
      answer(frame);
    }
  });
  // Spec §9.3: stdin is the component's lease on life. EOF means the engine is gone.
  process.stdin.on("end", () => process.exit(0));
}
