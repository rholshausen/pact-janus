---
engine: minor
---

# `janus derive` writes a message producer's shape from an AsyncAPI document

Staged plan S4.8, ADR 0091. `janus derive` reads AsyncAPI 2.x and 3.0 documents, as JSON or YAML,
beside OpenAPI ones, and writes a `derived` provider shape that `janus check` decides message
contracts against:

- **Only what the application produces.** One entry per message it sends (3.0 `action: send`, 2.x
  `subscribe`), and per reply it answers a request with (3.0 `action: receive` with a `reply`). Each
  entry's `message` part has the message's headers as `metadata`, its content type as a `contentType`
  member there, and its JSON payload as `body`. Message traits are merged as each version says.
- **Selected by the values a message fixes.** A header or payload member whose schema allows one value
  (`const`, or an `enum` of one) is a selector member, and a reply is selected by its request's. A
  channel's address selects only where a header fixes it, so a consumer is matched by what its message
  carries, as ADR 0090 decided.
- **New loss codes**, reported as OpenAPI's are: `selector-empty` (a message fixes nothing, so only its
  description can match it), `selector-ambiguous` (two entries one message could select, which
  `janus check` chooses neither of), `content-type-unstated`, `schema-format-unread` (an Avro or
  protobuf payload, until a component reads it) and `reply-unresolved`.

An AsyncAPI document refuses `--base-path`, and AsyncAPI 1.x is refused by name.
