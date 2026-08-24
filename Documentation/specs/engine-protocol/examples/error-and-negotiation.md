# Worked example — errors as values, version negotiation, unknowns named

Failure paths, one frame per ```` ```json ```` block, validated against the v1 schemas by
`cargo test -p pact_janus_schema_compat`. `→` is host to engine, `←` is engine to host.
The common property (spec §11.3): **every unknown arrives named** — degradation is a policy
decision made with the unknown's identity in hand.

## Version rejection

`→` a host from the future:

```json
{ "type": "request", "id": "r-1", "op": "engine/hello",
  "body": { "protocol-versions": [99], "host": { "name": "pact-js", "version": "9.0.0" } } }
```

`←` the supported versions are *named*; the host can report precisely, then shut down:

```json
{ "type": "response", "id": "r-1",
  "error": { "code": "protocol-version-unsupported", "category": "protocol",
             "message": "this engine speaks protocol versions [1]",
             "details": { "supported": [1] } } }
```

## Unknown operation

`→` an operation this engine has never heard of (perhaps added in a later engine release):

```json
{ "type": "request", "id": "r-2", "op": "verification/replay",
  "body": { "session": "vs-1" } }
```

`←` named, never silent (spike 1.1, gauntlet E3):

```json
{ "type": "response", "id": "r-2",
  "error": { "code": "operation-unsupported", "category": "protocol",
             "message": "unknown operation 'verification/replay'",
             "details": { "op": "verification/replay" } } }
```

## Malformed frame

A frame whose body was truncated by a host bug cannot be correlated; the empty `id` marks
the error as pipe-level. On the stdio pipe the declared `Content-Length`, not the JSON,
delimits the stream, so the session continues in sync (spike 1.3, finding 6):

`←`

```json
{ "type": "response", "id": "",
  "error": { "code": "malformed-frame", "category": "protocol",
             "message": "frame body is not valid JSON (unexpected end of input at byte 112)" } }
```

## Invalid user document

`→` an interaction spec with a typo'd operator — a *user* mistake, not a protocol one:

```json
{ "type": "request", "id": "r-3", "op": "consumer-session/add-interaction",
  "body": { "session": "cs-1",
            "interaction": { "description": "a request for an order",
                             "response": { "body": { "shape": { "id": { "tpye": "integer" } } } } } } }
```

`←` category `document`, with positions a DSL can surface to the test author:

```json
{ "type": "response", "id": "r-3",
  "error": { "code": "interaction-invalid", "category": "document",
             "message": "interaction specification is not valid",
             "details": { "problems": [ { "path": "$.response.body.shape.id",
                                          "message": "unknown shape operator 'tpye' (did you mean 'type'?)" } ] } } }
```

## Stale identifier

`→` polling a stream whose terminal event was already delivered (the session is gone —
sessions are the only resource, and it ended itself):

```json
{ "type": "request", "id": "r-4", "op": "events/poll",
  "body": { "streams": ["s-1"] } }
```

`←`

```json
{ "type": "response", "id": "r-4",
  "error": { "code": "stream-not-found", "category": "session",
             "message": "stream 's-1' does not exist or has ended",
             "details": { "stream": "s-1" } } }
```
