---
default: minor
---

# Text and binary bodies verify, through a built-in text component and bytes

A v1–v4 pact whose body is plain text or binary now verifies as today's verifier compares it
(ADR 0036). Both used to fail before anything was sent: text as `component-unavailable`, and a binary
body as `contract-invalid`.

- **A built-in `text` content component** decodes any `text/*` body as UTF-8 into a string, and
  encodes a string back. Every embedding registers it after JSON, `engine/hello` lists `content/text`,
  and a component a project declares for a `text/` type still comes first. It serves only a body whose
  shape admits a string: a `text/csv` body of rows with no CSV component declared is still refused as
  `component-unavailable` when the interaction arrives, now with `reason: compile-failed`.
- **A body no content component claims is now its octets**, the document model's bytes, wherever
  the engine decodes one. It used to be matched as absent. A Janus contract whose shape expected no
  body where the provider sends one of an unclaimed type now fails, as it should. A body that a
  component claims and cannot decode is still absent.
- **A v1–v4 body is read by the pact's content type**, as today's verifier picks a matcher: JSON as
  before, a binary type (`image/*`, `application/octet-stream`, and any type that is not text) as
  bytes, and other text as a string. Each is compared by the rules on `$`, or by equality. XML, form
  and multipart bodies are still `component-unavailable`.
- **A reply whose content type disagrees with the pact's fails**, with today's message (`Expected a
  body of 'text/plain' but the actual content type was 'text/html'`), when the pact names no
  `Content-Type` header. Plans show this as a new legacy action, `legacy:body-type`, so `explain`
  prints it for a JSON pact with no `Content-Type` header too. The reply's body is decoded as the
  pact's type whatever it is labelled.
- **A bytes mismatch names the first byte that differs.**
- **A corpus case may carry bytes**: `encoded` maps a path in `values` to `base64` (plan-grammar
  spec §6.1).
