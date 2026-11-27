#!/usr/bin/env node
// A provider that answers a corpora/verifier fixture's scripted routes, for a job that must run with no
// Rust toolchain (release.yml's `plugin-bridge` job, ADR 0112 decision 7). tools/verifier-diff's own
// scripted provider is the full one; this is the part of it the plugin fixtures use: a route is
// recognised by its method, its path and, when it has one, its JSON body, compared whole. A fixture whose
// routes need more (states, a query, request headers, patterns, a body that is not JSON, a delay) is
// refused rather than answered differently.
//
//   node scripted-provider.mjs <fixture.json>
//
// Listens on a free loopback port, prints its base URL as the first line on stdout, and serves until it
// is killed.

import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { isDeepStrictEqual } from "node:util";

const [file] = process.argv.slice(2);
if (!file) {
  console.error("usage: scripted-provider.mjs <fixture.json>");
  process.exit(2);
}
const fixture = JSON.parse(readFileSync(file, "utf8"));
if (fixture.provider?.kind !== "scripted") {
  console.error(`${file}: the provider is not scripted`);
  process.exit(2);
}
const routes = fixture.provider.routes ?? [];
for (const [i, route] of routes.entries()) {
  const unread = [
    ...Object.keys(route).filter((key) => !["request", "response"].includes(key)),
    ...Object.keys(route.request).filter((key) => !["method", "path", "body"].includes(key)),
    ...Object.keys(route.response).filter((key) => !["status", "headers", "body"].includes(key)),
    ...(route.request.body && !("json" in route.request.body) ? ["request.body (not JSON)"] : []),
  ];
  if (unread.length > 0) {
    console.error(`${file}: route ${i} needs what this provider does not read: ${unread.join(", ")}`);
    process.exit(2);
  }
}

function bytes(body) {
  if (!body) return Buffer.alloc(0);
  if ("json" in body) return Buffer.from(JSON.stringify(body.json));
  if ("text" in body) return Buffer.from(body.text);
  if ("base64" in body) return Buffer.from(body.base64, "base64");
  throw new Error(`a body that is none of json, text or base64: ${JSON.stringify(body)}`);
}

function recognises({ request }, method, path, body) {
  if (request.method.toUpperCase() !== method || request.path !== path) return false;
  if (!request.body) return true;
  try {
    return isDeepStrictEqual(JSON.parse(body.toString("utf8")), request.body.json);
  } catch {
    return false;
  }
}

const server = createServer((req, res) => {
  const chunks = [];
  req.on("data", (chunk) => chunks.push(chunk));
  req.on("end", () => {
    const path = new URL(req.url, "http://provider").pathname;
    const route = routes.find((candidate) => recognises(candidate, req.method, path, Buffer.concat(chunks)));
    if (!route) {
      console.error(`no route recognises ${req.method} ${req.url}`);
      res.writeHead(500, { "Content-Type": "text/plain" }).end(`the scripted provider recognises no ${req.method} ${path} with this body`);
      return;
    }
    // One header line per value, as tools/verifier-diff's provider sends them.
    for (const [name, values] of Object.entries(route.response.headers ?? {})) {
      res.setHeader(name, values);
    }
    res.statusCode = route.response.status;
    res.end(bytes(route.response.body));
  });
});
server.listen(0, "127.0.0.1", () => {
  console.log(`http://127.0.0.1:${server.address().port}`);
});
