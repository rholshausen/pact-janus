# Worked example — consumer session over HTTP, frame by frame

A `janus-ts` test run: one interaction with an `optional` field, giving two variants. Every
```` ```json ```` block below is one complete frame and is validated against the v1 schemas
by `cargo test -p pact_janus_schema_compat` — these transcripts cannot rot silently.
`→` is host to engine, `←` is engine to host. Interaction-spec and contract-file interiors are
illustrative sketches — their shapes belong to designs 2.2 and 2.5, not to this protocol.

`→` handshake first, always:

```json
{ "type": "request", "id": "r-1", "op": "engine/hello",
  "body": { "protocol-versions": [1], "host": { "name": "janus-ts", "version": "0.1.0" }, "capabilities": {} } }
```

`←`

```json
{ "type": "response", "id": "r-1",
  "ok": { "protocol-version": 1, "engine": { "name": "janus-engine", "version": "0.1.0" }, "capabilities": {} } }
```

`→` create the session:

```json
{ "type": "request", "id": "r-2", "op": "consumer-session/create",
  "body": { "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } } }
```

`←`

```json
{ "type": "response", "id": "r-2", "ok": { "session": "cs-1" } }
```

`→` submit the complete interaction specification in one call (the spec interior is 2.2's;
sketched here):

```json
{ "type": "request", "id": "r-3", "op": "consumer-session/add-interaction",
  "body": { "session": "cs-1",
            "interaction": {
              "description": "a request for an order",
              "transport": { "kind": "http", "mode": "passive" },
              "request": { "method": "GET", "path": "/orders/66" },
              "response": { "status": 200,
                            "body": { "shape": { "id": { "type": "integer" },
                                                 "discount": { "optional": { "type": "number" } } } } } } } }
```

`←`

```json
{ "type": "response", "id": "r-3", "ok": { "handle": "i-1" } }
```

`→` ask for the variant space (the `optional` operator contributes present/absent):

```json
{ "type": "request", "id": "r-4", "op": "consumer-session/variants",
  "body": { "session": "cs-1", "handle": "i-1" } }
```

`←` variant descriptors carry `id` for the protocol; the rest is design 2.3's:

```json
{ "type": "response", "id": "r-4",
  "ok": { "variants": [ { "id": "base", "dimensions": { "response.body.discount": "present" } },
                        { "id": "discount-absent", "dimensions": { "response.body.discount": "absent" } } ] } }
```

`→` start the HTTP transport; the result is an open endpoint descriptor, not a URL:

```json
{ "type": "request", "id": "r-5", "op": "consumer-session/start-transport",
  "body": { "session": "cs-1", "transport": "http" } }
```

`←`

```json
{ "type": "response", "id": "r-5",
  "ok": { "endpoint": { "scheme": "http", "host": "127.0.0.1", "port": 8123 } } }
```

`→` arm the first variant (this interaction is **passive**: the engine now waits for the
application under test to call the endpoint):

```json
{ "type": "request", "id": "r-6", "op": "consumer-session/serve-variant",
  "body": { "session": "cs-1", "handle": "i-1", "variant": "base" } }
```

`←`

```json
{ "type": "response", "id": "r-6", "ok": {} }
```

*(the application under test makes its request and asserts on the mock's response — outside
the protocol)*

`→` re-arm with the second variant and exercise it the same way:

```json
{ "type": "request", "id": "r-7", "op": "consumer-session/serve-variant",
  "body": { "session": "cs-1", "handle": "i-1", "variant": "discount-absent" } }
```

`←`

```json
{ "type": "response", "id": "r-7", "ok": {} }
```

`→` finalise: ends the session unconditionally, returns per-interaction/per-variant results,
and — because everything verified — the Janus contract document (persistence is the host's business):

```json
{ "type": "request", "id": "r-8", "op": "consumer-session/finalise",
  "body": { "session": "cs-1" } }
```

`←`

```json
{ "type": "response", "id": "r-8",
  "ok": { "results": [ { "handle": "i-1", "status": "verified",
                         "variants": [ { "variant": "base", "status": "verified" },
                                       { "variant": "discount-absent", "status": "verified" } ] } ],
          "contract": { "$format": "janus-contract/1",
                    "consumer": { "name": "web-app" }, "provider": { "name": "order-api" },
                    "interactions": [ { "description": "a request for an order" } ] } } }
```

After this frame `cs-1`, `i-1` and the endpoint no longer exist — nothing was ever
individually released.
