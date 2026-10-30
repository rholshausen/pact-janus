# Worked example — verifying a provider inside a test

An SDK's provider verification ([ADR 0055](../../../decisions/0055-verify-a-provider-in-a-test-through-the-run-with-handlers-as-loopback-hooks.md)):
the project's configuration loaded through the engine's host (§8.8), the test's own state handler
and request filter added as loopback `http` hooks, and one run. Every ```` ```json ```` block is one
frame, validated against the v1 schemas by `cargo test -p pact_janus_schema_compat`. `→` is host to
engine, `←` is engine to host.

`→`

```json
{ "type": "request", "id": "r-1", "op": "engine/hello",
  "body": { "protocol-versions": [1], "host": { "name": "pact-janus-typescript", "version": "0.0.0" } } }
```

`←` this engine has a loader:

```json
{ "type": "response", "id": "r-1",
  "ok": { "protocol-version": 1, "engine": { "name": "janus-engine", "version": "0.4.1" },
          "capabilities": { "config-loading": {}, "broker": {},
                            "components": { "loaders": ["in-tree", "wasm"] } } } }
```

`→` the test named `verifier.janus.yaml`; the SDK made its path absolute against its own working
directory:

```json
{ "type": "request", "id": "r-2", "op": "config/load",
  "body": { "path": "/work/orders-api/verifier.janus.yaml" } }
```

`←` the resolved document. `${AUTH_URL}` came from the engine's environment, which is the test
process's, since the SDK started the engine:

```json
{ "type": "response", "id": "r-2",
  "ok": { "config": { "version": 1,
                      "hooks": { "before-verification": [
                        { "name": "auth-token",
                          "run": { "kind": "http", "url": "https://auth.internal/token" } } ] },
                      "components": [] } } }
```

`→` the run. The configuration's hooks come first at each point; the SDK's follow, named by kind
and position, each at its own path on the loopback endpoint the SDK serves for this run, with the
run's token:

```json
{ "type": "request", "id": "r-3", "op": "verification/verify",
  "body": { "source": { "kind": "inline",
                        "contracts": [ { "consumer": { "name": "web-app" }, "provider": { "name": "orders-api" },
                                         "interactions": [ { "description": "get an order",
                                                             "providerStates": [ { "name": "an order exists" } ] } ],
                                         "metadata": { "pactSpecification": { "version": "3.0.0" } } } ] },
            "target": { "transports": [ { "transport": "http", "options": { "base-url": "http://127.0.0.1:8080" } } ],
                        "components": [],
                        "hooks": { "version": 1, "hooks": {
                          "before-verification": [
                            { "name": "auth-token", "run": { "kind": "http", "url": "https://auth.internal/token" } } ],
                          "state-setup": [
                            { "name": "state-1", "when": { "state": "an order exists" },
                              "run": { "kind": "http", "url": "http://127.0.0.1:53411/state-setup/state-1",
                                       "headers": { "authorization": "Bearer 9c1f0e8a2b7d4c55" } } } ],
                          "before-request": [
                            { "name": "request-filter-1", "changes": ["parts.request.headers"],
                              "run": { "kind": "http", "url": "http://127.0.0.1:53411/before-request/request-filter-1",
                                       "headers": { "authorization": "Bearer 9c1f0e8a2b7d4c55" } } } ] } } } } }
```

`←`

```json
{ "type": "response", "id": "r-3", "ok": { "session": "vs-1", "stream": "s-1" } }
```

Between here and the terminal event the engine calls the loopback endpoint, which is not a frame:
the state handler's setup receives the state's parameters and answers `{ "outcome": "ok" }`, and
the request filter answers its `changes`. The SDK polls:

```json
{ "type": "request", "id": "r-4", "op": "events/poll", "body": { "streams": ["s-1"], "wait-ms": 10000 } }
```

`←` the run's result, then its summary. A `verified` run passes the test:

```json
{ "type": "response", "id": "r-4",
  "ok": { "events": [
    { "stream": "s-1", "seq": 1, "kind": "verification/started", "payload": { "contracts": 1, "provider": "orders-api" } },
    { "stream": "s-1", "seq": 2, "kind": "verification/interaction-result",
      "payload": { "interaction": "get an order", "variant": "base", "status": "verified" } },
    { "stream": "s-1", "seq": 3, "kind": "verification/finished", "last": true,
      "payload": { "status": "verified", "consumers": ["web-app"], "providers": ["orders-api"],
                   "variants": { "verified": 1, "failed": 0 } } } ] } }
```

A configuration that names a variable the environment does not set is refused before any of this,
with the variable named:

```json
{ "type": "request", "id": "r-5", "op": "config/load",
  "body": { "path": "/work/orders-api/verifier.janus.yaml" } }
```

```json
{ "type": "response", "id": "r-5",
  "error": { "code": "hook-unresolved", "category": "document",
             "message": "environment variable 'AUTH_URL' is not set (used at /hooks/before-verification/0/run/url)",
             "details": { "variable": "AUTH_URL", "at": "/hooks/before-verification/0/run/url" } } }
```
