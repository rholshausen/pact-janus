---
engine: major
---

# The conformance suite holds an SDK to recording a provider shape

The behavioural specification has four new primitives (staged plan S4.1, ADR 0086): `path-template`,
the HTTP component's `http:path`, and `shape-recorder`, `observe` and `finish-recording`, which record
a provider shape from the provider's own tests by the operation each test names. Seven cases cover
them: `translation/path-template`, `provider/recorder-no-call-until-observe`,
`provider/recorder-call-sequence`, `provider/recorder-description`,
`provider/recorder-description-refused`, `provider/recorder-engine-error` and `live/recorded-shape`,
which pins the shape the engine writes from the sample provider's answers. `janus-sdk-gates
conformance check` expects them in every report, so an SDK must implement recording to move to this
release.

A case may have a `recorder`, a shape recorder as data, in a `provider` or `live` case, and expect
`observed`, `written` and `shape`. The frame schema's operations name the three
`provider-shape-session/*` operations.
