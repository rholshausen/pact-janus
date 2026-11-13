---
typescript-sdk: minor
---

# `ShapeRecorder.observeMessage` records the messages a provider produces

A provider's tests record the messages it publishes into the same shape as its HTTP answers, by what
identifies each message's operation (staged plan S4.6, ADR 0090):

```ts
const message = createOrderPlacedEvent("order-1"); // a `Message`, as a message producer returns one
await recorder.observeMessage({ metadata: { queue: "orders.placed" } }, message, {
  description: "publish orders.placed",
});
```

`operation` names the message's own `metadata` and `body`, or with `request`, the message a synchronous
message answers, the request's. `description` is required. `janus check` then matches a consumer's
message contract to the entry by what its message carries, here the queue.
