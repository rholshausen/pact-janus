---
engine: minor
---

# `JANUS_LOG_FORMAT=json` writes each log event as a JSON line

With `JANUS_LOG_FORMAT=json`, `janus-engine` and `janus` write each event on standard error as one
JSON object: `timestamp`, `level` (`TRACE` to `ERROR`), `target`, and `fields`, whose `message` is the
event's message. The same events are held back as in text, credentials included. An SDK reads these
members to hand the engine's events to its own runtime's logger (ADR 0061).
