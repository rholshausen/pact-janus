---
engine: minor
---

# A provider shape records the messages a provider produces, and `janus check` decides message contracts by it

Staged plan S4.6, ADR 0090. A message has no request, so its operation is named by the message itself:

- **`provider-shape-session/observe` takes `kind`**, the session's when absent, so one session records
  a provider's HTTP answers and its messages into one shape. The kind `message`, which no transport
  serves, is read as `produce-message` hooks produce it: the `message` part is recorded, answering a
  synchronous message's `request`, and a body with no `content-type` is read by its metadata's
  `contentType`, else by its octets, as a verification reads a produced message. Any other kind no
  transport serves is still `component-unavailable`.
- **An observation's `operation` may name any part it carries**: an asynchronous message's own
  `metadata` or `body`, such as the queue it was published to, or a synchronous message's `request`.
  Every part it names is checked against it, and one it does not admit is `interaction-invalid` at
  `/operation`.
- **`subsumption/check` decodes a content slot a selector names** by its declared type, so a selector
  over a member of a message's body selects by the member.

`janus check` on pact-js's message example, against the shape its provider records, is matched by the
queue the consumer names and finds the empty list it never tested. The `subsumption-findings` reason
now says the provider "may produce what this consumer has not tested", where it said "responses".
