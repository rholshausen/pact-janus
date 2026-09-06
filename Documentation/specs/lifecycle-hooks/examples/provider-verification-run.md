# Worked example — a provider verification run, hook by hook

Companion to [`../spec.md`](../spec.md). One provider (`order-service`) verifying one interaction
(`get an order`) at two variants, with five hooks across five points: a token fetched once, a state
endpoint that predates Janus, a signing script, an observing script, and a teardown command. It is
deliberately the shape of plan task 5.2, so that task starts from a document.

The second variant is the interesting one: it needs a state the provider cannot produce, and the run
reports that as `state-unavailable` rather than as a failure or a pass.

## 1. What the project writes

`verifier.janus.yaml`, shown here as JSON so it validates against
[`project-config.schema.json`](../schemas/v1/project-config.schema.json) (YAML is the usual file form,
and the same document):

```json project-config
{
  "version": 1,
  "hooks": {
    "before-verification": [
      { "name": "auth-token",
        "run": { "kind": "http",
                 "url": "${AUTH_URL}/oauth/token",
                 "headers": { "authorization": "Basic ${AUTH_BASIC}" } },
        "timeout-ms": 15000 }
    ],
    "state-setup": [
      { "name": "fixtures",
        "run": { "kind": "http",
                 "url": "${PROVIDER_URL}/_pact/state",
                 "format": "pact-state-change",
                 "headers": { "authorization": "Bearer ${FIXTURE_TOKEN}" } } }
    ],
    "before-request": [
      { "name": "sign-requests",
        "run": { "kind": "script", "path": "./hooks/sign.ts" },
        "config": { "secret": "${JANUS_SIGNING_SECRET}" },
        "changes": ["parts.request.headers"] }
    ],
    "after-response": [
      { "name": "capture-trace",
        "run": { "kind": "script", "path": "./hooks/trace.js" },
        "report-data": true }
    ],
    "state-teardown": [
      { "name": "drop-fixtures",
        "run": { "kind": "exec",
                 "command": "./scripts/drop-fixtures.sh",
                 "env": { "DATABASE_URL": "${DATABASE_URL}" } },
        "when": { "state": "an order exists" } }
    ]
  }
}
```

Five things a reviewer can see without reading any code: which hooks run, at which points, in what
order, what each one may change (only `sign-requests` may change anything, and only headers), and which
secrets the run needs — by name. No secret value is in the repository, and `${…}` is the only form
there is (§7.2).

`fixtures` is the migration case. `format: pact-state-change` sends the v3/v4 provider-state body, so
the endpoint this provider already has keeps working with no change on its side (§8.4).

## 2. What the engine receives

The loader interpolates, reads `./hooks/sign.ts`, transpiles it, and inlines the JavaScript
([ADR 0014](../../../decisions/0014-hooks-are-resolved-configuration-not-callbacks.md)). This document —
no templates, no paths — is what rides in `verification/verify`'s options:

```json hook-config
{
  "version": 1,
  "hooks": {
    "before-verification": [
      { "name": "auth-token",
        "run": { "kind": "http",
                 "url": "https://auth.internal.example/oauth/token",
                 "headers": { "authorization": "Basic b3JkZXJzOnMzY3IzdA==" } },
        "timeout-ms": 15000 }
    ],
    "state-setup": [
      { "name": "fixtures",
        "run": { "kind": "http",
                 "url": "https://orders.staging.example/_pact/state",
                 "format": "pact-state-change",
                 "headers": { "authorization": "Bearer eyJhbGciOiJIUzI1NiJ9.fixture" } } }
    ],
    "before-request": [
      { "name": "sign-requests",
        "run": { "kind": "script",
                 "source": "function hook(ctx) {\n  const headers = janus.json(ctx.parts.request.headers) || {};\n  const token = ctx.run.data['auth-token'].access_token;\n  const path = janus.text(ctx.parts.request.path);\n  headers.authorization = ['Bearer', token].join(' ');\n  headers['x-signature'] = sign(path + ctx.config.secret);\n  return { outcome: 'ok', changes: { 'parts.request.headers': janus.slot(headers) } };\n}\n\nfunction sign(s) {\n  let acc = 2166136261;\n  for (let i = 0; i < s.length; i++) { acc ^= s.charCodeAt(i); acc = Math.imul(acc, 16777619) >>> 0; }\n  return acc.toString(16).padStart(8, '0');\n}\n",
                 "entry": "hook" },
        "config": { "secret": "s3cr3t-signing-key" },
        "changes": ["parts.request.headers"] }
    ],
    "after-response": [
      { "name": "capture-trace",
        "run": { "kind": "script",
                 "source": "function hook(ctx) {\n  const headers = janus.json(ctx.parts.response.headers) || {};\n  return { outcome: 'ok', data: { trace: headers['x-trace-id'] } };\n}\n" },
        "report-data": true }
    ],
    "state-teardown": [
      { "name": "drop-fixtures",
        "run": { "kind": "exec",
                 "command": "./scripts/drop-fixtures.sh",
                 "env": { "DATABASE_URL": "postgres://ci@db.internal.example/orders" } },
        "when": { "state": "an order exists" } }
    ]
  }
}
```

