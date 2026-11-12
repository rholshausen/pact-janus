---
typescript-sdk: minor
---

# `ShapeRecorder` records a provider shape from the provider's own tests

A provider's tests record the shape of what the provider answers, by the operation each test names,
and the engine writes it for `janus check` (staged plan S4.1, ADR 0086):

```ts
import { pathTemplate } from "pact-janus";
import { useShapeRecorder } from "pact-janus/vitest"; // or pact-janus/jest

const recorder = useShapeRecorder({ provider: "order-service" });

test("an order", async () => {
  const response = await fetch(`${url}/orders/66`);
  await recorder.observe({ method: "GET", path: pathTemplate("/orders/{id}") }, response);
  // ...the test's own assertions
});
```

`useShapeRecorder` names the test file as the recording's source and finishes it after the file's
tests: the engine writes `shapes/order-service.provider-shape.json`, merged with every other test
file's, and keeps the counts in `.janus/evidence`. `new ShapeRecorder(config)` and `finish()` are the
same without a test framework. A `fetch` `Response` is read from a clone, so hand it over before
reading its body; `{ status, headers, body }` is the other form. `pathTemplate(template, example?)` is
the HTTP component's path template, which a consumer's request may use too, with an example.
