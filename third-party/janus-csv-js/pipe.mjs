// The component end of the subprocess binding (component-interfaces spec §9.3), with no dependencies:
// request frames in on stdin, each a `Content-Length` header section and then exactly that many bytes of
// JSON; response frames out on stdout, framed the same way. Logs go to stderr, because stdout carries
// frames and nothing else. When stdin ends, the process exits: the engine has gone, or has finished with
// this component.
//
// `serve(handle)` calls `handle(op, body)` for each request, in order, and writes what it returns as the
// response's `ok`, or what it throws as its `error`. A thrown value with a `code` is a ComponentError;
// anything else is answered as `internal`.

// Every author's first debugging move writes to stdout, which would corrupt the frame channel.
console.log = (...args) => console.error(...args);

export function serve(handle) {
  let buffered = Buffer.alloc(0);
  process.stdin.on("data", (chunk) => {
    buffered = Buffer.concat([buffered, chunk]);
    for (;;) {
      const end = buffered.indexOf("\r\n\r\n");
      if (end < 0) return;
      const headers = buffered.subarray(0, end).toString("latin1").split("\r\n");
      const length = headers
        .map((line) => line.match(/^content-length\s*:\s*(\d+)\s*$/i))
        .find(Boolean);
      if (!length) {
        // A header section with no length: skip it, and the next frame resynchronises the stream.
        buffered = buffered.subarray(end + 4);
        continue;
      }
      const size = Number(length[1]);
      if (buffered.length < end + 4 + size) return;
      const body = buffered.subarray(end + 4, end + 4 + size);
      buffered = buffered.subarray(end + 4 + size);
      write(answer(handle, body));
    }
  });
  process.stdin.on("end", () => process.exit(0));
}

function answer(handle, bytes) {
  let frame;
  try {
    frame = JSON.parse(bytes.toString("utf8"));
  } catch (err) {
    return { type: "response", id: "", error: failure("malformed-frame", "protocol", `frame is not JSON: ${err.message}`) };
  }
  const id = typeof frame?.id === "string" ? frame.id : "";
  if (frame?.type !== "request" || typeof frame.op !== "string" || !isObject(frame.body)) {
    return {
      type: "response",
      id,
      error: failure("malformed-frame", "protocol", "a request frame needs a string 'op' and an object 'body'"),
    };
  }
  try {
    return { type: "response", id, ok: handle(frame.op, frame.body) };
  } catch (err) {
    if (err && typeof err.code === "string" && typeof err.category === "string") {
      return { type: "response", id, error: err };
    }
    return { type: "response", id, error: failure("internal", "internal", String(err?.stack ?? err)) };
  }
}

function write(frame) {
  const body = Buffer.from(stringify(frame), "utf8");
  process.stdout.write(Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, "latin1"), body]));
}

/** A ComponentError (spec §11.1), to throw. */
export function failure(code, category, message, details) {
  const error = { code, category, message };
  if (details !== undefined) error.details = details;
  return error;
}

export function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

/**
 * JSON, with a `Map` written as an object in its own order. A JavaScript object puts a member whose name
 * is an array index first, whatever order it was added in; a `Map` keeps the order a document's members
 * arrived in, which the engine keeps too (ADR 0037).
 */
export function stringify(value) {
  if (value instanceof Map) {
    return `{${[...value].map(([name, member]) => `${JSON.stringify(name)}:${stringify(member)}`).join(",")}}`;
  }
  if (Array.isArray(value)) return `[${value.map((item) => (item === undefined ? "null" : stringify(item))).join(",")}]`;
  if (isObject(value)) {
    return `{${Object.entries(value)
      .filter(([, member]) => member !== undefined)
      .map(([name, member]) => `${JSON.stringify(name)}:${stringify(member)}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}
