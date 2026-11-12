---
engine: minor
---

# `janus derive` writes a provider shape from an OpenAPI document

Staged plan S4.2, ADR 0088. `janus derive <openapi.yaml|json>` reads an OpenAPI 3.0 or 3.1 document and
writes a provider shape with provenance `derived`, which `janus check` reads like a recorded one:

- **One entry per operation and documented response status**, selected by the method, the path as an
  `http:path` template (with the `servers` path in front, or `--base-path`) and the status, so it
  matches every consumer's interaction by operation, error responses included. A range such as `4XX`
  is the statuses the operation does not document explicitly; `default` is not imported.
- **Each response's documented headers, its `Content-Type`, and its JSON body**, mapped as spike 7.3
  measured. An array that states no `minItems` admits the empty array, as JSON Schema says;
  `--assume-non-empty` writes it as never empty.
- **Every mapping that loses something is reported**, by code and JSON pointer, on stderr or with the
  shape under `--output json`: an undiscriminated union, a closed object, a dropped bound or format, a
  body that is not JSON, an unstated `minItems`, and the rest `janus derive --help` lists.

The command does not speak the engine protocol: deriving is not an engine operation. Swagger 2.0 is
refused. The sample provider has an OpenAPI document, `samples/order-service/openapi.yaml`, whose
derived shape finds everything its recorded shape finds against its consumer's pact.
