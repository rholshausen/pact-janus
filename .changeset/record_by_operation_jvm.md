---
jvm-sdk: minor
---

# `ShapeRecorder` records a provider shape from the provider's own tests

A provider's tests record the shape of what the provider answers, by the operation each test names,
and the engine writes it for `janus check` (staged plan S4.1, ADR 0086):

```java
static final ShapeRecorder recorder = ShapeRecorder.forProvider("order-service");

@Test
void anOrder() throws Exception {
  HttpResponse<byte[]> response = client.send(request("/orders/66"), BodyHandlers.ofByteArray());
  recorder.observe(Map.of("method", "GET", "path", Shapes.pathTemplate("/orders/{id}")),
      ShapeRecorder.Response.of(response));
  // ...the test's own assertions
}

@AfterAll
static void finish() {
  recorder.finish();
}
```

The recording's source is the class that called `forProvider`. `finish()` has the engine write
`shapes/order-service.provider-shape.json`, merged with every other class's, and keep the counts in
`target/janus/evidence` beside a `pom.xml`, else `build/janus/evidence`. `Response.of` sends the
request the response answered, which the engine checks the operation admits; `new Response(status,
headers, body)` is the other form. `Shapes.pathTemplate(template)` and `pathTemplate(template,
example)` are the HTTP component's path template, which a consumer's request may use too, with an
example.
