# Worked example — verifying from a Pact Broker, and publishing

A provider verifying the pacts a broker selects for it, one of them pending, and publishing each
result back; then the same request to an engine with no broker client; then a consumer publishing
its contract. [ADR 0032](../../../decisions/0032-speak-to-the-broker-through-protocol-operations-backed-by-a-native-host-capability.md)
decided the shape: the broker client is the engine's, and fetching and publishing happen inside the
run. Every ```` ```json ```` block is one frame, validated against the v1 schemas by
`cargo test -p pact_janus_schema_compat`. `→` is host to engine, `←` is engine to host.

## 1. Verify from a broker

`→` the handshake. A native embedding has a broker client, and says so:

```json
{ "type": "request", "id": "r-1", "op": "engine/hello",
  "body": { "protocol-versions": [1], "host": { "name": "janus-jvm", "version": "0.1.0" }, "capabilities": {} } }
```

`←`

```json
{ "type": "response", "id": "r-1",
  "ok": { "protocol-version": 1, "engine": { "name": "janus-engine", "version": "0.1.0" },
          "capabilities": { "broker": {}, "components": { "loaders": ["in-tree", "wasm"] } } } }
```

`→` start the run. The source is the broker: which provider, which consumer versions (the
selectors are the broker's vocabulary, passed through as written), and the provider version's
branch, which is what the broker decides pending status from. The token came from the host's own
configuration; the engine never reads it from its environment and never repeats it:

```json
{ "type": "request", "id": "r-2", "op": "verification/verify",
  "body": { "source": { "kind": "broker",
                        "broker": { "url": "https://broker.example.com",
                                    "auth": { "kind": "bearer", "token": "pb-token-from-the-host" } },
                        "provider": "order-api",
                        "consumer-version-selectors": [ { "mainBranch": true }, { "deployedOrReleased": true } ],
                        "provider-branch": "main",
                        "include-pending": true,
                        "include-wip-since": "2026-10-01" },
            "target": { "transports": [ { "transport": "http",
                                          "options": { "base-url": "http://127.0.0.1:8080" } } ] },
            "options": { "publish": { "provider-version": "1.4.2+3f9c2a1", "branch": "main",
                                      "build-url": "https://ci.example.com/builds/812" } } } }
```

`←` the run has started; the fetch happens inside it, so this answers at once:

```json
{ "type": "response", "id": "r-2", "ok": { "session": "vs-1", "stream": "s-1" } }
```

`→` drain events:

```json
{ "type": "request", "id": "r-3", "op": "events/poll",
  "body": { "streams": ["s-1"], "wait-ms": 500 } }
```

`←` what was fetched, before anything is verified. `web-app`'s pact is verified on main and is not
pending; `mobile-app`'s is a work-in-progress pact, and so pending. The broker is named by its URL
alone:

```json
{ "type": "response", "id": "r-3",
  "ok": { "events": [
    { "stream": "s-1", "seq": 1, "kind": "verification/fetched",
      "payload": { "broker": "https://broker.example.com",
                   "pacts": [ { "consumer": "web-app", "pending": false, "wip": false,
                                "notices": [ "The pact at https://broker.example.com/pacts/… is being verified because it matches the selector {\"mainBranch\":true}" ] },
                              { "consumer": "mobile-app", "pending": true, "wip": true,
                                "notices": [ "This pact is in pending state for this version of order-api because a successful verification result for a version with branch main has not yet been published." ] } ] } },
    { "stream": "s-1", "seq": 2, "kind": "verification/started",
      "payload": { "contracts": 2, "interactions": 2, "consumers": ["web-app", "mobile-app"],
                   "providers": ["order-api", "order-api"] } },
    { "stream": "s-1", "seq": 3, "kind": "verification/interaction-result",
      "payload": { "interaction": { "description": "a request for an order", "index": 0, "id": "a1b2c3" },
                   "variant": "base", "status": "verified" } },
    { "stream": "s-1", "seq": 4, "kind": "verification/interaction-result",
      "payload": { "interaction": { "description": "a request for an order's tracking", "index": 0, "id": "d4e5f6" },
                   "variant": "base", "status": "failed",
                   "mismatches": [ { "path": "$.response.status", "message": "Expected 404 to equal 200" } ] } } ] } }