The two documents differ in exactly the four ways §7.1 lists, which is why they are two schemas: an
engine that received the first one would have to expand variables and read files to run it, and neither
is something the WASM-embedded engine can do.

## 3. Before the run: `before-verification`

```json hook-context
{ "point": "before-verification",
  "role": "provider",
  "run": { "id": "s-7",
           "consumer": { "name": "orders-ui" },
           "provider": { "name": "order-service" },
           "data": { } },
  "config": { },
  "mutable": [],
  "deadline-ms": 15000 }
```

The `http` hook POSTs that document to the token endpoint and reads the response body as its result:

```json hook-result
{ "outcome": "ok",
  "data": { "access_token": "eyJhbGciOiJIUzI1NiJ9.run", "expires_in": 3600 } }
```

That `data` lands in `run.data["auth-token"]` and is visible to every later hook in the run (§4.4). It
is fetched once here rather than once per exchange, and — because the entry did not set `report-data` —
it appears in no event, no report and no log (§7.3).

## 4. Variant 1: the exchange that works

The interaction has two selected variants. The first has `shippedAt` present, which the provider's
fixtures can produce.

### 4.1 `state-setup`

```json hook-context
{ "point": "state-setup",
  "role": "provider",
  "run": { "id": "s-7", "provider": { "name": "order-service" },
           "data": { "auth-token": { "access_token": "eyJhbGciOiJIUzI1NiJ9.run", "expires_in": 3600 } } },
  "interaction": { "description": "get an order",
                   "transport": { "kind": "http", "mode": "passive" } },
  "variant": { "id": "base",
               "assignment": [ { "dimension": "response.body.shippedAt#presence", "point": "present" },
                               { "dimension": "response.body.status#value", "point": "PENDING" } ] },
  "state": { "name": "an order exists", "params": { "id": "42", "shipped": true } },
  "exchange": { "id": "x-1", "data": { } },
  "config": { },
  "mutable": [],
  "deadline-ms": 5000 }
```

`state.params` arrives resolved: `id` is the state's literal parameter and `shipped` is the
variant-bound one, resolved from this variant's assignment by variant semantics §6.4. The hook is handed
one state, not a list, and the engine calls it again for the next one.

Because the entry set `format: pact-state-change`, what actually goes over the wire to the provider's
existing endpoint is the v3 body — `{ "state": "an order exists", "params": { "id": "42",
"shipped": true }, "action": "setup" }` — and its empty 200 is read as:

```json hook-result
{ "outcome": "ok" }
```

### 4.2 `before-request`

```json hook-context
{ "point": "before-request",
  "role": "provider",
  "run": { "id": "s-7",
           "data": { "auth-token": { "access_token": "eyJhbGciOiJIUzI1NiJ9.run", "expires_in": 3600 } } },
  "interaction": { "description": "get an order",
                   "transport": { "kind": "http", "mode": "passive" } },
  "variant": { "id": "base",
               "assignment": [ { "dimension": "response.body.shippedAt#presence", "point": "present" },
                               { "dimension": "response.body.status#value", "point": "PENDING" } ] },
  "exchange": { "id": "x-1", "data": { } },
  "parts": { "request": { "method": { "content": "GET" },
                          "path": { "content": "/orders/42" },
                          "headers": { "content": { "accept": ["application/json"] } } } },
  "endpoint": { "kind": "http", "host": "orders.staging.example", "port": 443, "tls": true },
  "config": { "secret": "s3cr3t-signing-key" },
  "mutable": ["parts.request.headers"],
  "deadline-ms": 5000 }
```

The script reads the token out of `run.data`, signs the path with the secret from its own `config`, and
replaces one slot:

```json hook-result
{ "outcome": "ok",
  "changes": { "parts.request.headers": { "content": { "accept": ["application/json"],
                                                        "authorization": "Bearer eyJhbGciOiJIUzI1NiJ9.run",
                                                        "x-signature": "5c3a2f81" } } } }
```

