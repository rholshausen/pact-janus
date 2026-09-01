# Worked example — the message points, in both directions and both roles

Companion to [`../spec.md`](../spec.md). Where the
[provider run](provider-verification-run.md) walks the request/response points, this one walks the two
message points — and it exists to make one claim concrete: `produce-message` and `consume-message` are
named for the **direction** of the message, not for the role of the party, so the same two points serve
a provider verification and a consumer test (spike 1.5, "mocking is the inbound loop, driving is the
outbound call, and the difference is direction, not kind").

The interaction: `orders-ui` consumes an `order-shipped` event that `order-service` publishes.

## 1. Provider side — `produce-message`

The verifier asks the provider's own producer code for the message it would emit, and matches the answer
against the contract. The provider declares the hook the same way it declares any other:

```json hook-config
{
  "version": 1,
  "hooks": {
    "produce-message": [
      { "name": "emit-order-shipped",
        "run": { "kind": "exec",
                 "command": "./gradlew",
                 "args": ["-q", "pactMessage", "--description=an order was shipped"],
                 "env": { "JAVA_HOME": "/usr/lib/jvm/temurin-21" } },
        "when": { "interaction": "an order was shipped" },
        "changes": ["parts"],
        "timeout-ms": 60000 }
    ]
  }
}
```

`changes: ["parts"]` is the whole of the mutable set at this point, and it is the one point where the
mutable member is the parts document itself: there is nothing to modify, because there is nothing there
yet.

```json hook-context
{ "point": "produce-message",
  "role": "provider",
  "run": { "id": "s-9",
           "consumer": { "name": "orders-ui" },
           "provider": { "name": "order-service" } },
  "interaction": { "description": "an order was shipped",
                   "transport": { "kind": "message", "mode": "emissive" },
                   "states": [ { "name": "an order exists", "params": { "id": "42" } } ] },
  "variant": { "id": "base",
               "assignment": [ { "dimension": "body.trackingNumber#presence", "point": "present" } ] },
  "exchange": { "id": "x-1", "data": { } },
  "parts": { },
  "config": { },
  "mutable": ["parts"],
  "deadline-ms": 60000 }
```

The command prints the produced message on stdout as a result document:

```json hook-result
{ "outcome": "ok",
  "changes": { "parts": { "message": {
      "content-type": { "content": "application/json" },
      "metadata": { "content": { "kafka-topic": "orders", "key": "42" } },
      "body": { "content": "eyJvcmRlcklkIjogIjQyIiwgInRyYWNraW5nTnVtYmVyIjogIkFCQzEyMyJ9",
                "encoded": "base64",
                "content-type": "application/json" } } } } }
```

Part and slot names — `message`, `metadata`, `body` — belong to the message transport and the content
component, never to the kernel (design 2.6 §4). The body arrives as octets, and decoding it is the
content component's job on the way into matching, exactly as it is for an HTTP response.

## 2. The claim this design has to keep

The same interaction can be verified without any hook at all, by binding it to a real message transport
that consumes from the topic the provider publishes to:

```json sketch
{ "target": { "transports": [ { "kind": "message",
                                "broker": "kafka://broker.staging.example:9092",
                                "topic": "orders" } ] } }
```

**Both paths MUST produce identical results for the same parts** (§3.4, protocol §8.3, spike 1.5
finding 6). The engine matches the parts it was handed; where they came from is not a matching input,
and nothing downstream of this point can tell the difference.

That is a claim a corpus can falsify, and plan task 4.3 owns the pair: one corpus case fed by a hook,
one fed by a transport, one expected result. §13 records it as an obligation rather than a promise
precisely because it is the kind of property that quietly stops being true — a transport that
normalises a header, a hook path that skips a content negotiation step — and only a test notices.

## 3. Consumer side — `consume-message`

Now the mirror. In a consumer test of the same interaction, the engine *produces* the message from the
contract and the application under test consumes it. The delivery path is a hook, because the engine has
no way to reach the application's handler:

```json hook-config
{
  "version": 1,
  "hooks": {
    "consume-message": [
      { "name": "deliver-to-handler",
        "run": { "kind": "http", "url": "http://127.0.0.1:8099/_pact/message" } }
    ]
  }
}
```

That URL is the loopback shim [ADR 0014](../../../decisions/0014-hooks-are-resolved-configuration-not-callbacks.md)
describes: the SDK serves an endpoint inside the test process, so a consumer test gets callback
ergonomics without the protocol growing an engine-to-host call. From the engine's side it is an
ordinary `http` hook.

```json hook-context
{ "point": "consume-message",
  "role": "consumer",
  "run": { "id": "s-3",
           "consumer": { "name": "orders-ui" },
           "provider": { "name": "order-service" } },
  "interaction": { "description": "an order was shipped",
                   "transport": { "kind": "message", "mode": "emissive" } },
  "variant": { "id": "body.trackingNumber#presence=absent",
               "assignment": [ { "dimension": "body.trackingNumber#presence", "point": "absent" } ] },
  "exchange": { "id": "x-4", "data": { } },
  "parts": { "message": {
      "content-type": { "content": "application/json" },
      "metadata": { "content": { "kafka-topic": "orders", "key": "42" } },
      "body": { "content": "eyJvcmRlcklkIjogIjQyIn0=",
                "encoded": "base64",
                "content-type": "application/json" } } },
  "config": { },
  "mutable": [],
  "deadline-ms": 5000 }
```

Nothing is mutable: the hook's answer *is* its outcome. The handler accepted the message, so:

```json hook-result
{ "outcome": "ok" }
```

and if it had not:

```json hook-result
{ "outcome": "failed",
  "error": { "code": "handler-rejected",
             "message": "OrderShippedHandler: trackingNumber was null" } }
```

which fails that exchange — the variant with `trackingNumber` absent is one the consumer said it could
handle, and this run says it cannot. That is the contract test doing its job on the consumer side, and
it is the same document, the same point and the same result vocabulary the provider used.

## 4. Provider side — `consume-message`

The fourth combination needs no new machinery, which is the point of naming by direction. When the
*provider* is the consumer of a message, verification hands the recorded message to its handler with
exactly the context of §3 and `role: "provider"`. Same point, same context, same result; the only thing
that changed is who is being tested.

Two points cover four cases. A vocabulary that named them by role — `provider-produces`,
`consumer-receives` — would need four, and the two that are actually the same operation would drift
apart the first time one of them grew a member.