```

`→` keep draining:

```json
{ "type": "request", "id": "r-4", "op": "events/poll",
  "body": { "streams": ["s-1"], "wait-ms": 500 } }
```

`←` each pact's result goes to that pact's own link, keyed by the broker's interaction ids, which the
run carried from the fetched documents (`id` above). `mobile-app`'s interaction failed, and its
pact is pending, so the run passes: the failure is reported and published, and does not fail it.
The summary says which sources were pending, and what was published:

```json
{ "type": "response", "id": "r-4",
  "ok": { "events": [
    { "stream": "s-1", "seq": 5, "kind": "verification/published",
      "payload": { "consumer": "web-app", "provider-version": "1.4.2+3f9c2a1", "outcome": "published" } },
    { "stream": "s-1", "seq": 6, "kind": "verification/published",
      "payload": { "consumer": "mobile-app", "provider-version": "1.4.2+3f9c2a1", "outcome": "published" } },
    { "stream": "s-1", "seq": 7, "kind": "verification/finished", "last": true,
      "payload": { "status": "verified", "contracts": 2, "interactions": 2,
                   "consumers": ["web-app", "mobile-app"], "providers": ["order-api", "order-api"],
                   "pending": [false, true],
                   "variants": { "total": 2, "verified": 1, "failed": 1, "state-unavailable": 0, "skipped": 0 },
                   "publication": [ { "consumer": "web-app", "outcome": "published" },
                                    { "consumer": "mobile-app", "outcome": "published" } ] } } ] } }
```

## 2. The same request, to an engine with no broker client

The offline WASM artifact has no broker client and declares no `broker` capability. A host that
asks anyway gets the component named, before any run starts:

`→`

```json
{ "type": "request", "id": "r-2", "op": "verification/verify",
  "body": { "source": { "kind": "broker", "broker": { "url": "https://broker.example.com" },
                        "provider": "order-api" },
            "target": { "transports": [ { "transport": "http",
                                          "options": { "base-url": "http://127.0.0.1:8080" } } ] } } }
```

`←`

```json
{ "type": "response", "id": "r-2",
  "error": { "code": "component-unavailable", "category": "component",
             "message": "this engine has no Pact Broker client: a broker source needs a native embedding",
             "details": { "component": "broker" } } }
```

## 3. A consumer publishes its contract

`→` a broker session: the contract, the consumer version it belongs to, and its branch. The
contract's interior is sketched; its shape is design 2.5's:

```json
{ "type": "request", "id": "r-5", "op": "broker/publish-contracts",
  "body": { "broker": { "url": "https://broker.example.com",
                        "auth": { "kind": "bearer", "token": "pb-token-from-the-host" } },
            "consumer-version": "2.7.0+8d1e44b",
            "branch": "main",
            "build-url": "https://ci.example.com/builds/377",
            "contracts": [ { "$format": "janus-contract/1",
                             "consumer": { "name": "web-app" }, "provider": { "name": "order-api" },
                             "interactions": [] } ] } }
```

`←` it reaches the network, so it reports on a stream:

```json
{ "type": "response", "id": "r-5", "ok": { "session": "bs-1", "stream": "s-2" } }
```

`→`

```json
{ "type": "request", "id": "r-6", "op": "events/poll",
  "body": { "streams": ["s-2"], "wait-ms": 500 } }
```

`←` one event per contract, and the terminal event, which ends the broker session:

```json
{ "type": "response", "id": "r-6",
  "ok": { "events": [
    { "stream": "s-2", "seq": 1, "kind": "broker/published",
      "payload": { "consumer": "web-app", "provider": "order-api", "outcome": "published" } },
    { "stream": "s-2", "seq": 2, "kind": "broker/finished", "last": true,
      "payload": { "status": "published",
                   "contracts": [ { "consumer": "web-app", "provider": "order-api", "outcome": "published" } ] } } ] } }
```
