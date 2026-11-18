# Worked example: provider shapes and subsumption reports in a Pact Broker

A provider publishes the shape its own tests recorded. Then a consumer's pipeline checks its
contract against that shape, fetched from the broker by branch, and publishes the report beside it.
Last, the same check is sent to a broker without the `janus` feature.
[ADR 0093](../../../decisions/0093-publish-and-fetch-provider-shapes-and-reports-through-the-engines-broker-client-found-by-the-links-a-broker-advertises.md)
decided the shape: the broker client fetches and publishes inside a broker session, finding each
resource by the link the broker's index advertises, and the host decides over the reports with
`subsumption/decide`. Every ```` ```json ```` block is one frame, validated against the v1 schemas by
`cargo test -p pact_janus_schema_compat`. `→` is host to engine, `←` is engine to host. The
handshake is [broker-verification.md](broker-verification.md)'s, and the engine declares `broker`
and `background`.

## 1. A provider publishes its shape

`→` the shape, for the provider version the build produced, on its branch. The document is the
file's text, as a host may send it (ADR 0063). The engine reads it before it publishes it, and takes
the provider from it:

```json
{ "type": "request", "id": "r-2", "op": "broker/publish-provider-shape",
  "body": { "broker": { "url": "https://broker.example.com",
                        "auth": { "kind": "bearer", "token": "pb-token-from-the-host" } },
            "provider-version": "4.1.0+9e2d7c1", "branch": "main",
            "build-url": "https://ci.example.com/builds/913",
            "provider-shape": "{\"$format\":\"janus-provider-shape/1\",\"provider\":{\"name\":\"order-service\"},\"provenance\":\"recorded\",\"interactions\":[]}" } }
```

`←`

```json
{ "type": "response", "id": "r-2", "ok": { "session": "bs-1", "stream": "s-1" } }
```

`→`

```json
{ "type": "request", "id": "r-3", "op": "events/poll", "body": { "streams": ["s-1"], "wait-ms": 1000 } }
```

`←` one event, the terminal one. The broker's notices come back as text, and the broker is named by
its URL alone:

```json
{ "type": "response", "id": "r-3",
  "ok": { "events": [
    { "stream": "s-1", "seq": 1, "kind": "broker/finished", "last": true,
      "payload": { "status": "published", "broker": "https://broker.example.com",
                   "provider": "order-service", "provider-version": "4.1.0+9e2d7c1",
                   "notices": ["Provider shape published for order-service version 4.1.0+9e2d7c1."] } } ] } }
```

## 2. A consumer checks against it, and publishes the report

`→` the consumer's contract (here the v1–v4 pact it published), against the latest shape
`order-service` published on `main`. With `publish-reports`, each report goes to the link the shape's
own document gave for it, with the contract it is about:

```json
{ "type": "request", "id": "r-4", "op": "broker/check-contracts",
  "body": { "broker": { "url": "https://broker.example.com",
                        "auth": { "kind": "bearer", "token": "pb-token-from-the-host" } },
            "provider-branch": "main",
            "publish-reports": true,
            "contracts": [ { "consumer": { "name": "web-app" }, "provider": { "name": "order-service" },
                             "interactions": [ { "description": "a request for an order",
                                                 "request": { "method": "GET", "path": "/orders/1" },
                                                 "response": { "status": 200, "body": { "status": "SHIPPED" } } } ],
                             "metadata": { "pactSpecification": { "version": "3.0.0" } } } ] } }
