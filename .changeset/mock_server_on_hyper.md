---
engine: minor
---

# A consumer test's mock serves HTTP/2, on hyper

The HTTP transport's mock server is now hyper, on a thread and runtime of its own per mock
(ADR 0062), where it was `tiny_http`. It serves HTTP/1.1 and HTTP/2 with prior knowledge, as today's
Pact mock server does. A client that offers an upgrade is still answered over HTTP/1.1. A recorded
`Content-Length` or `Transfer-Encoding` header is no longer served: the body served decides them.
Nothing an SDK sends to `janus-engine` changes.