`mutable` in the context and `changes` in the entry say the same thing from two directions, and the
engine applies the intersection (§4.3). A result that tried to change `parts.request.path` here would be
refused naming the path, and the whole result discarded — no half-applied changes.

### 4.3 `after-response`

```json hook-context
{ "point": "after-response",
  "role": "provider",
  "run": { "id": "s-7" },
  "interaction": { "description": "get an order",
                   "transport": { "kind": "http", "mode": "passive" } },
  "variant": { "id": "base",
               "assignment": [ { "dimension": "response.body.shippedAt#presence", "point": "present" },
                               { "dimension": "response.body.status#value", "point": "PENDING" } ] },
  "exchange": { "id": "x-1", "data": { } },
  "parts": { "request": { "method": { "content": "GET" },
                          "path": { "content": "/orders/42" },
                          "headers": { "content": { "accept": ["application/json"],
                                                     "authorization": "Bearer eyJhbGciOiJIUzI1NiJ9.run",
                                                     "x-signature": "5c3a2f81" } } },
             "response": { "status": { "content": 200 },
                           "headers": { "content": { "content-type": ["application/json"],
                                                      "x-trace-id": ["c0ffee-1"] } },
                           "body": { "content": "eyJpZCI6ICI0MiIsICJzdGF0dXMiOiAiUEVORElORyJ9",
                                     "encoded": "base64",
                                     "content-type": "application/json" } } },
  "config": { },
  "mutable": [],
  "deadline-ms": 5000 }
```

`mutable` is empty, and that is the design (§3.6): this hook observes. It captures the trace id and
returns it as data, which its entry opted into reporting:

```json hook-result
{ "outcome": "ok", "data": { "trace": ["c0ffee-1"] } }
```

The response body arrives as octets in the wrapper form design 2.6 §4 fixes — a transport carries
octets, not documents, and turning them into a document is the content component's job, which happens
after this point on the way into matching.

### 4.4 `state-teardown`

```json hook-context
{ "point": "state-teardown",
  "role": "provider",
  "run": { "id": "s-7" },
  "interaction": { "description": "get an order",
                   "transport": { "kind": "http", "mode": "passive" } },
  "variant": { "id": "base",
               "assignment": [ { "dimension": "response.body.shippedAt#presence", "point": "present" },
                               { "dimension": "response.body.status#value", "point": "PENDING" } ] },
  "state": { "name": "an order exists", "params": { "id": "42", "shipped": true } },
  "exchange": { "id": "x-1", "data": { }, "outcome": "passed" },
  "config": { },
  "mutable": [],
  "deadline-ms": 5000 }
```

The `exec` hook gets that on stdin with an environment consisting of exactly `DATABASE_URL`, and exits
0 with an empty stdout, which the engine reads as:

```json hook-result
{ "outcome": "ok" }
```

## 5. Variant 2: a state the provider cannot produce

The second variant has `shippedAt` absent while `status` is `SHIPPED` — a combination the consumer
declared and the order service cannot make.

```json hook-context
{ "point": "state-setup",
  "role": "provider",
  "run": { "id": "s-7" },
  "interaction": { "description": "get an order",
                   "transport": { "kind": "http", "mode": "passive" } },
  "variant": { "id": "response.body.shippedAt#presence=absent;response.body.status#value=SHIPPED",
               "assignment": [ { "dimension": "response.body.shippedAt#presence", "point": "absent" },
                               { "dimension": "response.body.status#value", "point": "SHIPPED" } ] },
  "state": { "name": "an order exists", "params": { "id": "42", "shipped": false } },
  "exchange": { "id": "x-2", "data": { } },
  "config": { },
  "mutable": [],
  "deadline-ms": 5000 }
```

The provider's endpoint answers 200 with a body saying it cannot:

```json hook-result
{ "outcome": "unsupported",
  "error": { "code": "state-unreachable",
             "message": "an order with status SHIPPED always has shippedAt set" } }
```

That is not a failure and not a pass. The variant is reported `state-unavailable`, the exchange does not
run, teardown does (with `exchange.outcome: "state-unavailable"`), and the run continues to the next
variant — an unproducible state is a fact about one region of the variant space, and stopping would hide
the rest. It fails the run unless waived per variant with a reason, and no hook setting can turn it into
a pass ([ADR 0009](../../../decisions/0009-variant-bound-provider-state-parameters.md), §5.5).

Note what the endpoint had to know to produce this: nothing about Janus. A v3 state endpoint that
returns a body with `outcome: "unsupported"` reaches this path; one that 500s produces `failed`, which
is a bug report rather than a contract finding, and the two are kept apart on purpose.