```

`←`

```json
{ "type": "response", "id": "r-4", "ok": { "session": "bs-2", "stream": "s-2" } }
```

`→`

```json
{ "type": "request", "id": "r-5", "op": "events/poll", "body": { "streams": ["s-2"], "wait-ms": 1000 } }
```

`←` one `broker/checked` per contract, then the terminal event carrying them all. The report is
`subsumption/check`'s, the same report the two documents give when they are files. It is abridged
here to one finding. The publication's outcome never changes the check's:

```json
{ "type": "response", "id": "r-5",
  "ok": { "events": [
    { "stream": "s-2", "seq": 1, "kind": "broker/checked",
      "payload": { "consumer": "web-app", "provider": "order-service", "outcome": "checked",
                   "provider-version": "4.1.0+9e2d7c1", "format": "pact/3.0.0",
                   "report": { "$format": "janus-subsumption-report/1",
                               "consumer": { "name": "web-app" }, "provider": { "name": "order-service" },
                               "interactions": [ { "description": "a request for an order", "matched": true,
                                                   "matched-by": "selector", "provenance": "recorded", "verdict": "no",
                                                   "findings": [ { "path": "response.body.status", "verdict": "no",
                                                                   "severity": "finding", "kind": "wider-values",
                                                                   "provider": { "summary": "one of 'CANCELLED' | 'PENDING' | 'SHIPPED'" },
                                                                   "consumer": { "summary": "exactly 'SHIPPED'" },
                                                                   "reason": "an option the consumer's shape does not admit" } ] } ],
                               "summary": { "interactions": 1, "matched": 1, "findings": 1, "reviews": 0 } },
                   "publication": { "outcome": "published" } } },
    { "stream": "s-2", "seq": 2, "kind": "broker/finished", "last": true,
      "payload": { "status": "checked", "broker": "https://broker.example.com",
                   "results": [ { "consumer": "web-app", "provider": "order-service", "outcome": "checked",
                                  "provider-version": "4.1.0+9e2d7c1", "format": "pact/3.0.0",
                                  "report": { "$format": "janus-subsumption-report/1",
                                              "consumer": { "name": "web-app" }, "provider": { "name": "order-service" },
                                              "interactions": [],
                                              "summary": { "interactions": 1, "matched": 1, "findings": 1, "reviews": 0 } },
                                  "publication": { "outcome": "published" } } ] } } ] } }
```

(The terminal event's copy of the report is abridged further. It carries the same report.)

The host now asks `subsumption/decide` with that report as the pair's `subsumption`, beside the
verification summaries and policy layers it holds (§8.6). A contract whose provider published no
shape on `main` would have come back as `{ "outcome": "not-published" }`, and goes to `decide` as a
pair with no report, which `decide` answers as `provider-shape-missing`, replay-only.

## 3. A broker without the `janus` feature

`→` the same check, against a broker whose index advertises no `pb:latest-branch-provider-shape`:

```json
{ "type": "request", "id": "r-6", "op": "broker/check-contracts",
  "body": { "broker": { "url": "https://old-broker.example.com" },
            "provider-branch": "main",
            "contracts": [ { "$format": "janus-contract/1", "consumer": { "name": "web-app" },
                             "provider": { "name": "order-service" }, "interactions": [] } ] } }
```

`←`

```json
{ "type": "response", "id": "r-6", "ok": { "session": "bs-3", "stream": "s-3" } }
```

`→`

```json
{ "type": "request", "id": "r-7", "op": "events/poll", "body": { "streams": ["s-3"], "wait-ms": 1000 } }
```

`←` every contract fails, naming why. None is `not-published`: that would read as a provider that
published nothing, which `decide` lets through as replay-only:

```json
{ "type": "response", "id": "r-7",
  "ok": { "events": [
    { "stream": "s-3", "seq": 1, "kind": "broker/checked",
      "payload": { "consumer": "web-app", "provider": "order-service", "outcome": "failed",
                   "error": { "code": "component-failed", "category": "component",
                              "message": "the 'broker' component failed: the broker's index has no 'pb:latest-branch-provider-shape' link: it does not store provider shapes (they need the Pact Broker's janus feature)",
                              "details": { "component": "broker",
                                           "error": { "code": "broker-response-invalid", "category": "component",
                                                      "message": "the broker's index has no 'pb:latest-branch-provider-shape' link: it does not store provider shapes (they need the Pact Broker's janus feature)",
                                                      "details": { "broker": "https://old-broker.example.com" } } } } } },
    { "stream": "s-3", "seq": 2, "kind": "broker/finished", "last": true,
      "payload": { "status": "failed", "broker": "https://old-broker.example.com",
                   "results": [ { "consumer": "web-app", "provider": "order-service", "outcome": "failed",
                                  "error": { "code": "component-failed", "category": "component",
                                             "message": "the 'broker' component failed: the broker's index has no 'pb:latest-branch-provider-shape' link: it does not store provider shapes (they need the Pact Broker's janus feature)",
                                             "details": { "component": "broker" } } } ] } } ] } }
```
