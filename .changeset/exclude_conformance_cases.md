---
engine: major
---

# The conformance suite holds an SDK to `exclude`

The behavioural specification has an `exclude` primitive (staged plan S3.2, ADR 0080). On an interaction
or a message, it appends `{ when, reason }` to the document's `policy.exclude`. It writes each pair and
the reason as given, and checks nothing: the engine resolves the references when the interaction is
added. Four cases cover it: `translation/exclude-policy`, `translation/exclude-on-a-message`,
`live/exclude-removes-combination` and `live/exclude-engine-validates`. `janus-sdk-gates conformance
check` expects them in every report, so an SDK must implement `exclude` to move to this release.

A case's chain takes `exclude`, replayed after `given`, and a contract's content is compared on
`policy` as well as `states`, `parts` and `selection`. Signature roles gain `point-list`, the role of
`exclude`'s pairs.
