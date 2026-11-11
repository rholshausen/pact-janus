---
typescript-sdk: minor
---

# `headerList` declares a header that is a list

`headerList(...elements)` is a header's value whose elements match however the client or provider
frames them, one field line or several, with any spacing around the commas (staged plan S3.3,
ADR 0082):

```ts
janus.interaction("list orders")
  .request({ method: "GET", path: "/orders", headers: { Accept: headerList("application/json", "text/plain") } })
  .response({ status: 200, headers: { Vary: headerList("Accept", "Origin") } })
```

A mock serves it as one field line, `Accept, Origin`. It is accepted only as a request or response
header's value; a query value, a body or another helper refuses it. A plain string or list is still
compared as the header's field lines, exactly. `HeaderList` and the `HeaderValue` type are exported.
