// A component built to break the subprocess binding's promises, one way per `transport/start`
// option `do` — the out-of-process twin of engine/component-host's WASM `misbehaving` fixture.
import net from "node:net";
import { serve } from "./pipe.mjs";

serve({
  "component/hello": () => ({
    "component-protocol-version": 1,
    component: { name: "misbehaving", version: "0.1.0" },
    interfaces: ["transport"],
    contributes: { transports: [{ kind: "misbehaving", roles: ["serve"] }] },
    capabilities: {},
    // Not in the handshake schema; the open world lets it through. The orphan test needs it.
    pid: process.pid,
  }),
  "transport/start": ({ options = {} }) => {
    switch (options.do) {
      case "hang":
        return new Promise(() => {}); // never answers
      case "spin":
        for (;;); // never answers, and never reads stdin again either
      case "exit":
        process.exit(3);
      case "garbage":
        // A well-framed body that is not JSON: the stream stays in sync, the call is never answered.
        process.stdout.write("Content-Length: 9\r\n\r\nnot json!");
        return undefined;
      case "noise":
        // Stray output an author's print would produce, then a proper answer.
        process.stdout.write("got here\n"); // no colon: not even mistakable for a header
        return { endpoint: { pid: process.pid } };
      case "listen":
        // A transport instance's real state: a listening socket, which lives in this process only.
        return new Promise((resolve) => {
          const server = net.createServer((socket) => socket.end("hello\n"));
          server.listen(0, "127.0.0.1", () => resolve({ endpoint: { port: server.address().port } }));
        });
      case "env":
        return { endpoint: { env: Object.keys(process.env).sort() } };
      default:
        return { endpoint: { pid: process.pid } };
    }
  },
  "transport/stop": () => ({}),
});

