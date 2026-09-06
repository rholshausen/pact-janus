# Worked example — the built-in HTTP transport, through the interface

Companion to [`../spec.md`](../spec.md). This example exercises the claim that costs the most to keep
true: **a built-in component goes through the same interface a third-party one does**
([ADR 0012](../../../decisions/0012-one-interface-two-bindings.md)). The HTTP transport is the hardest
case for it — it is the most privileged thing in today's Pact, it needs real sockets, and it is the one
component the kernel would most naturally reach into.

Every frame below is validated against the schemas by `cargo test -p pact_janus_schema_compat`.

## 1. The handshake

The engine asks, exactly as it would ask a plugin. The native binding (§9.1) passes these documents as
values rather than bytes; the documents are the same either way, which is the entire content of "one
interface, two bindings".

```json component-hello
{
  "component-protocol-versions": [1],
  "engine": { "name": "janus-engine", "version": "0.1.0" },
  "grants": { "network": true },
  "capabilities": { }
}
```

```json component-hello-result
{
  "component-protocol-version": 1,
  "component": { "name": "http", "version": "1.0.0" },
  "interfaces": ["transport", "matcher"],
  "contributes": {
    "transports": [ { "kind": "http", "roles": ["serve", "drive"] },
                    { "kind": "https", "roles": ["serve", "drive"] } ],
    "operators": [ { "name": "http:status-class", "comparability": "exact", "variant-facet": "value" } ],
    "actions":   [ { "name": "http:status-class" } ]
  },
  "capabilities": { "batch-apply": { } }
}
```

Two things to read here.

The component implements **two** interfaces, because knowing what a status class is and knowing how to
speak HTTP are the same knowledge (§2.1). `http:status-class` is exactly the operator shape spec §3.5
uses to make its point — the kernel does not know what a status code is, so `status-code` is not a core
operator, and the transport that does know contributes it.

And `grants.network` is `true` here where it would be `false` for a content component. A transport is
the interface that needs ambient capability, which is the same fact that makes third-party transports
awkward (§13).

Note what is *not* contributed: `header:parse`, which plan grammar §4.6 lists beside `json:parse`. It
cannot live here, because the namespace is the component's name (§2.4) and this component is called
`http` — so header-value syntax is its own small content component, named `header`. The rule forced a
decision that would otherwise have been made by accident, which is the argument for enforcing it at
load rather than trusting authors to respect it.

## 2. Serving: the consumer mock

`start` in the `serve` role. The engine names the instance (§3.4).

```json start
{ "instance": "t-1", "kind": "http", "role": "serve",
  "options": { "host": "127.0.0.1", "port": 0 } }
```

```json start-result
{ "endpoint": { "kind": "http", "host": "127.0.0.1", "port": 51993,
                "base-url": "http://127.0.0.1:51993" } }
```

The descriptor is an **open document**, and `base-url` is one of its members rather than the whole of
it (spike 1.5 finding 1). The engine hands it to the host unchanged as the `endpoint` of
`consumer-session/start-transport`; a host that wants a URL reads the member, and a message transport's
descriptor carries broker and topic details in the same position without the protocol changing.

The engine then polls. Nothing calls into the engine — an exported poll is trivial for a WASM
component, an engine-callback import would re-enter instances (spike 1.5 finding 4).

```json poll-inbound
{ "instance": "t-1", "timeout-ms": 2000 }
```

```json poll-inbound-result
{ "inbound": {
    "event": "e-7",
    "expects-reply": true,
    "parts": {
      "request": {
        "method":  { "content": "POST" },
        "path":    { "content": "/orders" },
        "headers": { "content": { "content-type": ["application/json"] } },
        "body":    { "content": "eyJpZCI6ICJvLTEifQ==", "encoded": "base64",
                     "content-type": "application/json" } } } } }
```

The body is **base64 octets with a declared content type**, not a parsed object. The transport carries
what arrived; the JSON content component decodes it (§4). A transport that parsed it would eventually
disagree with the component that also parses it, and the interaction would fail on a difference neither
of them made.

Matching happens in the kernel, between these two frames. Then the reply, and the disposition:

