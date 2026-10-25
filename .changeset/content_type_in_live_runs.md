---
default: patch
---

# `content-type` shapes match in live runs

A `content-type` shape, or a v1–v4 `contentType` matcher, failed every live verification and mock exchange
with "no content component is loaded": only the golden corpus runner gave the interpreter a detector. Both
paths now ask the session's content registry (ADR 0049).

- On a contract, a slot's type is the one the interaction declares for it, else the `Content-Type` it
  arrived with, and only a slot with neither is detected from its octets. A reply that labels JSON
  `text/plain` fails a `content-type` of `application/json`.
- A v1–v4 `contentType` matcher detects the type from the reply's octets, whatever they are labelled, as
  today's verifier does.
- Types are compared by essence: case and parameters aside, and `text/*` matches any text type.
- The built-in components detect JSON only. A `contentType` for a binary type such as `image/png` is not
  detected yet.
