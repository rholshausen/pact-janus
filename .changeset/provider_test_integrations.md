---
default: minor
---

# A test can produce a provider's messages, and a produced message is held only to a content type it has

A provider test's message producers are `produce-message` hooks it serves on its loopback endpoint,
as its state handlers are (ADR 0056). Each is named `message-<n>` and selected by its interaction's
description: `{ name, when: { interaction }, changes: ["parts"], run }`. It answers the whole
`message` part, with the body base64-encoded and labelled only when the producer gave a content type.

The engine changed in one place. A v1–v4 message whose produced metadata has no `contentType` member
at all is now read as carrying the message's own content type there: its body's label, else what its
octets are detected as. Before, it failed on the metadata's expected `contentType`. The `pact-message`
format always writes one, so nothing `janus verify` produces changes. A script or component that
produces a message with no `contentType` in its metadata now verifies where it failed.

For an SDK maintainer: the behavioural specification gains `message-producer`, with four conformance
ids, and `verify.transport-options-as-given`. A verifier's `transport` option is the HTTP transport's
options beside `base-url`. The conformance suite has three provider cases and one live case for them.