```json reply
{ "instance": "t-1", "event": "e-7",
  "parts": { "response": {
    "status":  { "content": 201 },
    "headers": { "content": { "content-type": ["application/json"] } },
    "body":    { "content": "eyJpZCI6ICJvLTEiLCAic3RhdHVzIjogIm5ldyJ9", "encoded": "base64",
                 "content-type": "application/json" } } } }
```

```json dispose
{ "instance": "t-1", "event": "e-7", "disposition": "accept" }
```

`dispose` looks redundant for HTTP — the reply completed the exchange — and it is *not* redundant for a
broker, where accept, reject and release are three different outcomes and dropping the message is none
of them (spike 1.5 finding 7b). The engine disposes of every arrival it polls; a transport whose kind
has no such concept ignores it. Uniformity here is what stops the message work in Phase 8 needing a
sixth primitive.

## 3. Driving: provider verification

The same component, the same operations, the other role.

```json start
{ "instance": "t-2", "kind": "https", "role": "drive",
  "options": { "base-url": "https://provider.internal:8443" } }
```

```json send
{ "instance": "t-2", "await-reply": true, "timeout-ms": 10000,
  "parts": { "request": {
    "method":  { "content": "POST" },
    "path":    { "content": "/orders" },
    "body":    { "content": "eyJpZCI6ICJvLTEifQ==", "encoded": "base64",
                 "content-type": "application/json" } } } }
```

```json send-result
{ "reply": { "response": {
    "status":  { "content": 500 },
    "headers": { "content": { "content-type": ["text/plain"] } },
    "body":    { "content": "aW50ZXJuYWwgZXJyb3I=", "encoded": "base64",
                 "content-type": "text/plain" } } } }
```

A 500 is **not** a transport error. The transport did its job: it sent parts and got parts back. Whether
those parts satisfy the contract is the kernel's question, answered as a mismatch. A transport error is
a connection refused, a TLS failure, a timeout — the machinery failing, which is what §11's `component`
category means.

```json component-error
{ "code": "transport-failed", "category": "component",
  "message": "connection refused: provider.internal:8443",
  "details": { "instance": "t-2", "kind": "https" } }
```

Mocking is the inbound loop and driving is the outbound call; §2 and §3 are the same interface used in
two directions, which is spike 1.5's core finding restated as frames.

## 4. The same case through both bindings

This is the conformance corpus (§9.4): one case, run natively and through the byte-pipe, required to
produce identical results.

```json conformance-case
{
  "name": "http/drive-reply-roundtrip",
  "component": "http",
  "interface": "transport",
  "calls": [
    { "op": "component/hello",
      "body": { "component-protocol-versions": [1] },
      "expect": { "ok": { "component-protocol-version": 1,
                          "component": { "name": "http", "version": "1.0.0" } } } },
    { "op": "transport/start",
      "body": { "instance": "t-1", "kind": "http", "role": "drive",
                "options": { "base-url": "http://127.0.0.1:0" } } },
    { "op": "transport/send",
      "body": { "instance": "t-1", "await-reply": true,
                "parts": { "request": { "method": { "content": "GET" },
                                        "path": { "content": "/health" } } } },
      "expect": { "ok": { "reply": { "response": { "status": { "content": 200 } } } } } },
    { "op": "transport/stop", "body": { "instance": "t-1" } }
  ]
}
```

`bindings` is absent, which means *every binding this component supports* — and that default is the
point. A case that had to name one binding would be a case that had escaped the rule it exists to
enforce.

What the run compares is the result documents after canonicalisation, not timings: the native binding
is expected to be faster, and §13 is where that difference gets measured rather than asserted.

## 5. What this example deliberately does not show

- **Matching.** Nothing between the request frame and the reply frame is the transport's business.
- **Passive versus emissive.** `serve-variant` means "arm and wait" for this transport and "publish now"
  for a message one, and neither appears here, because that distinction lives in the protocol layer
  (protocol §7.4) and not in these primitives — spike 1.5 finding 3, honoured by omission.
- **Decoding.** Every body above is octets with a content type. The next example is where they become
  documents.
