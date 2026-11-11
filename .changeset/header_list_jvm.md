---
jvm-sdk: minor
---

# `Shapes.headerList` declares a header that is a list

`Shapes.headerList(Object... elements)` is a header's value whose elements match however the client or
provider frames them, one field line or several, with any spacing around the commas (staged plan
S3.3, ADR 0082):

```java
janus.interaction("list orders")
    .request(r -> r.method("GET").path("/orders").header("Accept", headerList("application/json", "text/plain")))
    .response(r -> r.status(200).header("Vary", headerList("Accept", "Origin")));
```

A mock serves it as one field line, `Accept, Origin`. It is accepted only as a request or response
header's value; a query value or the literal rules refuse it with an `IllegalArgumentException`. A
`String` or `List` is still compared as the header's field lines, exactly.
