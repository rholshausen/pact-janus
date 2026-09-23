// A `tcp` transport: newline-delimited messages over a raw TCP socket — JSON lines, as JSON-RPC
// over TCP and a good many internal services speak it. It needs exactly what a WASM sandbox
// withholds (a listening socket, a long-lived server), which is the case the subprocess binding
// exists for (component-interfaces spec §9.3). Written from the spec's §5 table, in Node, with no
// dependencies: the pact-plugins architecture — a plugin in any language, driven by the engine —
// on this project's wire.
//
// Parts: `request.body` is one line in, `response.body` one line out. Both are content (§5.5), so
// the transport carries octets and the declared content component — JSON here — does the parsing.
import net from "node:net";
import { ComponentError, serve } from "./pipe.mjs";

const instances = new Map(); // instance -> { role, server?, sockets, arrivals, waiters, events, target? }
let nextEvent = 0;

const fail = (message, details) => new ComponentError("transport-failed", "component", message, details);
const live = (instance) => {
  const state = instances.get(instance);
  if (!state) throw fail(`no instance '${instance}'`, { instance });
  return state;
};
const octets = (line) => ({ content: Buffer.from(line).toString("base64"), encoded: "base64" });
const text = (slot) => {
  if (!slot) return "";
  if (slot.encoded === "base64") return Buffer.from(slot.content, "base64").toString();
  return typeof slot.content === "string" ? slot.content : JSON.stringify(slot.content);
};

function lines(socket, onLine) {
  let pending = "";
  socket.on("data", (chunk) => {
    pending += chunk.toString();
    let at;
    while ((at = pending.indexOf("\n")) >= 0) {
      onLine(pending.slice(0, at).replace(/\r$/, ""));
      pending = pending.slice(at + 1);
    }
  });
}

function arrive(state, arrival) {
  const waiter = state.waiters.shift();
  if (waiter) waiter(arrival);
  else state.arrivals.push(arrival);
}

serve({
  "component/hello": () => ({
    "component-protocol-version": 1,
    component: { name: "tcp", version: "0.1.0" },
    interfaces: ["transport"],
    contributes: {
      transports: [
        { kind: "tcp", roles: ["serve", "drive"], "content-slots": { request: ["body"], response: ["body"] } },
      ],
    },
    capabilities: {},
  }),

  "transport/start": async ({ instance, role, options = {} }) => {
    if (role === "drive") {
      instances.set(instance, { role, target: { host: options.host ?? "127.0.0.1", port: options.port } });
      return { endpoint: { kind: "tcp", host: options.host ?? "127.0.0.1", port: options.port } };
    }
    const state = { role, sockets: new Set(), arrivals: [], waiters: [], events: new Map() };
    state.server = net.createServer((socket) => {
      state.sockets.add(socket);
      socket.on("close", () => state.sockets.delete(socket));
      socket.on("error", () => {});
      lines(socket, (line) => {
        const event = `e-${++nextEvent}`;
        state.events.set(event, socket);
        arrive(state, { event, parts: { request: { body: octets(line) } }, "expects-reply": true });
      });
    });
    await new Promise((resolve, reject) => {
      state.server.once("error", reject);
      state.server.listen(options.port ?? 0, options.host ?? "127.0.0.1", resolve);
    });
    // After listening, a server error must not be uncaught either: one process hosts every instance.
    state.server.on("error", (e) => console.error(`${instance}: ${e.message}`));
    instances.set(instance, state);
    const { address, port } = state.server.address();
    return { endpoint: { kind: "tcp", host: address, port } };
  },

  "transport/stop": ({ instance }) => {
    const state = instances.get(instance);
    instances.delete(instance);
    if (state?.server) {
      state.server.close();
      for (const socket of state.sockets) socket.destroy();
      for (const waiter of state.waiters) waiter(undefined);
    }
    return {};
  },

  "transport/poll-inbound": ({ instance, "timeout-ms": timeout = 0 }) => {
    const state = live(instance);
    if (state.arrivals.length) return { inbound: state.arrivals.shift() };
    return new Promise((resolve) => {
      const waiter = (arrival) => {
        clearTimeout(timer);
        resolve(arrival ? { inbound: arrival } : {});
      };
      const timer = setTimeout(() => {
        state.waiters.splice(state.waiters.indexOf(waiter), 1);
        resolve({});
      }, timeout);
      state.waiters.push(waiter);
    });
  },

  "transport/reply": ({ instance, event, parts }) => {
    const socket = live(instance).events.get(event);
    if (!socket) throw fail(`no arrival '${event}'`, { event });
    socket.write(text(parts?.response?.body) + "\n");
    return {};
  },

  // Nothing to acknowledge on a socket; an arrival nobody replied to gets an error line, so the
  // client is not left waiting for a reply that is never coming.
  "transport/dispose": ({ instance, event, disposition }) => {
    const state = live(instance);
    const socket = state.events.get(event);
    state.events.delete(event);
    if (socket && disposition !== "accept") socket.write(JSON.stringify({ "janus-disposition": disposition }) + "\n");
    return {};
  },

  "transport/send": ({ instance, parts, "await-reply": awaitReply, "timeout-ms": timeout = 10000 }) => {
    const { target } = live(instance);
    return new Promise((resolve, reject) => {
      const socket = net.connect(target.port, target.host);
      const timer = setTimeout(() => {
        socket.destroy();
        reject(fail(`no reply from ${target.host}:${target.port} within ${timeout} ms`));
      }, timeout);
      socket.on("error", (e) => {
        clearTimeout(timer);
        reject(fail(`${target.host}:${target.port}: ${e.message}`));
      });
      socket.on("connect", () => {
        socket.write(text(parts?.request?.body) + "\n");
        if (!awaitReply) {
          clearTimeout(timer);
          socket.end();
          resolve({});
        }
      });
      if (awaitReply)
        lines(socket, (line) => {
          clearTimeout(timer);
          socket.destroy();
          resolve({ reply: { response: { body: octets(line) } } });
        });
    });
  },
});
