# Worked example — provider verification with the event stream

Verifying one pact (two interactions, one failing) against a local provider. Every
```` ```json ```` block is one frame, validated against the v1 schemas by
`cargo test -p pact_janus_schema_compat`. `→` is host to engine, `←` is engine to host.
This transcript uses the baseline **poll** delivery from ADR 0005 — it is identical on all
three pipes; with the negotiated `push-events` capability the same `Event` objects would
arrive as EventFrames instead of PollResults.

`→`

```json
{ "type": "request", "id": "r-1", "op": "engine/hello",
  "body": { "protocol-versions": [1], "host": { "name": "janus-cli", "version": "0.1.0" } } }
```

`←`

```json
{ "type": "response", "id": "r-1",
  "ok": { "protocol-version": 1, "engine": { "name": "janus-engine", "version": "0.1.0" } } }
```

`→` start the run. The source is `inline` — the host fetched the pact itself (file, URL or
broker; I/O stays out of the kernel). The target's transport bindings are open descriptor
documents. The pact interior is sketched; its shape is design 2.5's:

```json
{ "type": "request", "id": "r-2", "op": "verification/verify",
  "body": { "source": { "kind": "inline",
                        "pacts": [ { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" },
                                     "interactions": [ { "description": "a request for an order" },
                                                       { "description": "a request for a missing order" } ] } ] },
            "target": { "transports": [ { "transport": "http",
                                          "options": { "scheme": "http", "host": "127.0.0.1", "port": 8080 } } ] },
            "options": { "emit-executed-plans": false } } }
```

`←` the run has started; everything else arrives on stream `s-1`:

```json
{ "type": "response", "id": "r-2", "ok": { "session": "vs-1", "stream": "s-1" } }
```

`→` drain events, long-polling up to 500 ms:

```json
{ "type": "request", "id": "r-3", "op": "events/poll",
  "body": { "streams": ["s-1"], "wait-ms": 500 } }
```

`←` per-stream `seq` is gapless from 1; event `kind` is an open vocabulary:

```json
{ "type": "response", "id": "r-3",
  "ok": { "events": [
    { "stream": "s-1", "seq": 1, "kind": "verification/started",
      "payload": { "provider": "order-api", "pacts": 1, "interactions": 2 } },
    { "stream": "s-1", "seq": 2, "kind": "verification/interaction-started",
      "payload": { "interaction": "a request for an order", "variant": "base" } },
    { "stream": "s-1", "seq": 3, "kind": "verification/interaction-result",
      "payload": { "interaction": "a request for an order", "variant": "base", "status": "verified" } } ] } }
```

`→` keep draining:

```json
{ "type": "request", "id": "r-4", "op": "events/poll",
  "body": { "streams": ["s-1"], "wait-ms": 500 } }
```

`←` a mismatch is a *result*, not an error — the machinery worked, the provider didn't. The
final event carries `last: true` (structural termination, whatever its kind) and the summary;
delivering it ends stream `s-1` and closes session `vs-1`:

```json
{ "type": "response", "id": "r-4",
  "ok": { "events": [
    { "stream": "s-1", "seq": 4, "kind": "verification/interaction-started",
      "payload": { "interaction": "a request for a missing order", "variant": "base" } },
    { "stream": "s-1", "seq": 5, "kind": "verification/interaction-result",
      "payload": { "interaction": "a request for a missing order", "variant": "base", "status": "failed",
                   "mismatches": [ { "path": "$.status", "expected": "404", "actual": "500" } ] } },
    { "stream": "s-1", "seq": 6, "kind": "verification/finished", "last": true,
      "payload": { "status": "failed", "interactions": { "verified": 1, "failed": 1 } } } ] } }
```

No cleanup call follows — the terminal event already ended the session.