## 6. What the run recorded

Each invocation is one `verification/hook` event as it happens (protocol §9.6):

```json hook-invocation
{ "point": "before-request",
  "hook": "sign-requests",
  "implementation": "script",
  "outcome": "ok",
  "exchange": "x-1",
  "interaction": "get an order",
  "variant": "base",
  "changed": ["parts.request.headers"],
  "duration-ms": 1 }
```

`changed` carries the path and not the value — the reader needs to know a header was rewritten and by
whom; the bearer token that was written into it belongs in no log (§10.2).

The summary carries the whole report:

```json hook-report
{ "invocations": [
    { "point": "before-verification", "hook": "auth-token", "implementation": "http",
      "outcome": "ok", "changed": [], "duration-ms": 212 },
    { "point": "state-setup", "hook": "fixtures", "implementation": "http", "outcome": "ok",
      "exchange": "x-1", "interaction": "get an order", "variant": "base",
      "state": "an order exists", "changed": [], "duration-ms": 34 },
    { "point": "before-request", "hook": "sign-requests", "implementation": "script", "outcome": "ok",
      "exchange": "x-1", "interaction": "get an order", "variant": "base",
      "changed": ["parts.request.headers"], "duration-ms": 1 },
    { "point": "after-response", "hook": "capture-trace", "implementation": "script", "outcome": "ok",
      "exchange": "x-1", "interaction": "get an order", "variant": "base",
      "changed": [], "duration-ms": 1, "data": { "trace": ["c0ffee-1"] } },
    { "point": "state-teardown", "hook": "drop-fixtures", "implementation": "exec", "outcome": "ok",
      "exchange": "x-1", "interaction": "get an order", "variant": "base",
      "state": "an order exists", "changed": [], "duration-ms": 96 },
    { "point": "state-setup", "hook": "fixtures", "implementation": "http", "outcome": "unsupported",
      "exchange": "x-2", "interaction": "get an order",
      "variant": "response.body.shippedAt#presence=absent;response.body.status#value=SHIPPED",
      "state": "an order exists", "changed": [], "duration-ms": 28,
      "effect": "state-unavailable",
      "error": { "code": "state-unreachable",
                 "message": "an order with status SHIPPED always has shippedAt set" } },
    { "point": "state-teardown", "hook": "drop-fixtures", "implementation": "exec", "outcome": "ok",
      "exchange": "x-2", "interaction": "get an order",
      "variant": "response.body.shippedAt#presence=absent;response.body.status#value=SHIPPED",
      "state": "an order exists", "changed": [], "duration-ms": 91 } ] }
```

Seven invocations, one line each, in run order. `capture-trace` shows its data because its entry asked
to; `auth-token` does not, and the difference is one boolean in the configuration rather than a policy
somebody has to remember.

## 7. What a failure looks like instead

If `sign-requests` throws, the invocation is `errored`, the point's default policy (`fail-exchange`)
applies, the exchange fails with the hook's error as its cause, and `state-teardown` still runs:

```json hook-invocation
{ "point": "before-request", "hook": "sign-requests", "implementation": "script",
  "outcome": "errored", "exchange": "x-1", "interaction": "get an order", "variant": "base",
  "changed": [], "duration-ms": 2, "effect": "failed-exchange",
  "error": { "code": "hook-script-threw", "category": "component",
             "message": "TypeError: cannot read property 'access_token' of undefined" } }
```

The alternative — letting the exchange proceed unsigned — produces a 401 that the report attributes to
the provider, and a team looking at the wrong service. That is the reasoning behind every default in
§5.2.

Configuration problems never get this far. An unset `${JANUS_SIGNING_SECRET}` fails at load, before the
first exchange, naming the variable:

```json engine-error
{ "code": "hook-unresolved",
  "category": "document",
  "message": "hook 'sign-requests': environment variable JANUS_SIGNING_SECRET is not set",
  "details": { "hook": "sign-requests", "variable": "JANUS_SIGNING_SECRET" } }
```

And an embedding that cannot spawn processes refuses the `exec` hook by name rather than skipping it:

```json engine-error
{ "code": "hook-unavailable",
  "category": "component",
  "message": "hook 'drop-fixtures' needs implementation 'exec', which this embedding cannot run",
  "details": { "hook": "drop-fixtures", "kind": "exec",
               "implementations": ["component", "script", "http"] } }
```

which is the negotiated capability of §8.5, declared in the handshake:

```json capability
{ "hooks": { "implementations": ["component", "script", "http"] },
  "components": { "loaders": ["native"] } }
```
