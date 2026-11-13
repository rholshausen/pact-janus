---
jvm-sdk: minor
---

# `ShapeRecorder.observeMessage` records the messages a provider produces

A provider's tests record the messages it publishes into the same shape as its HTTP answers, by what
identifies each message's operation (staged plan S4.6, ADR 0090):

```java
recorder.observeMessage(Map.of("metadata", Map.of("queue", "orders.placed")), orderPlaced(),
    ShapeRecorder.MessageOptions.described("publish orders.placed"));
```

The message is the `Message` a `@ProvidesMessage` method returns. `operation` names its `metadata` and
`body`, or with `MessageOptions.withRequest(message)`, a synchronous message's request. `janus check`
then matches a consumer's message contract to the entry by what its message carries, here the queue.
